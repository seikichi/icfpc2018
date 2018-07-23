use super::common::*;
use super::model::*;

pub mod bfs;
pub mod builder;
pub mod config;
pub mod grid_fission;
pub mod naive_reassemble;
pub mod utils;
pub mod gvoid;
pub mod gvoid_2d;
pub mod void;
pub mod void_assemble;


pub trait AssembleAI {
    fn assemble(&mut self, model: &Model) -> Vec<Command>;
}

pub trait DisassembleAI {
    fn disassemble(&mut self, model: &Model) -> Vec<Command>;
}

pub trait ReassembleAI {
    fn reassemble(&mut self, source: &Model, target: &Model) -> Vec<Command>;
}
