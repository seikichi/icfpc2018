use common::*;
use std::error::*;
use std::io::BufRead;

#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Debug)]
pub struct Model {
    pub matrix: Vec<Vec<Vec<Voxel>>>,
}

impl Model {
    pub fn new<T: BufRead>(reader: &mut T) -> Result<Self, Box<Error>> {
        let mut buffer = [0; 1];
        reader.read(&mut buffer)?;
        let R: usize = buffer[0] as usize;
        let mut matrix = vec![vec![vec![Voxel::Void; R]; R]; R];

        let mut buffer = vec![];
        reader.read_to_end(&mut buffer)?;
        for (i, byte) in buffer.iter().enumerate() {
            for j in 0..8 {
                if byte & (1 << j) != 0 {
                    continue;
                }

                let pos = i * 8 + j;
                if pos >= R * R * R {
                    break;
                }

                let x = pos / (R * R);
                let y = (pos % (R * R)) / R;
                let z = (pos % (R * R)) % R;
                matrix[x][y][z] = Voxel::Full;
            }
        }
        Ok(Model { matrix })
    }
}

#[test]
fn test_single_voxel() {
    let mut bytes: &[u8] = &[1, 0b10000000];
    let model = Model::new(&mut bytes).unwrap();
    assert_eq!(Voxel::Full, model.matrix[0][0][0]);
}

#[test]
fn test_2x2_voxel() {
    let mut bytes: &[u8] = &[2, 0b01101001];
    let model = Model::new(&mut bytes).unwrap();
    assert_eq!(Voxel::Void, model.matrix[0][0][0]);
    assert_eq!(Voxel::Full, model.matrix[1][0][0]);
    assert_eq!(Voxel::Full, model.matrix[0][1][0]);
    assert_eq!(Voxel::Void, model.matrix[1][1][0]);
    assert_eq!(Voxel::Full, model.matrix[0][0][1]);
    assert_eq!(Voxel::Void, model.matrix[1][0][1]);
    assert_eq!(Voxel::Void, model.matrix[0][1][1]);
    assert_eq!(Voxel::Full, model.matrix[1][1][1]);
}

#[test]
fn test_3x3_voxel() {
    let mut bytes: &[u8] = &[3, 0b10000000, 0b00000000, 0b00000000, 0b00100000];
    let model = Model::new(&mut bytes).unwrap();
    assert_eq!(Voxel::Full, model.matrix[0][0][0]);
    assert_eq!(Voxel::Full, model.matrix[2][2][2]);
}
