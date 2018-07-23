#![allow(dead_code)]

use common::*;
use std::error::*;
use std::io::BufRead;

#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Debug)]
pub struct Model {
    pub matrix: Vec<Vec<Vec<Voxel>>>,
}

impl Model {
    pub fn initial(r: usize) -> Model {
        Model {
            matrix: vec![vec![vec![Voxel::Void; r]; r]; r],
        }
    }
    pub fn new<T: BufRead>(reader: &mut T) -> Result<Self, Box<Error>> {
        let mut buffer = [0; 1];
        reader.read(&mut buffer)?;
        let r: usize = buffer[0] as usize;
        let mut matrix = vec![vec![vec![Voxel::Void; r]; r]; r];

        let mut buffer: Vec<u8> = vec![];
        reader.read_to_end(&mut buffer)?;

        for (i, byte) in buffer.iter().enumerate() {
            for j in 0..8 {
                if byte & (1 << j) == 0 {
                    continue;
                }

                let pos = i * 8 + j;
                if pos >= r * r * r {
                    break;
                }

                let x = pos / (r * r);
                let y = (pos % (r * r)) / r;
                let z = pos % r;
                assert!(1 <= x && x <= r - 2 && y <= r - 2 && 1 <= z && z <= r - 2);
                matrix[x][y][z] = Voxel::Full;
            }
        }
        Ok(Model { matrix })
    }
    pub fn voxel_at(&self, p: Position) -> Voxel {
        self.matrix[p.x as usize][p.y as usize][p.z as usize]
    }

    pub fn set_voxel_at(&mut self, p: Position, v: Voxel) {
        self.matrix[p.x as usize][p.y as usize][p.z as usize] = v
    }
}

#[test]
fn test_3x3_model_with_single_full_voxel() {
    let mut bytes: &[u8] = &[3, 0b0000000, 0b00000100, 0b00000000, 0b00000000];
    let model = Model::new(&mut bytes).unwrap();
    assert_eq!(Voxel::Full, model.matrix[1][0][1]);
}

#[test]
fn test_3x3_model_with_2_full_voxels() {
    let mut bytes: &[u8] = &[3, 0b0000000, 0b00100100, 0b00000000, 0b00000000];
    let model = Model::new(&mut bytes).unwrap();
    assert_eq!(Voxel::Full, model.matrix[1][0][1]);
    assert_eq!(Voxel::Full, model.matrix[1][1][1]);
}
