#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Debug)]
pub enum Harmonics {
    Low,
    High,
}

#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Debug)]
pub enum Voxel {
    Full,
    Void,
}

#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Debug)]
pub enum Command {
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
pub struct NCD {
    x: i32,
    y: i32,
    z: i32,
}

#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Debug)]
pub struct LCD {
    x: i32,
    y: i32,
    z: i32,
}

#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Debug)]
pub struct Bid(usize);

#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Debug)]
pub struct Nanobot {
    bid: usize,
    pos: (i32, i32, i32),
    seeds: Vec<Bid>,
}
