use std::cmp::*;
use std::error::*;
use std::fmt;
use std::fs;
use std::io::Write;
use std::ops::Add;
use std::path::Path;

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
    SMove(LLCD),
    LMove(SLCD, SLCD),
    Fission(NCD, usize),
    Fill(NCD),
    // group
    FusionP(NCD),
    FusionS(NCD),
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
            Command::FusionP(ncd) => {
                let ncd_enc = ncd.encode();
                vec![(ncd_enc << 3) | 0b111]
            }
            Command::FusionS(ncd) => {
                let ncd_enc = ncd.encode();
                vec![(ncd_enc << 3) | 0b110]
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

#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Debug)]
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

#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Debug)]
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

#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Debug)]
pub struct LLCD {
    x: i32,
    y: i32,
    z: i32,
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
pub struct Region(Position, Position);

pub fn region(p1: Position, p2: Position) -> impl Iterator<Item = Position> {
    (min(p1.z, p2.z)..max(p1.z, p2.z) + 1).flat_map(move |z| {
        (min(p1.y, p2.y)..max(p1.y, p2.y) + 1).flat_map(move |y| {
            (min(p1.x, p2.x)..max(p1.x, p2.x) + 1).map(move |x| Position::new(x, y, z))
        })
    })
}

#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Debug)]
pub struct Bid(pub usize);

#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Debug)]
pub struct Nanobot {
    pub bid: Bid,
    pub pos: Position,
    pub seeds: Vec<Bid>,
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
