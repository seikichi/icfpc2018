use common::*;

#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Debug)]
struct State {
    energy: i64,
    harmonics: Harmonics,
    matrix: Vec<Vec<Vec<Voxel>>>,
    bots: Vec<Nanobot>,
    trace: Vec<Command>,
}

impl State {
    fn encode(&self) -> Vec<u8> {
        vec![]
    }
}
