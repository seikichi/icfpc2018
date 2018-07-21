#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Debug)]
enum Harmonics {
    Low,
    High,
}

#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Debug)]
enum Voxel {
    Full,
    Void,
}

#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Debug)]
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

#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Debug)]
struct NCD {
    x: i32,
    y: i32,
    z: i32,
}

#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Debug)]
struct LCD {
    x: i32,
    y: i32,
    z: i32,
}

#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Debug)]
struct Bid(usize);

#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Debug)]
struct Nanobot {
    bid: usize,
    pos: (i32, i32, i32),
    seeds: Vec<Bid>,
}

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

#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Debug)]
struct Model {
    matrix: Vec<Vec<Vec<Voxel>>>,
}

impl Model {
    fn new() -> Self {
        Model { matrix: vec![] }
    }
}

fn main() {
    println!("Hello, world!");
}
