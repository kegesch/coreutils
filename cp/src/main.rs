use clap::{App, load_yaml};

fn main() {
    let yaml = load_yaml!("dirname.yml");
    let matches = App::from_yaml(yaml).get_matches();

}