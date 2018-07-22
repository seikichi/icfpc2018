use std::cmp::*;
use std::error::*;
use std::fmt;
use std::fs;
use std::io::Write;
use std::ops::Add;
use std::path::Path;

#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Copy, Debug)]
pub enum Harmonics {
    Low,
    High,
}

#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Copy, Debug)]
pub enum Voxel {
    Full,
    Void,
}

#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Copy, Debug)]
pub enum Command {
    // singleton
    Halt,
    Wait,
    Flip,
    SMove(LLCD),
    LMove(SLCD, SLCD),
    Fission(NCD, usize),
    Fill(NCD),
    Void(NCD),
    // group
    FusionP(NCD),
    FusionS(NCD),
    GFill(NCD, FCD),
    GVoid(NCD, FCD),
}

impl Command {
    pub fn encode(&self) -> Vec<u8> {
        match self {
            Command::Halt => vec![0b11111111],
            Command::Wait => vec![0b11111110],
            Command::Flip => vec![0b11111101],
            Command::SMove(lcd) => {
                let lcd_enc = lcd.encode();
                vec![lcd_enc.0 << 4 | 0b0100, lcd_enc.1]
            }
            Command::LMove(lcd1, lcd2) => {
                let lcd1_enc = lcd1.encode();
                let lcd2_enc = lcd2.encode();
                vec![
                    (lcd2_enc.0 << 6) | (lcd1_enc.0 << 4) | 0b1100,
                    (lcd2_enc.1 << 4) | lcd1_enc.1,
                ]
            }
            Command::Fission(ncd, m) => {
                let ncd_enc = ncd.encode();
                vec![(ncd_enc << 3) | 0b101, *m as u8]
            }
            Command::Fill(ncd) => {
                let ncd_enc = ncd.encode();
                vec![(ncd_enc << 3) | 0b011]
            }
            Command::Void(ncd) => {
                let ncd_enc = ncd.encode();
                vec![(ncd_enc << 3) | 0b010]
            }
            Command::FusionP(ncd) => {
                let ncd_enc = ncd.encode();
                vec![(ncd_enc << 3) | 0b111]
            }
            Command::FusionS(ncd) => {
                let ncd_enc = ncd.encode();
                vec![(ncd_enc << 3) | 0b110]
            }
            Command::GFill(ncd, fcd) => {
                let ncd_enc = ncd.encode();
                let fcd_enc = fcd.encode();
                vec![(ncd_enc << 3) | 0b001, fcd_enc.0, fcd_enc.1, fcd_enc.2]
            }
            Command::GVoid(ncd, fcd) => {
                let ncd_enc = ncd.encode();
                let fcd_enc = fcd.encode();
                vec![(ncd_enc << 3) | 0b000, fcd_enc.0, fcd_enc.1, fcd_enc.2]
            }
        }
    }
}

#[test]
fn command_encode_test() {
    let flip = Command::Flip.encode();
    assert_eq!(flip.len(), 1);
    assert_eq!(flip[0], 0b11111101);
    let smove = Command::SMove(LLCD::new(12, 0, 0)).encode();
    assert_eq!(smove.len(), 2);
    assert_eq!(smove[0], 0b00010100);
    assert_eq!(smove[1], 0b00011011);
    let smove = Command::SMove(LLCD::new(0, 0, -4)).encode();
    assert_eq!(smove.len(), 2);
    assert_eq!(smove[0], 0b00110100);
    assert_eq!(smove[1], 0b00001011);
    let lmove = Command::LMove(SLCD::new(3, 0, 0), SLCD::new(0, -5, 0)).encode();
    assert_eq!(lmove.len(), 2);
    assert_eq!(lmove[0], 0b10011100);
    assert_eq!(lmove[1], 0b00001000);
    let fusionp = Command::FusionP(NCD::new(-1, 1, 0)).encode();
    assert_eq!(fusionp.len(), 1);
    assert_eq!(fusionp[0], 0b00111111);
    let fusions = Command::FusionS(NCD::new(1, -1, 0)).encode();
    assert_eq!(fusions.len(), 1);
    assert_eq!(fusions[0], 0b10011110);
    let fission = Command::Fission(NCD::new(0, 0, 1), 5).encode();
    assert_eq!(fission.len(), 2);
    assert_eq!(fission[0], 0b01110101);
    assert_eq!(fission[1], 0b00000101);
    let fill = Command::Fill(NCD::new(0, -1, 0)).encode();
    assert_eq!(fill.len(), 1);
    assert_eq!(fill[0], 0b01010011);
    let void = Command::Void(NCD::new(1, 0, 1)).encode();
    assert_eq!(void.len(), 1);
    assert_eq!(void[0], 0b10111010);
    let gfill = Command::GFill(NCD::new(0, -1, 0), FCD::new(10, -15, 20)).encode();
    assert_eq!(gfill.len(), 4);
    assert_eq!(gfill[0], 0b01010001);
    assert_eq!(gfill[1], 0b00101000);
    assert_eq!(gfill[2], 0b00001111);
    assert_eq!(gfill[3], 0b00110010);
    let gvoid = Command::GVoid(NCD::new(1, 0, 0), FCD::new(5, 5, -5)).encode();
    assert_eq!(gvoid.len(), 4);
    assert_eq!(gvoid[0], 0b10110000);
    assert_eq!(gvoid[1], 0b00100011);
    assert_eq!(gvoid[2], 0b00100011);
    assert_eq!(gvoid[3], 0b00011001);
}

pub trait CD {
    fn x(&self) -> i32;
    fn y(&self) -> i32;
    fn z(&self) -> i32;
    fn manhattan_length(&self) -> i32 {
        self.x().abs() + self.y().abs() + self.z().abs()
    }
    fn chessboard_length(&self) -> i32 {
        vec![self.x().abs(), self.y().abs(), self.z().abs()]
            .into_iter()
            .max()
            .unwrap()
    }
}

impl fmt::Display for CD {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({}, {}, {})", self.x(), self.y(), self.z())
    }
}

#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Copy, Debug)]
pub struct NCD {
    x: i32,
    y: i32,
    z: i32,
}

impl NCD {
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        let ncd = NCD { x, y, z };
        assert!(ncd.manhattan_length() <= 2 && ncd.chessboard_length() == 1);
        ncd
    }
    pub fn encode(&self) -> u8 {
        ((self.x + 1) * 9 + (self.y + 1) * 3 + (self.z + 1)) as u8
    }
}

impl CD for NCD {
    fn x(&self) -> i32 {
        self.x
    }
    fn y(&self) -> i32 {
        self.y
    }
    fn z(&self) -> i32 {
        self.z
    }
}

#[test]
fn ncd_encode_test() {
    let ncd = NCD::new(1, 0, 0);
    assert_eq!(ncd.encode(), 18 + 3 + 1);
}

#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Copy, Debug)]
pub struct FCD {
    x: i32,
    y: i32,
    z: i32,
}

impl FCD {
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        let fcd = FCD { x, y, z };
        assert!(0 < fcd.chessboard_length() && fcd.chessboard_length() <= 30);
        fcd
    }
    pub fn encode(&self) -> (u8, u8, u8) {
        (
            (self.x + 30) as u8,
            (self.y + 30) as u8,
            (self.z + 30) as u8,
        )
    }
}

impl CD for FCD {
    fn x(&self) -> i32 {
        self.x
    }
    fn y(&self) -> i32 {
        self.y
    }
    fn z(&self) -> i32 {
        self.z
    }
}

#[test]
fn fcd_encode_test() {
    let fcd = FCD::new(20, 10, -5);
    let enc = fcd.encode();
    assert_eq!(enc.0, 50);
    assert_eq!(enc.1, 40);
    assert_eq!(enc.2, 25);
}

#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Copy, Debug)]
pub struct SLCD {
    x: i32,
    y: i32,
    z: i32,
}

impl SLCD {
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        let slcd = SLCD { x, y, z };
        assert!(
            slcd.manhattan_length() <= 5
                && slcd.chessboard_length() > 0
                && slcd.manhattan_length() == slcd.chessboard_length()
        );
        slcd
    }
    pub fn encode(&self) -> (u8, u8) {
        let ret = if self.x != 0 {
            (0b01, self.x + 5)
        } else if self.y != 0 {
            (0b10, self.y + 5)
        } else {
            (0b11, self.z + 5)
        };
        (ret.0, ret.1 as u8)
    }
}

impl CD for SLCD {
    fn x(&self) -> i32 {
        self.x
    }
    fn y(&self) -> i32 {
        self.y
    }
    fn z(&self) -> i32 {
        self.z
    }
}

#[test]
fn slcd_encode_test() {
    let slcd = SLCD::new(-3, 0, 0);
    let enc = slcd.encode();
    assert_eq!(enc.0, 1);
    assert_eq!(enc.1, 2);
}

#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Copy, Debug)]
pub struct LLCD {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl LLCD {
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        let llcd = LLCD { x, y, z };
        assert!(
            llcd.manhattan_length() <= 15
                && llcd.chessboard_length() > 0
                && llcd.manhattan_length() == llcd.chessboard_length()
        );
        llcd
    }
    pub fn encode(&self) -> (u8, u8) {
        let ret = if self.x != 0 {
            (0b01, self.x + 15)
        } else if self.y != 0 {
            (0b10, self.y + 15)
        } else {
            (0b11, self.z + 15)
        };
        (ret.0, ret.1 as u8)
    }
}

impl CD for LLCD {
    fn x(&self) -> i32 {
        self.x
    }
    fn y(&self) -> i32 {
        self.y
    }
    fn z(&self) -> i32 {
        self.z
    }
}

#[test]
fn llcd_encode_test() {
    let llcd = LLCD::new(0, 10, 0);
    let enc = llcd.encode();
    assert_eq!(enc.0, 2);
    assert_eq!(enc.1, 25);
}

#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Copy, Debug, Hash)]
pub struct Position {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl Position {
    pub fn new(x: i32, y: i32, z: i32) -> Position {
        Position { x, y, z }
    }

    pub fn zero() -> Position {
        Position { x: 0, y: 0, z: 0 }
    }

    pub fn index(&self, r: usize) -> usize {
        (self.x as usize) + (self.y as usize) * r + (self.z as usize) * r * r
    }
}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({}, {}, {})", self.x, self.y, self.z)
    }
}

impl<'a> Add<&'a CD> for Position {
    type Output = Position;

    fn add(self, other: &'a CD) -> Position {
        Position {
            x: self.x + other.x(),
            y: self.y + other.y(),
            z: self.z + other.z(),
        }
    }
}

#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Copy, Debug, Hash)]
pub struct Region(pub Position, pub Position);

impl Region {
    pub fn dimension(&self) -> i32 {
        (if self.0.x == self.1.x { 0 } else { 1 }) +
            (if self.0.y == self.1.y { 0 } else { 1 }) +
            (if self.0.z == self.1.z { 0 } else { 1 })
    }

    pub fn canonical(&self) -> Region {
        let p1 = Position::new(
            min(self.0.x, self.1.x),
            min(self.0.y, self.1.y),
            min(self.0.z, self.1.z));
        let p2 = Position::new(
            max(self.0.x, self.1.x),
            max(self.0.y, self.1.y),
            max(self.0.z, self.1.z));
        Region(p1, p2)
    }

    pub fn iter(&self) -> impl Iterator<Item=Position> {
        let c = self.canonical();
        (c.0.z..(c.1.z + 1)).flat_map(move |z| {
            (c.0.y..(c.1.y + 1)).flat_map(move |y| {
                (c.0.x..(c.1.x + 1)).map(move |x| Position::new(x, y, z))
            })
        })
    }
}

impl CD for Position {
    fn x(&self) -> i32 {
        self.x
    }
    fn y(&self) -> i32 {
        self.y
    }
    fn z(&self) -> i32 {
        self.z
    }
}

pub fn adjacent(p: Position) -> Vec<Position> {
    vec![
        p + &Position::new(-1, 0, 0),
        p + &Position::new(1, 0, 0),
        p + &Position::new(0, -1, 0),
        p + &Position::new(0, 1, 0),
        p + &Position::new(0, 0, -1),
        p + &Position::new(0, 0, 1),
    ]
}

#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Copy, Debug, Hash)]
pub struct Bid(pub usize);

#[derive(Eq, Clone, Debug)]
pub struct Nanobot {
    pub bid: Bid,
    pub pos: Position,
    pub seeds: Vec<Bid>,
}

impl Ord for Nanobot {
    fn cmp(&self, other: &Nanobot) -> Ordering {
        self.bid.cmp(&other.bid)
    }
}

impl PartialOrd for Nanobot {
    fn partial_cmp(&self, other: &Nanobot) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Nanobot {
    fn eq(&self, other: &Nanobot) -> bool {
        self.bid == other.bid
    }
}

pub fn encode_trace(trace: &[Command]) -> Vec<u8> {
    let mut ret = Vec::with_capacity(trace.len() * 1);
    for t in trace.iter() {
        ret.append(&mut t.encode());
    }
    ret
}

#[test]
fn encode_trace_test() {
    let fusions = Command::FusionS(NCD::new(1, -1, 0));
    let fission = Command::Fission(NCD::new(0, 0, 1), 5);
    let trace = vec![fusions, fission];
    let result = encode_trace(&trace[..]);
    assert_eq!(result.len(), 3);
    assert_eq!(result[0], 0b10011110);
    assert_eq!(result[1], 0b01110101);
    assert_eq!(result[2], 0b00000101);
}

pub fn write_trace_file(path: &Path, trace: &[Command]) -> Result<(), Box<Error>> {
    let mut buffer = fs::File::create(path)?;
    buffer.write_all(&encode_trace(trace)[..])?;
    Ok(())
}
