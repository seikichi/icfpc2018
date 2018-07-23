use ai::builder::*;
use ai::config::Config;
use ai::ReassembleAI;
use common::Command;
use model::Model;
use state::State;

pub struct ReassembleBruteForceAI {
    best_assembler_commands: Vec<Command>,
    best_disassembler_commands: Vec<Command>,
}

impl ReassembleBruteForceAI {
    pub fn new(config: &Config, source: &Model, target: &Model) -> Self {
        let assemblers = vec![
            "default",
            "kichi",
            "bfs",
        ];

        let disassemblers = vec![
            "default",
            "gvoid_2d",
        ];

        let mut min_energy = <i64>::max_value();
        let mut best_disassembler = "default";
        let mut best_disassembler_commands = vec![];

        for disassembler_name in disassemblers {
            let mut disassembler = build_disassembler(&disassembler_name.to_string(), &config, source);
            let commands = disassembler.disassemble(source);

            let mut state = State::initial_with_model(source);

            let score = simulate(&mut state, &commands);
            println!("{:?}  : {:?}", disassembler_name, score);

            if score < min_energy {
                min_energy = score;
                best_disassembler = disassembler_name;
                best_disassembler_commands = commands;
            }
        }
        println!("BEST DISASM: {:?}  : {:?}", best_disassembler, min_energy);


        let mut min_energy = <i64>::max_value();
        let mut best_assembler = "default";
        let mut best_assembler_commands = vec![];

        for assembler_name in assemblers {
            let mut assembler = build_assembler(&assembler_name.to_string(), &config, target);
            let commands = assembler.assemble(target);
            ;
            let mut state = State::initial(target.matrix.len());

            let score = simulate(&mut state, &commands);

            println!("{:?}  : {:?}", assembler_name, score);

            if score < min_energy {
                min_energy = score;
                best_assembler = assembler_name;
                best_assembler_commands = commands;
            }
        }

        println!("BEST ASM: {:?}  : {:?}", best_assembler, min_energy);

        ReassembleBruteForceAI {
            best_assembler_commands,
            best_disassembler_commands,
        }
    }
}


fn simulate(state: &mut State, trace: &[Command]) -> i64 {
    let mut offset = 0;
    while offset < trace.len() {
        let bot_cnt = state.get_bot_count();
        match state.update_time_step(&trace[offset..offset + bot_cnt]) {
            Ok(_) => {}
            Err(err) => {
                panic!(err.to_string());
            }
        }
        offset += bot_cnt;
    }
    state.get_energy()
}

impl ReassembleAI for ReassembleBruteForceAI {
    fn reassemble(&mut self, _source: &Model, _target: &Model) -> Vec<Command> {
        let mut commands = vec![];
        commands.extend(self.best_disassembler_commands.clone());
        commands.pop(); // pop Halt
        commands.extend(self.best_assembler_commands.clone());
        commands
    }
}
