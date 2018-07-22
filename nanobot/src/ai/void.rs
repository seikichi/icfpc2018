use ai::DisassembleAI;
use common::*;
use model::*;

pub struct VoidAI {}

impl VoidAI {
    pub fn new() -> Self { VoidAI {} }
}

impl DisassembleAI for VoidAI {
    fn disassemble(&self, model: &Model) -> Vec<Command> {
        unimplemented!()
    }
}

