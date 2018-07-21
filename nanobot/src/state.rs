use common::*;
use std::collections::HashSet;
use std::error::*;
use std::fmt;
use std::iter::Extend;

#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Debug)]
pub struct State {
    energy: i64,
    harmonics: Harmonics,
    matrix: Vec<Vec<Vec<Voxel>>>,
    bots: Vec<Nanobot>,
}

impl State {
    // returns inital state
    pub fn initial(r: usize) -> State {
        let bot = Nanobot {
            bid: Bid(1),
            pos: Position::new(0, 0, 0),
            seeds: (2..21).map(|bid| Bid(bid)).collect(),
        };
        State {
            energy: 0,
            harmonics: Harmonics::Low,
            matrix: vec![vec![vec![Voxel::Void; r]; r]; r],
            bots: vec![bot],
        }
    }
}

#[derive(Debug)]
pub struct SimulationError {
    message: String,
}

impl SimulationError {
    pub fn new(message: String) -> SimulationError {
        SimulationError {
            message: message.to_string(),
        }
    }
}

impl fmt::Display for SimulationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "SimulationError: {}", self.message)
    }
}

impl Error for SimulationError {
    fn cause(&self) -> Option<&Error> {
        None
    }
}

type VolatileCoordinates = HashSet<Position>;

pub struct UpdateOneOutput {
    pub vc: VolatileCoordinates,
    pub added_bots: Vec<Nanobot>,
    pub deleted_bot_bids: Vec<Bid>,
}

impl UpdateOneOutput {
    fn from_vc(vc: VolatileCoordinates) -> UpdateOneOutput {
        UpdateOneOutput {
            vc,
            added_bots: vec![],
            deleted_bot_bids: vec![],
        }
    }

    fn from_single_volatile_coordinate(p: Position) -> UpdateOneOutput {
        UpdateOneOutput::from_vc(single_volatile_coordinate(p))
    }
}

fn single_volatile_coordinate(p: Position) -> VolatileCoordinates {
    let mut vc = VolatileCoordinates::new();
    vc.insert(p);
    vc
}

fn couple_volatile_coordinates(p1: Position, p2: Position) -> VolatileCoordinates {
    let mut vc = VolatileCoordinates::new();
    vc.insert(p1);
    vc.insert(p2);
    vc
}

impl State {
    pub fn update_time_step(&mut self, commands: &Vec<Command>) -> Result<(), Box<Error>> {
        assert_eq!(commands.len(), self.bots.len());
        let r = self.matrix.len();
        self.energy += (r * r * r) as i64 * match self.harmonics {
            Harmonics::Low => 3,
            Harmonics::High => 30,
        };
        self.energy += self.bots.len() as i64 * 20;

        let mut vcs = VolatileCoordinates::new();
        let mut added_bots = vec![];
        let mut deleted_bot_bids = HashSet::new();

        for (i, command) in commands.iter().enumerate() {
            let output = self.update_one(i, command)?;

            let vc = output.vc;
            if !vcs.is_disjoint(&vc) {
                let message = format!(
                    "nanobot interfere : command={:?}, naonbot_index={}",
                    command, i
                );
                return Err(Box::new(SimulationError::new(message)));
            }
            vcs.extend(vc);

            added_bots.extend(output.added_bots);
            deleted_bot_bids.extend(output.deleted_bot_bids)
        }

        self.bots.retain(|bot| !deleted_bot_bids.contains(&bot.bid));
        self.bots.extend(added_bots);
        self.bots.sort();

        Ok(())
    }

    pub fn update_one(
        &mut self,
        nanobot_index: usize,
        command: &Command,
    ) -> Result<UpdateOneOutput, Box<Error>> {
        let c = self.bots[nanobot_index].pos;
        match command {
            Command::Halt => {
                if c != Position::new(0, 0, 0) {
                    let message = format!(
                        "nanobot position is not origin: command=Halt, naonbot_index={}, c={}",
                        nanobot_index, c
                    );
                    return Err(Box::new(SimulationError::new(message)));
                }
                if self.bots.len() != 1 {
                    let message = format!(
                        "the number of nanobots is not 1: command=Halt, n_nanobots={}",
                        self.bots.len()
                    );
                    return Err(Box::new(SimulationError::new(message)));
                }
                if self.harmonics != Harmonics::Low {
                    let message = format!("harmonics is not Low: command=Halt");
                    return Err(Box::new(SimulationError::new(message)));
                }
                self.bots.pop();

                Ok(UpdateOneOutput::from_single_volatile_coordinate(c))
            }

            Command::Wait => Ok(UpdateOneOutput::from_single_volatile_coordinate(c)),

            Command::Flip => {
                self.harmonics = match self.harmonics {
                    Harmonics::Low => Harmonics::High,
                    Harmonics::High => Harmonics::Low,
                };
                Ok(UpdateOneOutput::from_single_volatile_coordinate(c))
            }

            Command::SMove(llcd) => {
                let vc = self.move_straight(llcd, nanobot_index, command)?;
                Ok(UpdateOneOutput::from_vc(vc))
            }

            Command::LMove(slcd1, slcd2) => {
                let mut vc1 = self.move_straight(slcd1, nanobot_index, command)?;
                let vc2 = self.move_straight(slcd2, nanobot_index, command)?;
                self.energy += 4;
                vc1.extend(&vc2);

                Ok(UpdateOneOutput::from_vc(vc1))
            }

            Command::Fill(ncd) => {
                let new_c = c + ncd;

                if !self.is_valid_coordinate(&new_c) {
                    let message = format!("nanobot is out of matrix: command=Fill, c={}", new_c);
                    return Err(Box::new(SimulationError::new(message)));
                }

                match self.matrix[new_c.z as usize][new_c.y as usize][new_c.x as usize] {
                    Voxel::Void => {
                        self.matrix[new_c.z as usize][new_c.y as usize][new_c.x as usize] =
                            Voxel::Full;
                        self.energy += 12
                    }
                    Voxel::Full => self.energy += 6,
                }

                let mut vc = VolatileCoordinates::new();
                vc.insert(c);
                vc.insert(new_c);

                Ok(UpdateOneOutput::from_vc(vc))
            }

            Command::Fission(ncd, m) => {
                let new_c = c + ncd;
                if !self.is_valid_coordinate(&new_c) {
                    let message = format!("nanobot is out of matrix: command=Fission, c={}", new_c);
                    return Err(Box::new(SimulationError::new(message)));
                }

                let mut bot = &mut self.bots[nanobot_index];
                if *m >= bot.seeds.len() {
                    let message = format!(
                        "too large m: command=Fission, nanobot_index={}, m={}, len={}",
                        nanobot_index,
                        m,
                        bot.seeds.len()
                    );
                    return Err(Box::new(SimulationError::new(message)));
                }

                let new_bot = Nanobot {
                    bid: bot.seeds[0],
                    pos: new_c,
                    seeds: bot.seeds[1..m + 1].to_vec(),
                };

                bot.seeds = bot.seeds[m + 1..].to_vec();

                self.energy += 24;

                Ok(UpdateOneOutput {
                    vc: couple_volatile_coordinates(c, new_c),
                    added_bots: vec![new_bot],
                    deleted_bot_bids: vec![],
                })
            }

            Command::FusionP(ncd) => {
                let secondary_c = c + ncd;
                let secondary_bot_index = self.find_bot_by_coordinate(secondary_c)
                    .ok_or_else(|| {
                        let message = format!(
                            "failed to find nanobot at the location: command={:?}, c={}",
                            command, secondary_c
                        );
                        return Box::new(SimulationError::new(message));
                    })?;
                let mut secondary_bot = self.bots[secondary_bot_index].clone();

                let bot = &mut self.bots[nanobot_index];
                bot.seeds.push(secondary_bot.bid);
                bot.seeds.append(&mut secondary_bot.seeds);
                self.energy -= 24;

                Ok(UpdateOneOutput {
                    vc: couple_volatile_coordinates(c, secondary_c),
                    added_bots: vec![],
                    deleted_bot_bids: vec![secondary_bot.bid],
                })
            }

            Command::FusionS(_) => {
                // do nothing
                Ok(UpdateOneOutput {
                    vc: VolatileCoordinates::new(),
                    added_bots: vec![],
                    deleted_bot_bids: vec![],
                })
            }
        }
    }

    fn find_bot_by_coordinate(&self, p: Position) -> Option<usize> {
        for (i, bot) in self.bots.iter().enumerate() {
            if bot.pos == p {
                return Some(i);
            }
        }
        None
    }

    fn move_straight(
        &mut self,
        diff: &CD,
        nanobot_index: usize,
        command: &Command,
    ) -> Result<VolatileCoordinates, Box<Error>> {
        let c = self.bots[nanobot_index].pos;
        let new_c = c + diff;
        if !self.is_valid_coordinate(&new_c) {
            let message = format!(
                "nanobot is out of matrix: command={:?}, c={}",
                command, new_c
            );
            return Err(Box::new(SimulationError::new(message)));
        }
        for p in region(c, new_c) {
            if self.matrix[p.z as usize][p.y as usize][p.x as usize] == Voxel::Full {
                let message = format!("nanobot hits full voxel : command={:?}, c={}", command, p);
                return Err(Box::new(SimulationError::new(message)));
            }
        }

        self.bots[nanobot_index].pos = new_c;
        self.energy += 2 * diff.manhattan_length() as i64;
        Ok(region(c, new_c).collect())
    }

    fn is_valid_coordinate(&self, p: &Position) -> bool {
        let rx = self.matrix[0][0].len() as i32;
        let ry = self.matrix[0].len() as i32;
        let rz = self.matrix.len() as i32;
        if p.x < 0 || p.x >= rx {
            return false;
        }
        if p.y < 0 || p.y >= ry {
            return false;
        }
        if p.z < 0 || p.z >= rz {
            return false;
        }
        true
    }
}

#[test]
fn test_halt_command() {
    {
        let mut state = State::initial(3);
        let vc = state.update_one(0, &Command::Halt).unwrap().vc;
        assert_eq!(state.bots.len(), 0);
        assert_eq!(vc, single_volatile_coordinate(Position::zero()));
    }

    {
        let mut state = State::initial(3);
        state
            .update_one(0, &Command::SMove(LLCD::new(1, 0, 0)))
            .unwrap();
        let r = state.update_one(0, &Command::Halt);
        assert!(r.is_err());
    }

    {
        let mut state = State::initial(3);
        state.update_one(0, &Command::Flip).unwrap();
        let r = state.update_one(0, &Command::Halt);
        assert!(r.is_err());
    }

    {
        let mut state = State::initial(3);
        state.update_time_step(&vec![Command::Fission(NCD::new(1, 0, 0), 0)]);

        let r = state.update_one(0, &Command::Halt);
        assert!(r.is_err());
    }
}

#[test]
fn test_flip_command() {
    {
        let mut state = State::initial(3);
        let vc = state.update_one(0, &Command::Flip).unwrap().vc;
        assert!(state.harmonics == Harmonics::High);
        assert_eq!(vc, single_volatile_coordinate(Position::zero()));
        state.update_one(0, &Command::Flip).unwrap();
        assert!(state.harmonics == Harmonics::Low);
    }
}

#[test]
fn test_smove_command() {
    {
        let mut state = State::initial(3);
        state
            .update_one(0, &Command::SMove(LLCD::new(1, 0, 0)))
            .unwrap();
        assert_eq!(state.bots[0].pos, Position::new(1, 0, 0));
        assert_eq!(state.energy, 2);
        let vc = state
            .update_one(0, &Command::SMove(LLCD::new(0, 2, 0)))
            .unwrap()
            .vc;
        assert_eq!(state.bots[0].pos, Position::new(1, 2, 0));
        assert_eq!(state.energy, 6);
        assert_eq!(
            vc,
            region(Position::new(1, 0, 0), Position::new(1, 2, 0)).collect()
        );
        state
            .update_one(0, &Command::SMove(LLCD::new(0, 0, 1)))
            .unwrap();
        assert_eq!(state.bots[0].pos, Position::new(1, 2, 1));
        assert_eq!(state.energy, 8);
    }
    {
        let mut state = State::initial(3);
        let r = state.update_one(0, &Command::SMove(LLCD::new(0, 0, -1)));
        assert!(r.is_err());
    }
    {
        let mut state = State::initial(3);
        let r = state.update_one(0, &Command::SMove(LLCD::new(3, 0, 0)));
        assert!(r.is_err());
    }
    {
        let mut state = State::initial(3);
        state.update_one(0, &Command::Fill(NCD::new(1, 0, 0)));
        let r = state.update_one(0, &Command::SMove(LLCD::new(1, 0, 0)));
        assert!(r.is_err());
    }
}

#[test]
fn test_lmove_command() {
    {
        let mut state = State::initial(3);
        let vc = state
            .update_one(0, &Command::LMove(SLCD::new(1, 0, 0), SLCD::new(0, 1, 0)))
            .unwrap()
            .vc;
        let mut expected_vc = VolatileCoordinates::new();
        expected_vc.insert(Position::new(0, 0, 0));
        expected_vc.insert(Position::new(1, 0, 0));
        expected_vc.insert(Position::new(1, 1, 0));
        assert_eq!(state.bots[0].pos, Position::new(1, 1, 0));
        assert_eq!(state.energy, 8);
        assert_eq!(vc, expected_vc);
        state
            .update_one(0, &Command::LMove(SLCD::new(0, 0, 1), SLCD::new(0, 0, -1)))
            .unwrap();
        assert_eq!(state.bots[0].pos, Position::new(1, 1, 0));
        assert_eq!(state.energy, 16);
    }
    {
        let mut state = State::initial(3);
        let r = state.update_one(0, &Command::LMove(SLCD::new(0, 0, 4), SLCD::new(0, 0, 1)));
        assert!(r.is_err());
    }
    {
        let mut state = State::initial(3);
        let r = state.update_one(0, &Command::LMove(SLCD::new(0, 0, 1), SLCD::new(0, -3, 0)));
        assert!(r.is_err());
    }
}

#[test]
fn test_fill_command() {
    {
        let mut state = State::initial(3);
        assert_eq!(state.matrix[0][0][1], Voxel::Void);
        let vc = state
            .update_one(0, &Command::Fill(NCD::new(1, 0, 0)))
            .unwrap()
            .vc;
        let mut expected_vc = VolatileCoordinates::new();
        expected_vc.insert(Position::new(0, 0, 0));
        expected_vc.insert(Position::new(1, 0, 0));
        assert_eq!(state.matrix[0][0][1], Voxel::Full);
        assert_eq!(state.energy, 12);
        assert_eq!(vc, expected_vc);
        state
            .update_one(0, &Command::Fill(NCD::new(1, 0, 0)))
            .unwrap();
        assert_eq!(state.energy, 18);
    }
    {
        let mut state = State::initial(3);
        let r = state.update_one(0, &Command::Fill(NCD::new(-1, 0, 0)));
        assert!(r.is_err());
    }
    {
        let mut state = State::initial(3);
        state
            .update_one(0, &Command::SMove(LLCD::new(2, 0, 0)))
            .unwrap();
        let r = state.update_one(0, &Command::Fill(NCD::new(1, 0, 0)));
        assert!(r.is_err());
    }
}

#[test]
fn test_fission_command() {
    {
        let mut state = State::initial(3);
        let output = state
            .update_one(0, &Command::Fission(NCD::new(1, 0, 0), 1))
            .unwrap();
        let mut expected_vc = VolatileCoordinates::new();
        expected_vc.insert(Position::new(0, 0, 0));
        expected_vc.insert(Position::new(1, 0, 0));
        assert_eq!(state.energy, 24);
        assert_eq!(state.bots.len(), 1);
        assert_eq!(state.bots[0].pos, Position::zero());
        assert_eq!(state.bots[0].bid, Bid(1));
        assert_eq!(
            state.bots[0].seeds,
            (4..21).map(|i| Bid(i)).collect::<Vec<Bid>>()
        );
        assert_eq!(output.vc, expected_vc);
        assert_eq!(output.added_bots.len(), 1);
        assert_eq!(output.added_bots[0].pos, Position::new(1, 0, 0));
        assert_eq!(output.added_bots[0].bid, Bid(2));
        assert_eq!(output.added_bots[0].seeds, vec![Bid(3)]);
    }
    {
        let mut state = State::initial(3);
        let r = state.update_one(0, &Command::Fission(NCD::new(-1, 0, 0), 1));
        assert!(r.is_err());
    }
    {
        let mut state = State::initial(3);
        let r = state.update_one(0, &Command::Fission(NCD::new(1, 0, 0), 19));
        assert!(r.is_err());
    }
    {
        let mut state = State::initial(3);
        let r = state.update_one(0, &Command::Fission(NCD::new(1, 0, 0), 0));
        assert!(r.is_ok());
    }
    {
        let mut state = State::initial(3);
        state.bots[0].seeds = vec![];
        let r = state.update_one(0, &Command::Fission(NCD::new(1, 0, 0), 0));
        assert!(r.is_err());
    }
}

#[test]
fn test_update_time_step() {
    {
        let mut state = State::initial(3);
        let commands = vec![Command::Wait];
        state.update_time_step(&commands).unwrap();
        let mut expected_energy = 3 * 3 * 3 * 3 + 20;
        assert_eq!(state.energy, expected_energy);

        let commands = vec![Command::Flip];
        state.update_time_step(&commands).unwrap();
        expected_energy += 3 * 3 * 3 * 3 + 20;
        assert_eq!(state.energy, expected_energy);

        let commands = vec![Command::Wait];
        state.update_time_step(&commands).unwrap();
        expected_energy += 3 * 3 * 3 * 30 + 20;
        assert_eq!(state.energy, expected_energy);
    }

    {
        let mut state = State::initial(3);
        state.update_time_step(&vec![Command::Fission(NCD::new(1, 0, 0), 5)]).unwrap();
        state.update_time_step(&vec![
            Command::Fission(NCD::new(0, 1, 0), 1),
            Command::Fission(NCD::new(1, 1, 0), 1),
        ]).unwrap();

        assert_eq!(
            state.bots.iter().map(|bot| bot.bid).collect::<Vec<_>>(),
            vec![Bid(1), Bid(2), Bid(3), Bid(8)])
    }

    {
        let mut state = State::initial(3);
        state.update_time_step(&vec![Command::Fission(NCD::new(1, 0, 0), 0)]);
        let commands = vec![Command::Wait, Command::SMove(LLCD::new(-1, 0, 0))];
        let r = state.update_time_step(&commands);
        assert!(r.is_err());
    }

    {
        // xxx
        // xxx
        // 12x

        // x2x
        // 131
        // 12x
        let mut state = State::initial(3);
        state.update_time_step(&vec![Command::Fission(NCD::new(1, 0, 0), 0)]);
        let commands = vec![
            Command::LMove(SLCD::new(0, 1, 0), SLCD::new(2, 0, 0)),
            Command::SMove(LLCD::new(0, 2, 0)),
        ];
        let r = state.update_time_step(&commands);
        assert!(r.is_err());
    }
}
