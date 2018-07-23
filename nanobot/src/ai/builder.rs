use ai::config::Config;
use ai::grid_fission::GridFissionAI;
use ai::naive_reassemble::NaiveReassembleAI;
use ai::void::VoidAI;
use ai::AssembleAI;
use ai::DisassembleAI;
use ai::ReassembleAI;
use model::Model;

use std::process;

pub fn build_assembler(name: &String, config: &Config, _target: &Model) -> Box<AssembleAI> {
    match name.as_str() {
        "default" => Box::new(GridFissionAI::new(config)),
        _ => {
            eprintln!("failed to build assembler AI (name = {})", name);
            process::exit(1);
        }
    }
}

pub fn build_disassembler(name: &String, config: &Config, _source: &Model) -> Box<DisassembleAI> {
    match name.as_str() {
        "default" => Box::new(VoidAI::new(config)),
        _ => {
            eprintln!("failed to build assembler AI (name = {})", name);
            process::exit(1);
        }
    }
}

pub fn build_reassembler(
    name: &String,
    config: &Config,
    source: &Model,
    target: &Model,
) -> Box<ReassembleAI> {
    match name.as_str() {
        "default" => Box::new(NaiveReassembleAI::new(config, source, target)),
        _ => {
            eprintln!("failed to build assembler AI (name = {})", name);
            process::exit(1);
        }
    }
}
