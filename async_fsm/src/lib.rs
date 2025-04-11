use async_trait::async_trait;
use log::info;
use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;
use tokio::sync::broadcast;
use tokio::sync::mpsc;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::time::Instant;

/// The data catured on the incomming event.
pub struct Data<Event, State, UserData> {
    /// Previous state - one of the states defined by the user.
    pub prev_state: Option<State>,
    /// Current state - one of the states defined by the user.
    pub state: State,
    /// UserData are defined and maintained by the user.
    /// Those data can be significant to store some useful informations
    /// that are shared across the different states.
    pub user_data: UserData,
    /// Capture the time during handling incomming event.
    #[allow(dead_code)]
    pub events: HashMap<Event, Instant>,
}

/// The trains needs to be implemented for each "State" to ensure state transitions.
#[async_trait]
pub trait Transition<
    Event: Debug + Copy + Clone + PartialEq + Eq + Hash,
    State: Default + Debug + Eq + PartialEq + Copy + Clone + Hash,
    UserData: Debug + Default,
>
{
    /// Process the incomming event and calculate next state.
    /// * `state` - the current state hold by a StateMachine.
    /// * `data` - holds the state machine shared data f.e [prev_state](Data::prev_state) and the UserData defined by the user.
    /// * return the next state.
    async fn next(&mut self, event: Event, data: &Data<Event, State, UserData>) -> State;

    // The method is called just after the switch to the new state.
    fn enter(&mut self, _data: &Data<Event, State, UserData>) {}
}

/// Definition of the callback triggered during incomming event registration.
type FnOnEventRegister<Event, State, UserData> = fn(Event, &mut Data<Event, State, UserData>);

/// StateMachine it is a Finite State Machine that provides an abstract interface and async interactions.
pub struct StateMachine<Event, State, UserData> {
    event_receiver: Receiver<Event>,
    broadcast: (
        tokio::sync::broadcast::Sender<State>,
        tokio::sync::broadcast::Receiver<State>,
    ),
    transitions: HashMap<State, Box<dyn Transition<Event, State, UserData> + Send + Sync>>,
    data: Data<Event, State, UserData>,
    on_event_register: Option<FnOnEventRegister<Event, State, UserData>>,
}

impl<Event, State, UserData> StateMachine<Event, State, UserData>
where
    Event: Debug + Copy + Clone + PartialEq + Eq + Hash,
    State: Default + Debug + Eq + PartialEq + Copy + Clone + Hash,
    UserData: Debug + Default,
{
    /// Creates a StateMachine
    ///
    /// # Examples
    ///
    /// ```
    /// use async_trait::async_trait;
    /// use async_fsm::*;
    ///
    /// #[derive(Default, Debug, Eq, PartialEq, Copy, Clone, Hash)]
    /// enum State {
    ///     #[default]
    ///     Unknown,
    ///     SomeState
    /// }
    ///
    /// #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
    /// enum Event {
    ///     KeyPress(char),
    ///     MouseClick,
    /// }
    ///
    /// #[derive(Debug, Default)]
    /// struct UserData {}
    ///
    /// struct UnknownState {}
    ///
    /// #[async_trait]
    /// impl Transition<Event, State, UserData> for UnknownState {
    ///     async fn next(&mut self, event: Event, data: &Data<Event, State, UserData>) -> State {
    ///         match event {
    ///             Event::MouseClick => State::SomeState, // Transit into the new state.
    ///             _ => data.state, // Remain in the same state.
    ///         }
    ///     }
    /// }
    /// struct SomeState {}
    ///
    /// #[async_trait]
    /// impl Transition<Event, State, UserData> for SomeState {
    ///     async fn next(&mut self, event: Event, _data: &Data<Event, State, UserData>) -> State {
    ///         match event {
    ///             _ => State::Unknown, // Go back to the unknown state.
    ///         }
    ///     }
    /// }
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let (mut stm, event_sender) = StateMachine::<Event, State, UserData>::new(100);
    ///     stm.add_transition(State::Unknown, Box::new(UnknownState {}));
    ///     stm.add_transition(State::SomeState, Box::new(SomeState {}));
    ///
    ///     let mut states = stm.subscribe();
    ///
    ///     // Spawn the task to process the state machine events.
    ///     let task = tokio::spawn(async move {
    ///         stm.process().await;
    ///     });
    ///
    ///     // Send the external events into StateMachine
    ///     let _ = event_sender.send(Event::MouseClick).await;
    ///     assert_eq!(states.recv().await.unwrap(), State::SomeState);
    ///
    ///     let _ = event_sender.send(Event::KeyPress('q')).await;
    ///     assert_eq!(states.recv().await.unwrap(), State::Unknown);
    ///     task.abort();
    /// }
    /// ```
    ///
    pub fn new(size: usize) -> (Self, Sender<Event>) {
        let (event_sender, event_receiver) = mpsc::channel::<Event>(size);
        let fsm = Self {
            event_receiver,
            broadcast: broadcast::channel::<State>(size),
            transitions: HashMap::new(),
            data: Data {
                prev_state: None,
                state: State::default(),
                user_data: UserData::default(),
                events: HashMap::new(),
            },
            on_event_register: None,
        };
        (fsm, event_sender)
    }

    /// Add the possible transitions between the states.
    /// * `state` - one of the states defined by the user.
    /// * `transition` - The Transition which implementes [Transition](Transition::next) trait.
    pub fn add_transition(
        &mut self,
        state: State,
        transition: Box<dyn Transition<Event, State, UserData> + Send + Sync>,
    ) {
        self.transitions.insert(state, transition);
    }

    /// Subscribe to a state changes.
    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<State> {
        self.broadcast.0.subscribe()
    }

    /// The handler to manipulate or store user specyfic data.
    /// * `callback` - the callback closure called when the event is receviced.
    /// The callback supposed to be used if user want to store or manipulate some specyfic data which could be reused in other states.
    ///
    /// # Examples
    /// ```ignore
    /// #[derive(Debug, Default)]
    /// struct UserData {
    ///    event_counter: u64,
    /// }
    /// stm.add_on_register_callback(|_, data| {
    ///     data.user_data.event_counter = data.user_data.event_counter + 1;
    /// });
    /// ```
    pub fn add_on_register_callback(
        &mut self,
        callback: FnOnEventRegister<Event, State, UserData>,
    ) {
        self.on_event_register = Some(callback);
    }

    ///The event processor. It's responsible listen on receive event channel process the event in the current state
    /// and switch into the new state. The state changes are
    pub async fn process(&mut self) {
        while let Some(event) = self.event_receiver.recv().await {
            self.register_event(event);
            self.process_event(event).await;
            self.broadcast.0.send(self.data.state).unwrap();
        }
    }

    async fn process_event(&mut self, event: Event) {
        if let Some(transition) = self.transitions.get_mut(&self.data.state) {
            self.data.prev_state = Some(self.data.state);
            self.data.state = transition.next(event.clone(), &mut self.data).await;
            if self.data.prev_state.unwrap() != self.data.state {
                self.on_state_change();
            }
        }
        info!(
            "[fsm] Processed event: {event:?}; {:?} => {:?}",
            self.data.prev_state, self.data.state
        );
    }

    fn register_event(&mut self, event: Event) {
        self.data.events.insert(event, Instant::now());
        if let Some(callback) = self.on_event_register {
            (callback)(event, &mut self.data);
        }
    }

    fn on_state_change(&mut self) {
        if let Some(transition) = self.transitions.get_mut(&self.data.state) {
            transition.enter(&mut self.data);
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use tokio::task::JoinHandle;

    #[derive(Default, Debug, Eq, PartialEq, Copy, Clone, Hash)]
    pub enum State {
        #[default]
        Idle,
        State1,
        State2,
    }

    #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
    pub enum Event {
        Event1,
        Event2,
        Event3,
    }

    #[derive(Debug, Default)]
    struct UserData {
        event_counter: u64,
    }

    struct IdleState;

    #[async_trait]
    impl Transition<Event, State, UserData> for IdleState {
        async fn next(&mut self, event: Event, data: &Data<Event, State, UserData>) -> State {
            match event {
                Event::Event1 => State::State1,
                _ => data.state,
            }
        }
    }

    struct State1State;
    #[async_trait]
    impl Transition<Event, State, UserData> for State1State {
        async fn next(&mut self, event: Event, data: &Data<Event, State, UserData>) -> State {
            match event {
                Event::Event2 => State::State2,
                _ => data.state,
            }
        }
    }

    struct State2State;
    #[async_trait]
    impl Transition<Event, State, UserData> for State2State {
        async fn next(&mut self, event: Event, data: &Data<Event, State, UserData>) -> State {
            if data.user_data.event_counter > 5 {
                // to many events
                return State::Idle;
            }
            match event {
                Event::Event3 => State::Idle,
                _ => data.state,
            }
        }
    }

    async fn create_stm() -> (
        JoinHandle<()>,
        tokio::sync::mpsc::Sender<Event>,
        tokio::sync::broadcast::Receiver<State>,
    ) {
        let (mut stm, event_sender) = StateMachine::<Event, State, UserData>::new(100);

        stm.add_transition(State::Idle, Box::new(IdleState {}));
        stm.add_transition(State::State1, Box::new(State1State {}));
        stm.add_transition(State::State2, Box::new(State2State {}));

        stm.add_on_register_callback(|_, data| {
            data.user_data.event_counter = data.user_data.event_counter + 1;
        });

        let sub = stm.subscribe();

        let task = tokio::spawn(async move {
            let _ = stm.process().await;
        });

        (task, event_sender, sub)
    }

    #[tokio::test]
    async fn given_idle_state_when_event1_occur_then_state_change_to_state1() {
        let (task, sender, mut states) = create_stm().await;

        // when
        let _ = sender.send(Event::Event1).await;

        // then
        assert_eq!(states.recv().await.unwrap(), State::State1);

        task.abort();
    }

    #[tokio::test]
    async fn given_idle_state_when_event2_occur_then_state_remain_the_same() {
        let (task, sender, mut states) = create_stm().await;

        // when
        // Event is not handled in Idle state
        let _ = sender.send(Event::Event2).await;

        // then
        assert_eq!(states.recv().await.unwrap(), State::Idle);

        task.abort();
    }

    #[tokio::test]
    async fn given_state1_when_event3_occur_then_state_return_to_idle() {
        let (task, sender, mut states) = create_stm().await;

        // given
        let _ = sender.send(Event::Event1).await;
        assert_eq!(states.recv().await.unwrap(), State::State1);
        let _ = sender.send(Event::Event2).await;
        assert_eq!(states.recv().await.unwrap(), State::State2);

        //when
        let _ = sender.send(Event::Event3).await;

        // then
        assert_eq!(states.recv().await.unwrap(), State::Idle);

        task.abort();
    }

    #[tokio::test]
    async fn given_state2_when_events_counter_exceeded_then_state_return_to_idle() {
        let (task, sender, mut states) = create_stm().await;

        // given
        let _ = sender.send(Event::Event1).await;
        assert_eq!(states.recv().await.unwrap(), State::State1);
        let _ = sender.send(Event::Event2).await;
        assert_eq!(states.recv().await.unwrap(), State::State2);

        //when
        for _ in 0..3 {
            let _ = sender.send(Event::Event1).await;
            assert_eq!(states.recv().await.unwrap(), State::State2);
        }
        let _ = sender.send(Event::Event1).await;

        // then
        // after 5th event stae should get back to Idle.
        assert_eq!(states.recv().await.unwrap(), State::Idle);

        task.abort();
    }
}
