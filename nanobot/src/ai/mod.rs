use super::common::*;
use super::model::*;

pub mod builder;
pub mod config;
pub mod grid_fission;
pub mod simple;
pub mod utils;

pub trait AssembleAI {
    fn assemble(&self, model: &Model) -> Vec<Command>;
}

pub trait DisassembleAI {
    fn disassemble(&self, model: &Model) -> Vec<Command>;
}

pub trait ReassembleAI {
    fn reassemble(&self, source: &Model, target: &Model) -> Vec<Command>;
}

pub struct NaiveReassembleAI {
    assembler: Box<AssembleAI>,
    disassembler: Box<DisassembleAI>,
}

impl ReassembleAI for NaiveReassembleAI {
    fn reassemble(&self, source: &Model, target: &Model) -> Vec<Command> {
        let mut commands = self.disassembler.disassemble(source);
        commands.pop(); // pop Halt
        commands.append(&mut self.assembler.assemble(target));
        commands
    }
}
