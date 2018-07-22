use ai::config::Config;
use ai::grid_fission::GridFissionAI;
use ai::naive_reassemble::NaiveReassembleAI;
use ai::void::VoidAI;
use ai::AssembleAI;
use ai::DisassembleAI;
use ai::ReassembleAI;

use std::process;

pub fn build_assembler(name: &String, config: &Config) -> Box<AssembleAI> {
    match name.as_str() {
        "default" => Box::new(GridFissionAI::new(config)),
        _ => {
            eprintln!("failed to build assembler AI (name = {})", name);
            process::exit(1);
        }
    }
}

pub fn build_disassembler(name: &String, _config: &Config) -> Box<DisassembleAI> {
    match name.as_str() {
        "default" => Box::new(VoidAI::new()),
        _ => {
            eprintln!("failed to build assembler AI (name = {})", name);
            process::exit(1);
        }
    }
}

pub fn build_reassembler(name: &String, config: &Config) -> Box<ReassembleAI> {
    match name.as_str() {
        "default" => Box::new(NaiveReassembleAI::new(config)),
        _ => {
            eprintln!("failed to build assembler AI (name = {})", name);
            process::exit(1);
        }
    }
}
