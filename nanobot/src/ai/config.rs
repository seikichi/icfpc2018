pub struct Config {
    // for NaiveReassembleAI
    assembler: String,
    disassembler: String,
}

impl Config {
    pub fn new() -> Self {
        Config {
            assembler: String::from(""),
            disassembler: String::from(""),
        }
    }
}
