use super::common::*;
use super::model::*;

pub mod grid_fission;
pub mod simple;
pub mod utils;

pub trait AI {
    fn generate(&self, model: &Model) -> Vec<Command>;
}

pub trait AssembleAI {
    fn assemble(&self, model: &Model) -> Vec<Command>;
}

pub trait DisassembleAI {
    fn disassemble(&self, model: &Model) -> Vec<Command>;
}

pub trait ReassembleAI {
    fn reassemble(&self, source: &Model, target: &Model) -> Vec<Command>;
}

pub struct NaiveReassembleAI<D: DisassembleAI, A: AssembleAI> {
    assembler: A,
    disassembler: D,
}

impl<D: DisassembleAI, A: AssembleAI> NaiveReassembleAI<D, A> {
    pub fn new(disassembler: D, assembler: A) -> Self {
        NaiveReassembleAI {
            disassembler,
            assembler,
        }
    }
}

impl<D: DisassembleAI, A: AssembleAI> ReassembleAI for NaiveReassembleAI<D, A> {
    fn reassemble(&self, source: &Model, target: &Model) -> Vec<Command> {
        let mut commands = self.disassembler.disassemble(source);
        commands.pop(); // pop Halt
        commands.append(&mut self.assembler.assemble(target));
        commands
    }
}
