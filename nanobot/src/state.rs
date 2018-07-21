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
            seeds: (2..20).map(|bid| Bid(bid)).collect(),
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

fn single_volatile_coordinate(p: Position) -> VolatileCoordinates {
    let mut vc = VolatileCoordinates::new();
    vc.insert(p);
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
        for (i, command) in commands.iter().enumerate() {
            let vc = self.update_one(i, command)?;
            if !vcs.is_disjoint(&vc) {
                let message = format!(
                    "nanobot interfere : command={:?}, naonbot_index={}",
                    command, i
                );
                return Err(Box::new(SimulationError::new(message)));
            }
            vcs.extend(vc);
        }
        Ok(())
    }
    pub fn update_one(
        &mut self,
        nanobot_index: usize,
        command: &Command,
    ) -> Result<VolatileCoordinates, Box<Error>> {
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

                Ok(single_volatile_coordinate(c))
            }

            Command::Wait => Ok(single_volatile_coordinate(c)),

            Command::Flip => {
                self.harmonics = match self.harmonics {
                    Harmonics::Low => Harmonics::High,
                    Harmonics::High => Harmonics::Low,
                };
                Ok(single_volatile_coordinate(c))
            }

            Command::SMove(llcd) => self.move_straight(llcd, nanobot_index, command),

            Command::LMove(slcd1, slcd2) => {
                let mut vc1 = self.move_straight(slcd1, nanobot_index, command)?;
                let vc2 = self.move_straight(slcd2, nanobot_index, command)?;
                self.energy += 4;
                vc1.extend(&vc2);

                Ok(vc1)
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

                Ok(vc)
            }

            _ => unimplemented!(),
        }
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
        let vc = state.update_one(0, &Command::Halt).unwrap();
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

        let new_bot = state.bots[0].clone();
        state.bots.push(new_bot); // FIXME: 後でFissionにする

        let r = state.update_one(0, &Command::Halt);
        assert!(r.is_err());
    }
}

#[test]
fn test_flip_command() {
    {
        let mut state = State::initial(3);
        let vc = state.update_one(0, &Command::Flip).unwrap();
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
            .unwrap();
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
}

#[test]
fn test_lmove_command() {
    {
        let mut state = State::initial(3);
        let vc = state
            .update_one(0, &Command::LMove(SLCD::new(1, 0, 0), SLCD::new(0, 1, 0)))
            .unwrap();
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
            .unwrap();
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

    // TODO interfere check
}
