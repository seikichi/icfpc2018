use ai::builder::*;
use ai::config::Config;
use ai::AssembleAI;
use ai::DisassembleAI;
use ai::ReassembleAI;
use common::Command;
use model::Model;

pub struct NaiveReassembleAI {
    assembler: Box<AssembleAI>,
    disassembler: Box<DisassembleAI>,
}

impl NaiveReassembleAI {
    pub fn new(config: &Config, source: &Model, target: &Model) -> Self {
        let assembler = build_assembler(&config.assembler, config, source);
        let disassembler = build_disassembler(&config.disassembler, config, target);
        NaiveReassembleAI {
            assembler,
            disassembler,
        }
    }
}

impl ReassembleAI for NaiveReassembleAI {
    fn reassemble(&mut self, source: &Model, target: &Model) -> Vec<Command> {
        let mut commands = self.disassembler.disassemble(source);
        commands.pop(); // pop Halt
        commands.append(&mut self.assembler.assemble(target));
        commands
    }
}
