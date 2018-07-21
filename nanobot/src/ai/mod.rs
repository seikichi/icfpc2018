use super::common::*;
use super::model::*;

pub mod simple;
pub mod utils;

pub trait AI {
    fn generate(&self, model: &Model) -> Vec<Command>;
}
