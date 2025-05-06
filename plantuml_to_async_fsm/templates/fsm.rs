use async_fsm::*;
use async_trait::async_trait;
use log::info;
use log::LevelFilter;
use std::io::Write;

#[derive(Default, Debug, Eq, PartialEq, Copy, Clone, Hash)]
pub enum State {
    #[default]
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Event {
{% for event in events %}
   {{event}},
{% endfor %}
}

#[derive(Debug, Default)]
struct UserData {}

#[derive(Clone)]
struct BatteryLevelState;

#[async_trait]
impl Transition<Event, State, UserData> for BatteryLevelState {
    async fn next(&mut self, event: Event, data: &Data<Event, State, UserData>) -> State {
        match event {
            _ => data.state,
        }
    }
}

#[tokio::main]
async fn main() {
    env_logger::Builder::new()
        .format(|buf, record| writeln!(buf, "[{}] {}", record.level(), record.args()))
        .filter(None, LevelFilter::Debug)
        .init();

    let (mut stm, event_sender) = StateMachine::<Event, State, UserData>::new(100);
    stm.add_transition(State::Charging, Box::new(ChargingState {}));

    let mut state_subscription = stm.subscribe();

    let _ = tokio::spawn(async move {
        stm.process().await;
    });

    let states = tokio::spawn(async move {
        while let Ok(state) = state_subscription.recv().await {
            info!("Changed State: {state:?}");
        }
    });

    // Send example events
    // let _ = event_sender.send(Event::BatteryLevel(82)).await;
    let _ = states.await;
}
