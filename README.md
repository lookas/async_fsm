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

[Example](./example/src/main.rs)

