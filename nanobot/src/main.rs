mod ai;
mod common;
mod model;
mod state;
mod union_find;

use std::env;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::process;

use ai::builder::build_assembler;
use ai::config::Config;
use common::write_trace_file;
use model::Model;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        usage(&args);
        process::exit(1);
    }

    let task = &args[1];
    match &task[..] {
        "assemble" => assemble(&args),
        _ => {
            usage(&args);
            process::exit(1);
        }
    }
}

fn assemble(args: &Vec<String>) {
    let f = File::open(args[2].clone()).expect("file not found");
    let mut f = BufReader::new(f);
    let model = Model::new(&mut f).expect("failed to open model");

    let trace_output_path = Path::new(&args[3]);
    let config = Config::new();
    let name = env::var("GOLD_AI").expect("failed to get AI from ENV");
    let ai = build_assembler(&name, &config);
    let commands = ai.assemble(&model);
    write_trace_file(trace_output_path, &commands).expect("failed to write trace");
}

fn usage(args: &Vec<String>) {
    eprintln!(
        "not enough arguments

Example:
  $ {0} assemble    target.mdl output_trace.nbt
  $ {0} disassemble source.mdl output_trace.nbt
  $ {0} reassemble  source.mdl target.mdl output_trace.nbt

NOTE: Use following environment variables to configure AIs

## Required

GOLD_AI=default

## Optional

GOLD_ASSEMBLER
GOLD_DISASSEMBLER
...
",
        args[0]
    );
}
