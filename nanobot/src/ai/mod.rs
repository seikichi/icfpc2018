use super::common::*;
use super::model::*;

pub mod simple;

pub trait AI {
    fn generate(&self, model: &Model) -> Vec<Command>;
}
