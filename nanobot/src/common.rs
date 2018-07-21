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
    fn encode(&self) -> Vec<u8> {
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
}

#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Debug)]
pub struct NCD {
    x: i32,
    y: i32,
    z: i32,
}

impl NCD {
    fn encode(&self) -> u8 {
        ((self.x + 1) * 9 + (self.y + 1) * 3 + (self.z + 1)) as u8
    }
}

#[test]
fn ncd_encode_test() {
    let ncd = NCD { x: 1, y: 0, z: 0 };
    assert_eq!(ncd.encode(), 18 + 3 + 1);
}

#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Debug)]
pub struct SLCD {
    x: i32,
    y: i32,
    z: i32,
}
impl SLCD {
    fn encode(&self) -> (u8, u8) {
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
#[test]
fn slcd_encode_test() {
    let slcd = SLCD { x: -3, y: 0, z: 0 };
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
    fn encode(&self) -> (u8, u8) {
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
#[test]
fn llcd_encode_test() {
    let llcd = LLCD { x: 0, y: 10, z: 0 };
    let enc = llcd.encode();
    assert_eq!(enc.0, 2);
    assert_eq!(enc.1, 25);
}

#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Debug)]
pub struct Bid(usize);

#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Debug)]
pub struct Nanobot {
    bid: usize,
    pos: (i32, i32, i32),
    seeds: Vec<Bid>,
}
