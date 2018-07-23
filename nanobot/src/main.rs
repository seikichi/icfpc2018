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

use ai::builder::*;
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
        "disassemble" => disassemble(&args),
        "reassemble" => reassemble(&args),
        _ => {
            usage(&args);
            process::exit(1);
        }
    }
}

fn assemble(args: &Vec<String>) {
    let f = File::open(args[2].clone()).expect("target file not found");
    let mut f = BufReader::new(f);
    let target = Model::new(&mut f).expect("failed to open target model");

    let trace_output_path = Path::new(&args[3]);
    let config = Config::new();
    let name = env::var("GOLD_AI").expect("failed to get AI from ENV");
    let mut ai = build_assembler(&name, &config, &target);
    let commands = ai.assemble(&target);
    write_trace_file(trace_output_path, &commands).expect("failed to write trace");
}

fn disassemble(args: &Vec<String>) {
    let f = File::open(args[2].clone()).expect("source file not found");
    let mut f = BufReader::new(f);
    let source = Model::new(&mut f).expect("failed to open source model");

    let trace_output_path = Path::new(&args[3]);
    let config = Config::new();
    let name = env::var("GOLD_AI").expect("failed to get AI from ENV");
    let mut ai = build_disassembler(&name, &config, &source);
    let commands = ai.disassemble(&source);
    write_trace_file(trace_output_path, &commands).expect("failed to write trace");
}

fn reassemble(args: &Vec<String>) {
    if args.len() < 4 {
        usage(&args);
        process::exit(1);
    }

    let f = File::open(args[2].clone()).expect("source file not found");
    let mut f = BufReader::new(f);
    let source = Model::new(&mut f).expect("failed to open source model");

    let f = File::open(args[3].clone()).expect("target file not found");
    let mut f = BufReader::new(f);
    let target = Model::new(&mut f).expect("failed to open target model");

    let trace_output_path = Path::new(&args[4]);
    let config = Config::new();
    let name = env::var("GOLD_AI").expect("failed to get AI from ENV");
    let mut ai = build_reassembler(&name, &config, &source, &target);
    let commands = ai.reassemble(&source, &target);
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

### GOLD_AI=default

GOLD_ASSEMBLER=default
GOLD_DISASSEMBLER=default
GOLD_DRY_RUN_MAX_RESOLUTION=30 (default)
...
",
        args[0]
    );
}
