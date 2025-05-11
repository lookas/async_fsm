use clap::{arg, command, value_parser};
use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Cursor;
use std::path::PathBuf;

mod generator;
mod parser;

fn main() {
    let matches = command!()
        .arg(arg!([name] "Optional name to operate on"))
        .arg(
            arg!(
                -i --input <FILE> "Sets a input file with plantuml state machine diagram."
            )
            .required(true)
            .value_parser(value_parser!(PathBuf)),
        )
        .arg(
            arg!(
                -o --output <DIR> "Sets a out directory for generated fsm."
            )
            .required(true)
            .value_parser(value_parser!(PathBuf)),
        )
        .get_matches();

    // input and output are required params
    let input_path = matches.get_one::<PathBuf>("input").unwrap();
    let output_path = matches.get_one::<PathBuf>("output").unwrap();

    let input_file = match File::open(input_path) {
        Err(err) => {
            println!("Unable to open input file: {input_path:?}! Error: {err:?}");
            return;
        }
        Ok(f) => f,
    };
    let reader = BufReader::new(input_file).lines();

    println!("Generating async_fsm from: {:?}", input_path);

    let mut parser = parser::Uml::default();
    parser.parse(reader);

    let fsm_main = generator::get_main(&parser.events, &parser.states, &parser.transitions);
    generator::create_output(&output_path, &fsm_main);
    println!("Output generated at: {output_path:?}");
}
