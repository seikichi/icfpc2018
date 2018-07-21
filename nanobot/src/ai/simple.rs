use ai::AI;
use model::*;
use common::*;

pub struct SimpleAI {}

impl SimpleAI {
    pub fn new() -> Self {
        SimpleAI {}
    }
}

impl AI for SimpleAI {
    fn generate(&self, model: &Model) -> Vec<Command> {
        vec![]
    }
}

#[test]
fn generate_commands_for_empty_3x3() {
    let mut bytes: &[u8] = &[3, 0b00000000, 0b00000000, 0b00000000, 0b00000000];
    let model = Model::new(&mut bytes).unwrap();

    let ai = SimpleAI::new();
    let commands = ai.generate(&model);
    let expected = vec![Command::Halt];

    assert_eq!(expected, commands);
}

#[test]
fn generate_commands_for_non_empty_3x3() {
    let mut bytes: &[u8] = &[3, 0b00000000, 0b00000000, 0b00000000, 0b00000000];
    let model = Model::new(&mut bytes).unwrap();

    let ai = SimpleAI::new();
    let commands = ai.generate(&model);
    let expected = vec![Command::Fill(NCD::new(1, 1, 1)), Command::Halt];

    assert_eq!(expected, commands);
}
