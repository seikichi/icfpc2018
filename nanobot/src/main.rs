mod common;
mod state;
mod model;
mod ai;

use std::env;
use std::fs::File;
use std::io::BufReader;
use std::process;

use model::Model;
use ai::AI;
use ai::simple::SimpleAI;

#[test]
fn test_truth() {
    assert!(true);
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("not enough arguments");
        process::exit(1);
    }

    let filename = args[1].clone();
    let f = File::open(filename).expect("file not found");
    let mut f = BufReader::new(f);

    let model = Model::new(&mut f).expect("failed to open model");

    let ai = SimpleAI::new();
    let commands = ai.generate(&model);
}
