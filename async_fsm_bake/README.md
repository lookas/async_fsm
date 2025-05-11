# async_fsm_bake

Bake state machine (async_fsm) source code snippet out of plantuml diagram.

```sh
async_fsm_bake -i <input_file>.puml -o <output_dir_name>
```

## params

- -i --input <FILE> Sets a input file with plantuml state machine diagram.
- -o --output <OUT_DIR> Sets a out directory for generated fsm.

## example diagram
```
@startuml

[*] --> Idle
Idle --> [*] : EvOnExit
Idle : Some notes

Idle-> Operation : EvOnTouch
Operation --> Idle : EvOnKeyDown

@enduml
```
