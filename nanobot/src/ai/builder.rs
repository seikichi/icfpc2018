use ai::config::Config;
use ai::grid_fission::GridFissionAI;
use ai::AssembleAI;

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

// fn build_disassembler(name: &String, config: &Config) -> Box<dyn DisassembleAI> {}

// fn build_reassembler(name: &String, config: &Config) -> Box<dyn ReassembleAI> {}
