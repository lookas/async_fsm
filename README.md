# Async fsm
Async fsm is a rust lib providing engine to create Async Finite State Machine.

Aync fsm is using tokio mpsc channel to send events into the StateMachine engine and
providing channel to subscribe for the state chages. The more advanced
logic of the transitions need to be implemented in the Transition trait.

## Dependencies
- tokio
- log
- async_trait

## Example implementation

```rust
use async_fsm::*;
use async_trait::async_trait;
use log::info;
use log::LevelFilter;
use std::io::Write;

#[derive(Default, Debug, Eq, PartialEq, Copy, Clone, Hash)]
pub enum State {
    #[default]
    Unknown,
    LowBattery,
    HalflyCharged,
    FullyCharged,
    Charging,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Event {
    BatteryLevel(u32),
    PlugIn,
    PlugOut,
}

#[derive(Debug, Default)]
struct UserData {}

#[derive(Clone)]
struct BatteryLevelState;

#[async_trait]
impl Transition<Event, State, UserData> for BatteryLevelState {
    async fn next(&mut self, event: Event, data: &Data<Event, State, UserData>) -> State {
        match event {
            Event::BatteryLevel(level) if level >= 80 => State::FullyCharged,
            Event::BatteryLevel(level) if level >= 50 => State::HalflyCharged,
            Event::BatteryLevel(_) => State::LowBattery,
            Event::PlugIn => State::Charging,
            _ => data.state,
        }
    }
}

struct ChargingState;
#[async_trait]
impl Transition<Event, State, UserData> for ChargingState {
    async fn next(&mut self, event: Event, data: &Data<Event, State, UserData>) -> State {
        match event {
            Event::PlugOut => State::Unknown, // switch to unknown state and wait for battery level update.
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
    let battery_level_state = Box::new(BatteryLevelState {});
    stm.add_transition(State::Unknown, battery_level_state.clone());
    stm.add_transition(State::LowBattery, battery_level_state.clone());
    stm.add_transition(State::HalflyCharged, battery_level_state.clone());
    stm.add_transition(State::FullyCharged, battery_level_state.clone());
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

    let _ = event_sender.send(Event::BatteryLevel(82)).await;
    let _ = event_sender.send(Event::BatteryLevel(45)).await;
    let _ = event_sender.send(Event::PlugIn).await;
    let _ = event_sender.send(Event::PlugOut).await;
    let _ = event_sender.send(Event::BatteryLevel(55)).await;

    let _ = states.await;
}
```

