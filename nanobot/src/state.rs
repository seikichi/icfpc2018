use common::*;
use std::fmt;
use std::error::*;

#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Debug)]
pub struct State {
    energy: i64,
    harmonics: Harmonics,
    matrix: Vec<Vec<Vec<Voxel>>>,
    bots: Vec<Nanobot>,
}

#[derive(Debug)]
pub struct SimulationError {
    message: String,
}

impl SimulationError {
    pub fn new(message: String) -> SimulationError {
        SimulationError{message: message.to_string()}
    }
}

impl fmt::Display for SimulationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "SimulationError: {}", self.message)
    }
}

impl Error for SimulationError {
    fn cause(&self) -> Option<&Error> { None }
}

impl State {
    pub fn update_one(&mut self, nanobot_index: usize, command: &Command) -> Result<(), Box<Error>> {
        let c = self.bots[nanobot_index].pos;
        match command {
            Command::Halt => {
                if c != Position::new(0, 0, 0) {
                    let message = format!("nanobot position is not origin: command=Halt, naonbot_index={}, c={}", nanobot_index, c);
                    return Err(Box::new(SimulationError::new(message)));
                }
                if self.bots.len() != 1 {
                    let message = format!("the number of nanobots is not 1: command=Halt, n_nanobots={}", self.bots.len());
                    return Err(Box::new(SimulationError::new(message)));
                }
                if self.harmonics != Harmonics::Low {
                    let message = format!("harmonics is not Low: command=Halt");
                    return Err(Box::new(SimulationError::new(message)));
                }
                self.bots.pop();
                Ok(())
            },

            Command::Wait => {
                Ok(())
            },

            Command::Flip => {
                self.harmonics = match self.harmonics {
                    Harmonics::Low => Harmonics::High,
                    Harmonics::High => Harmonics::Low,
                };
                Ok(())
            },

            Command::SMove(llcd) => {
                let new_c = c + llcd;
                if !self.is_valid_coordinate(&new_c) {
                    let message = format!("nanobot is out of matrix: command=SMove, c={}", new_c);
                    return Err(Box::new(SimulationError::new(message)))
                }
                self.bots[nanobot_index].pos = new_c;
                self.energy += 2 * llcd.manhattan_length() as i64;
                Ok(())
            },

            Command::LMove(slcd1, slcd2) => {
                let new_c1 = c + slcd1;
                let new_c2 = new_c1 + slcd2;

                for new_c in vec![&new_c1, &new_c2] {
                    if !self.is_valid_coordinate(new_c) {
                        let message = format!("nanobot is out of matrix: command=SMove, c={}", new_c);
                        return Err(Box::new(SimulationError::new(message)))
                    }
                }

                self.bots[nanobot_index].pos = new_c2;
                self.energy += 2 * (slcd1.manhattan_length() + slcd2.manhattan_length() + 2) as i64;

                Ok(())
            }
            _ => unimplemented!()
        }
    }

    fn is_valid_coordinate(&self, p: &Position) -> bool {
        let rx = self.matrix[0][0].len() as i32;
        let ry = self.matrix[0].len() as i32;
        let rz = self.matrix.len() as i32;
        if p.x < 0 || p.x >= rx { return false; }
        if p.y < 0 || p.y >= ry { return false; }
        if p.z < 0 || p.z >= rz { return false; }
        true
    }
}
