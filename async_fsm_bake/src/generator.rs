use askama::Template;
use std::collections::HashMap;
use std::collections::HashSet;
use std::io::Write;
use std::path::PathBuf;

#[derive(Template)]
#[template(path = "fsm.txt")]
struct FsmTemplate {
    events: HashSet<String>,
    states: HashSet<String>,
    transitions: HashMap<String, Vec<(String, String)>>,
}

pub fn get_main(
    events: &HashSet<String>,
    states: &HashSet<String>,
    transitions: &HashMap<String, Vec<(String, String)>>,
) -> String {
    let fsm_template = FsmTemplate {
        events: events.clone(),
        states: states.clone(),
        transitions: transitions.clone(),
    };
    fsm_template.render().unwrap()
}

pub fn create_output(out: &PathBuf, main_content: &String) {
    // ignore result as some parts of the output path can be already created
    let out_src_path = out.join("src");
    let _ = std::fs::create_dir_all(&out_src_path);
    let mut file_main_rs = std::fs::File::create(out_src_path.join("main.rs")).unwrap();
    if let Err(err) = file_main_rs.write(&main_content.as_bytes()) {
        println!("Unable to write content to main.rs, error: {err:?}");
    }

    let cargo_toml = include_bytes!("../templates/Cargo.toml.txt");
    let mut cargo_file = std::fs::File::create(out.join("Cargo.toml")).unwrap();
    if let Err(err) = cargo_file.write(cargo_toml) {
        println!("Unable to write content to Cargo.toml, error: {err:?}");
    }
}
