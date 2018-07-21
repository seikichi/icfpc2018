mod ai;
mod common;
mod model;
mod state;

use std::env;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::process;

// use ai::simple::SimpleAI;
use ai::grid_fission::GridFissionAI;
use ai::AI;
use common::write_trace_file;
use model::Model;

#[test]
fn test_truth() {
    assert!(true);
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("not enough arguments");
        process::exit(1);
    }

    let f = File::open(args[1].clone()).expect("file not found");
    let mut f = BufReader::new(f);
    let trace_output_path = Path::new(&args[2]);

    let model = Model::new(&mut f).expect("failed to open model");

    let ai = GridFissionAI::new();
    let commands = ai.generate(&model);

    write_trace_file(trace_output_path, &commands).expect("failed to write trace");
}
