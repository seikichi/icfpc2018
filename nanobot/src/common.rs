#![allow(dead_code)]

use std::cmp::*;
use std::error::*;
use std::fmt;
use std::fs;
use std::io::Read;
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

#[derive(Debug)]
pub struct CommandParseError {
    message: String,
}

impl CommandParseError {
    pub fn new(message: String) -> CommandParseError {
        CommandParseError {
            message: message.to_string(),
        }
    }
}

impl fmt::Display for CommandParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "CommandParseError: {}", self.message)
    }
}

impl Error for CommandParseError {
    fn cause(&self) -> Option<&Error> {
        None
    }
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
    pub fn decode(input: &[u8], offset: &mut usize) -> Result<Command, Box<Error>> {
        if input[*offset] == 0b11111111 {
            *offset += 1;
            return Ok(Command::Halt);
        } else if input[*offset] == 0b11111110 {
            *offset += 1;
            return Ok(Command::Wait);
        } else if input[*offset] == 0b11111101 {
            *offset += 1;
            return Ok(Command::Flip);
        } else if (input[*offset] & 0b00001111) == 0b0100 {
            let v1 = input[*offset] >> 4;
            let v2 = input[*offset + 1];
            let lcd = LLCD::decode(v1, v2);
            *offset += 2;
            return Ok(Command::SMove(lcd));
        } else if (input[*offset] & 0b00001111) == 0b1100 {
            let v11 = (input[*offset] >> 4) & 0b11;
            let v12 = (input[*offset + 1] >> 0) & 0b1111;
            let v21 = (input[*offset] >> 6) & 0b11;
            let v22 = (input[*offset + 1] >> 4) & 0b1111;
            let slcd1 = SLCD::decode(v11, v12);
            let slcd2 = SLCD::decode(v21, v22);
            *offset += 2;
            return Ok(Command::LMove(slcd1, slcd2));
        } else if (input[*offset] & 0b00000111) == 0b101 {
            let v1 = input[*offset] >> 3;
            let ncd = NCD::decode(v1);
            let m = input[*offset + 1] as usize;
            *offset += 2;
            return Ok(Command::Fission(ncd, m));
        } else if (input[*offset] & 0b00000111) == 0b011 {
            let v1 = input[*offset] >> 3;
            let ncd = NCD::decode(v1);
            *offset += 1;
            return Ok(Command::Fill(ncd));
        } else if (input[*offset] & 0b00000111) == 0b010 {
            let v1 = input[*offset] >> 3;
            let ncd = NCD::decode(v1);
            *offset += 1;
            return Ok(Command::Void(ncd));
        } else if (input[*offset] & 0b00000111) == 0b111 {
            let v1 = input[*offset] >> 3;
            let ncd = NCD::decode(v1);
            *offset += 1;
            return Ok(Command::FusionP(ncd));
        } else if (input[*offset] & 0b00000111) == 0b110 {
            let v1 = input[*offset] >> 3;
            let ncd = NCD::decode(v1);
            *offset += 1;
            return Ok(Command::FusionS(ncd));
        } else if (input[*offset] & 0b00000111) == 0b001 {
            let v11 = input[*offset] >> 3;
            let v21 = input[*offset + 1];
            let v22 = input[*offset + 2];
            let v23 = input[*offset + 3];
            let ncd = NCD::decode(v11);
            let fcd = FCD::decode(v21, v22, v23);
            *offset += 4;
            return Ok(Command::GFill(ncd, fcd));
        } else if (input[*offset] & 0b00000111) == 0b000 {
            let v11 = input[*offset] >> 3;
            let v21 = input[*offset + 1];
            let v22 = input[*offset + 2];
            let v23 = input[*offset + 3];
            let ncd = NCD::decode(v11);
            let fcd = FCD::decode(v21, v22, v23);
            *offset += 4;
            return Ok(Command::GVoid(ncd, fcd));
        } else {
            let message = format!(
                "Unknown Command: value={}, offset={}",
                input[*offset], offset
            );
            return Err(Box::new(CommandParseError::new(message)));
        }
    }
}

#[test]
fn command_encdec_test() {
    let mut offset = 0;
    let flip = Command::Flip;
    let flip_enc = flip.encode();
    let flip2 = Command::decode(&flip_enc[..], &mut offset).unwrap();
    assert_eq!(flip_enc.len(), 1);
    assert_eq!(flip_enc[0], 0b11111101);
    assert_eq!(flip2, flip);
    assert_eq!(offset, 1);

    let mut offset = 0;
    let smove = Command::SMove(LLCD::new(12, 0, 0));
    let smove_enc = smove.encode();
    let smove2 = Command::decode(&smove_enc[..], &mut offset).unwrap();
    assert_eq!(smove_enc.len(), 2);
    assert_eq!(smove_enc[0], 0b00010100);
    assert_eq!(smove_enc[1], 0b00011011);
    assert_eq!(smove2, smove);
    assert_eq!(offset, 2);

    let mut offset = 0;
    let smove = Command::SMove(LLCD::new(0, 0, -4));
    let smove_enc = smove.encode();
    let smove2 = Command::decode(&smove_enc[..], &mut offset).unwrap();
    assert_eq!(smove_enc.len(), 2);
    assert_eq!(smove_enc[0], 0b00110100);
    assert_eq!(smove_enc[1], 0b00001011);
    assert_eq!(smove2, smove);
    assert_eq!(offset, 2);

    let mut offset = 0;
    let lmove = Command::LMove(SLCD::new(3, 0, 0), SLCD::new(0, -5, 0));
    let lmove_enc = lmove.encode();
    let lmove2 = Command::decode(&lmove_enc[..], &mut offset).unwrap();
    assert_eq!(lmove_enc.len(), 2);
    assert_eq!(lmove_enc[0], 0b10011100);
    assert_eq!(lmove_enc[1], 0b00001000);
    assert_eq!(lmove2, lmove);
    assert_eq!(offset, 2);

    let mut offset = 0;
    let fusionp = Command::FusionP(NCD::new(-1, 1, 0));
    let fusionp_enc = fusionp.encode();
    let fusionp2 = Command::decode(&fusionp_enc[..], &mut offset).unwrap();
    assert_eq!(fusionp_enc.len(), 1);
    assert_eq!(fusionp_enc[0], 0b00111111);
    assert_eq!(fusionp2, fusionp);
    assert_eq!(offset, 1);

    let mut offset = 0;
    let fusions = Command::FusionS(NCD::new(1, -1, 0));
    let fusions_enc = fusions.encode();
    let fusions2 = Command::decode(&fusions_enc[..], &mut offset).unwrap();
    assert_eq!(fusions_enc.len(), 1);
    assert_eq!(fusions_enc[0], 0b10011110);
    assert_eq!(fusions2, fusions);
    assert_eq!(offset, 1);

    let mut offset = 0;
    let fission = Command::Fission(NCD::new(0, 0, 1), 5);
    let fission_enc = fission.encode();
    let fission2 = Command::decode(&fission_enc[..], &mut offset).unwrap();
    assert_eq!(fission_enc.len(), 2);
    assert_eq!(fission_enc[0], 0b01110101);
    assert_eq!(fission_enc[1], 0b00000101);
    assert_eq!(fission2, fission);
    assert_eq!(offset, 2);

    let mut offset = 0;
    let fill = Command::Fill(NCD::new(0, -1, 0));
    let fill_enc = fill.encode();
    let fill2 = Command::decode(&fill_enc[..], &mut offset).unwrap();
    assert_eq!(fill_enc.len(), 1);
    assert_eq!(fill_enc[0], 0b01010011);
    assert_eq!(fill2, fill);
    assert_eq!(offset, 1);

    let mut offset = 0;
    let void = Command::Void(NCD::new(1, 0, 1));
    let void_enc = void.encode();
    let void2 = Command::decode(&void_enc[..], &mut offset).unwrap();
    assert_eq!(void_enc.len(), 1);
    assert_eq!(void_enc[0], 0b10111010);
    assert_eq!(void2, void);
    assert_eq!(offset, 1);

    let mut offset = 0;
    let gfill = Command::GFill(NCD::new(0, -1, 0), FCD::new(10, -15, 20));
    let gfill_enc = gfill.encode();
    let gfill2 = Command::decode(&gfill_enc[..], &mut offset).unwrap();
    assert_eq!(gfill_enc.len(), 4);
    assert_eq!(gfill_enc[0], 0b01010001);
    assert_eq!(gfill_enc[1], 0b00101000);
    assert_eq!(gfill_enc[2], 0b00001111);
    assert_eq!(gfill_enc[3], 0b00110010);
    assert_eq!(gfill2, gfill);
    assert_eq!(offset, 4);

    let mut offset = 0;
    let gvoid = Command::GVoid(NCD::new(1, 0, 0), FCD::new(5, 5, -5));
    let gvoid_enc = gvoid.encode();
    let gvoid2 = Command::decode(&gvoid_enc[..], &mut offset).unwrap();
    assert_eq!(gvoid_enc.len(), 4);
    assert_eq!(gvoid_enc[0], 0b10110000);
    assert_eq!(gvoid_enc[1], 0b00100011);
    assert_eq!(gvoid_enc[2], 0b00100011);
    assert_eq!(gvoid_enc[3], 0b00011001);
    assert_eq!(gvoid2, gvoid);
    assert_eq!(offset, 4);
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
    pub fn decode(v: u8) -> Self {
        let x = v as i32 / 9 % 3 - 1;
        let y = v as i32 / 3 % 3 - 1;
        let z = v as i32 / 1 % 3 - 1;
        Self::new(x, y, z)
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
fn ncd_encdec_test() {
    let ncd = NCD::new(1, 0, 0);
    assert_eq!(ncd.encode(), 18 + 3 + 1);
    let ncd2 = NCD::decode(ncd.encode());
    assert_eq!(ncd2, ncd);
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
    pub fn decode(v1: u8, v2: u8, v3: u8) -> Self {
        let x = v1 as i32 - 30;
        let y = v2 as i32 - 30;
        let z = v3 as i32 - 30;
        Self::new(x, y, z)
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
fn fcd_encdec_test() {
    let fcd = FCD::new(20, 10, -5);
    let enc = fcd.encode();
    assert_eq!(enc.0, 50);
    assert_eq!(enc.1, 40);
    assert_eq!(enc.2, 25);
    let fcd2 = FCD::decode(enc.0, enc.1, enc.2);
    assert_eq!(fcd2, fcd);
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
    pub fn decode(v1: u8, v2: u8) -> Self {
        let mut x = 0;
        let mut y = 0;
        let mut z = 0;
        if v1 == 0b01 {
            x = v2 as i32 - 5;
        } else if v1 == 0b10 {
            y = v2 as i32 - 5;
        } else if v1 == 0b11 {
            z = v2 as i32 - 5;
        } else {
            assert!(false);
        }
        Self::new(x, y, z)
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
fn slcd_encdec_test() {
    let slcd = SLCD::new(-3, 0, 0);
    let enc = slcd.encode();
    assert_eq!(enc.0, 1);
    assert_eq!(enc.1, 2);
    let slcd2 = SLCD::decode(enc.0, enc.1);
    assert_eq!(slcd2, slcd);
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
    pub fn decode(v1: u8, v2: u8) -> Self {
        let mut x = 0;
        let mut y = 0;
        let mut z = 0;
        if v1 == 0b01 {
            x = v2 as i32 - 15;
        } else if v1 == 0b10 {
            y = v2 as i32 - 15;
        } else if v1 == 0b11 {
            z = v2 as i32 - 15;
        } else {
            assert!(false);
        }
        Self::new(x, y, z)
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
fn llcd_encdec_test() {
    let llcd = LLCD::new(0, 10, 0);
    let enc = llcd.encode();
    assert_eq!(enc.0, 2);
    assert_eq!(enc.1, 25);
    let llcd2 = LLCD::decode(enc.0, enc.1);
    assert_eq!(llcd2, llcd);
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
        (if self.0.x == self.1.x { 0 } else { 1 })
            + (if self.0.y == self.1.y { 0 } else { 1 })
            + (if self.0.z == self.1.z { 0 } else { 1 })
    }

    pub fn canonical(&self) -> Region {
        let p1 = Position::new(
            min(self.0.x, self.1.x),
            min(self.0.y, self.1.y),
            min(self.0.z, self.1.z),
        );
        let p2 = Position::new(
            max(self.0.x, self.1.x),
            max(self.0.y, self.1.y),
            max(self.0.z, self.1.z),
        );
        Region(p1, p2)
    }

    pub fn contains(&self, p: Position) -> bool {
        let c = self.canonical();
        (c.0.x <= p.x && p.x <= c.1.x)
            && (c.0.y <= p.y && p.y <= c.1.y)
            && (c.0.z <= p.z && p.z <= c.1.z)
    }

    pub fn iter(&self) -> impl Iterator<Item = Position> {
        let c = self.canonical();
        (c.0.z..(c.1.z + 1)).flat_map(move |z| {
            (c.0.y..(c.1.y + 1))
                .flat_map(move |y| (c.0.x..(c.1.x + 1)).map(move |x| Position::new(x, y, z)))
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

impl Nanobot {
    pub fn initial() -> Nanobot {
        Nanobot {
            bid: Bid(1),
            pos: Position::new(0, 0, 0),
            seeds: (2..41).map(|bid| Bid(bid)).collect(),
        }
    }
    pub fn fission(&mut self, ncd: &NCD, m: usize) -> Nanobot {
        let new_c = self.pos + ncd;
        assert!(0 <= new_c.x && 0 <= new_c.y && 0 <= new_c.z);
        assert!(m < self.seeds.len());

        let new_bot = Nanobot {
            bid: self.seeds[0],
            pos: new_c,
            seeds: self.seeds[1..m + 1].to_vec(),
        };
        self.seeds = self.seeds[m + 1..].to_vec();
        new_bot
    }
    pub fn fusion(&mut self, secondary_bot: &mut Nanobot) {
        self.seeds.push(secondary_bot.bid);
        self.seeds.append(&mut secondary_bot.seeds);
        self.seeds.sort();
    }
}

pub fn encode_trace(trace: &[Command]) -> Vec<u8> {
    let mut ret = Vec::with_capacity(trace.len() * 1);
    for t in trace.iter() {
        ret.append(&mut t.encode());
    }
    ret
}

pub fn decode_trace(input: &[u8]) -> Vec<Command> {
    let mut ret = Vec::with_capacity(input.len() / 4);
    let mut offset = 0;
    while offset < input.len() {
        ret.push(Command::decode(input, &mut offset).unwrap());
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
    let trace2 = decode_trace(&result[..]);
    assert_eq!(trace2, trace);
}

pub fn write_trace_file(path: &Path, trace: &[Command]) -> Result<(), Box<Error>> {
    let mut buffer = fs::File::create(path)?;
    buffer.write_all(&encode_trace(trace)[..])?;
    Ok(())
}

pub fn read_trace_file(path: &Path) -> Result<Vec<Command>, Box<Error>> {
    let mut f = fs::File::open(path)?;
    let mut buffer = vec![];
    f.read_to_end(&mut buffer)?;
    let ret = decode_trace(&buffer[..]);
    Ok(ret)
}
