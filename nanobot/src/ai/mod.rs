use super::common::*;
use super::model::*;

pub mod builder;
pub mod config;
pub mod grid_fission;
pub mod naive_reassemble;
pub mod utils;
pub mod void;

pub trait AssembleAI {
    fn assemble(&self, model: &Model) -> Vec<Command>;
}

pub trait DisassembleAI {
    fn disassemble(&self, model: &Model) -> Vec<Command>;
}

pub trait ReassembleAI {
    fn reassemble(&self, source: &Model, target: &Model) -> Vec<Command>;
}
