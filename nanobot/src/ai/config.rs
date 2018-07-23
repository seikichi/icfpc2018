use std::env;

pub struct Config {
    // for NaiveReassembleAI
    pub assembler: String,
    pub disassembler: String,
    // for VoidAI, GridFissionAI
    pub dry_run_max_resolution: i32,
}

impl Config {
    pub fn new() -> Self {
        let dry_run_max_resolution = env::var("GOLD_DRY_RUN_MAX_RESOLUTION")
            .unwrap_or(String::from("0"))
            .parse::<i32>()
            .unwrap_or(0);

        Config {
            assembler: env::var("GOLD_ASSEMBLER").unwrap_or(String::from("")),
            disassembler: env::var("GOLD_DISASSEMBLER").unwrap_or(String::from("")),
            dry_run_max_resolution,
        }
    }
}
