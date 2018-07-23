use ai::bfs::BfsAI;
use ai::config::Config;
use ai::grid_fission::GridFissionAI;
use ai::naive_reassemble::NaiveReassembleAI;
use ai::reassemble_brute_force::ReassembleBruteForceAI;
use ai::void::VoidAI;
use ai::void_assemble::VoidAssembleAI;
use ai::gvoid::GvoidAI;
use ai::gvoid_2d::Gvoid2dAI;
use ai::AssembleAI;
use ai::DisassembleAI;
use ai::ReassembleAI;
use model::Model;

use std::process;

pub fn build_assembler(name: &String, config: &Config, target: &Model) -> Box<AssembleAI> {
    let r = target.matrix.len();
    let source = Model::initial(r);
    match name.as_str() {
        "default" => Box::new(GridFissionAI::new(config)),
        "kichi" => Box::new(VoidAssembleAI::new(config)),
        "bfs" => Box::new(BfsAI::new(config, &source, &target)),
        _ => {
            eprintln!("failed to build assembler AI (name = {})", name);
            process::exit(1);
        }
    }
}

pub fn build_disassembler(name: &String, config: &Config, _source: &Model) -> Box<DisassembleAI> {
    match name.as_str() {
        "default" => Box::new(VoidAI::new(config)),
        "gvoid" => Box::new(GvoidAI::new(config)),
        "gvoid_2d" => Box::new(Gvoid2dAI::new(config)),
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
        "bruteforce" => Box::new(ReassembleBruteForceAI::new(config, source, target)),
        _ => {
            eprintln!("failed to build assembler AI (name = {})", name);
            process::exit(1);
        }
    }
}
