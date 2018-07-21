enum Harmonics {
    Low,
    High,
}

enum Voxel {
    Full,
    Void,
}

enum Command {
    // singleton
    Halt,
    Wait,
    Flip,
    SMove(LCD),
    LMove(LCD, LCD),
    Fission(NCD, usize),
    Fill(NCD),
    // group
    FusionP(NCD),
    FusionS(NCD),
}

struct NCD {
    x: i32,
    y: i32,
    z: i32,
}

struct LCD {
    x: i32,
    y: i32,
    z: i32,
}

struct Bid(usize);

struct Nanobot {
    bid: usize,
    pos: (i32, i32, i32),
    seeds: Vec<Bid>,
}

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

struct Model {
    matrix: Vec<Vec<Vec<Voxel>>>
}

impl Model {
    fn new() -> Self {
        Model { matrix: vec![] }
    }
}


fn main() {
    println!("Hello, world!");
}
