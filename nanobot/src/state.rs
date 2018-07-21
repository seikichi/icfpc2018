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
        let c = self.bots[nanobot_index].pos.clone();
        match command {
            Command::Halt => {
                if c != Position::new(0, 0, 0) {
                    let message = format!("nanobot position is not origin: command=Halt, naonbot_index={}, c={:?}", nanobot_index, c);
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
            _ => unimplemented!()
        }
    }
}
