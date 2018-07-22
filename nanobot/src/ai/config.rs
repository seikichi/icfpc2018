use std::env;

pub struct Config {
    // for NaiveReassembleAI
    pub assembler: String,
    pub disassembler: String,
}

impl Config {
    pub fn new() -> Self {
        Config {
            assembler: env::var("GOLD_ASSEMBLER").unwrap_or(String::from("")),
            disassembler: env::var("GOLD_DISASSEMBLER").unwrap_or(String::from("")),
        }
    }
}
