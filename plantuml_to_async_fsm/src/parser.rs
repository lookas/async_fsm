use regex::Regex;
use std::collections::HashMap;
use std::collections::HashSet;
use std::io::BufReader;
use std::io::Lines;
use std::io::Read;

#[derive(Default)]
pub struct Uml {
    pub states: HashSet<String>,
    pub events: HashSet<String>,
    // source state, Vec<(event, dest state);
    pub transitions: HashMap<String, Vec<(String, String)>>,
}

impl Uml {
    pub fn parse<R: Read>(&mut self, mut lines: Lines<BufReader<R>>) {
        while let Some(Ok(line)) = lines.next() {
            self.parse_line(&line);
        }
    }

    fn add_transition(&mut self, from: &String, event: &String, to: &String) {
        if let Some(transition) = self.transitions.get_mut(from) {
            transition.push((event.clone(), to.clone()));
        } else {
            let mut v = Vec::<(String, String)>::new();
            v.push((event.clone(), to.clone()));
            self.transitions.insert(from.clone(), v);
        }
    }
    fn add_state(&mut self, state: &String) {
        if !self.states.contains(state) {
            self.states.insert(state.clone());
        }
    }

    fn parse_line(&mut self, line: &String) {
        let start_point_regex = Regex::new(r"\[\*\]\s*-+>\s*(?<start_point>\S+)").unwrap();
        if let Some(caps) = start_point_regex.captures(line) {
            let start_point = &caps["start_point"];
            self.add_state(&start_point.to_string());
            return;
        }

        let end_point_regex = Regex::new(r"\s*(?<end_point>\S+)\s*-+>\s*\[\*\]").unwrap();
        if let Some(caps) = end_point_regex.captures(line) {
            let end_point = &caps["end_point"];
            self.add_state(&end_point.to_string());
            return;
        }

        let transition_regex =
            Regex::new(r"(?<from>\S+)\s*-+>\s*(?<to>\S+)\s*:\s*(?<event>\S+)").unwrap();
        if let Some(caps) = transition_regex.captures(line) {
            let from = &caps["from"];
            let to = &caps["to"];
            let event = &caps["event"];
            self.add_state(&from.to_string());
            self.add_state(&to.to_string());
            self.events.insert(event.to_string());
            self.add_transition(&from.into(), &event.into(), &to.into());
            return;
        }
    }
}
