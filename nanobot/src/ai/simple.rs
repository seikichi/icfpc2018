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
