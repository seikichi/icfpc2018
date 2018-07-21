use common::*;

#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Debug)]
struct Model {
    matrix: Vec<Vec<Vec<Voxel>>>,
}

impl Model {
    fn new() -> Self {
        Model { matrix: vec![] }
    }
}
