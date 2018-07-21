use common::*;
use model::*;
use std::cmp::{max, min};

#[derive(Debug)]
pub struct Bounding {
    pub min_x: i32,
    pub max_x: i32,
    pub min_y: i32,
    pub max_y: i32,
    pub min_z: i32,
    pub max_z: i32,
}

pub fn calc_bounding_box(model: &Model) -> Option<Bounding> {
    let r = model.matrix.len() as i32;
    let mut min_x = r;
    let mut max_x = 0;
    let mut min_y = r;
    let mut max_y = 0;
    let mut min_z = r;
    let mut max_z = 0;
    let mut found = false;

    for x in 0..r {
        for y in 0..r {
            for z in 0..r {
                let voxel = &model.matrix[x as usize][y as usize][z as usize];
                if *voxel == Voxel::Void {
                    continue;
                }

                found = true;
                min_x = min(x as i32, min_x);
                max_x = max(x as i32, max_x);
                min_y = min(y as i32, min_y);
                max_y = max(y as i32, max_y);
                min_z = min(z as i32, min_z);
                max_z = max(z as i32, max_z);
            }
        }
    }
    if !found {
        return None;
    }
    Some(Bounding {
        min_x,
        max_x,
        min_y,
        max_y,
        min_z,
        max_z,
    })
}

#[test]
fn test_bounding_box() {
    let mut matrix = vec![vec![vec![Voxel::Void; 4]; 4]; 4];
    matrix[1][0][1] = Voxel::Full;
    matrix[1][0][2] = Voxel::Full;
    matrix[2][0][1] = Voxel::Full;
    matrix[2][0][2] = Voxel::Full;
    matrix[1][1][1] = Voxel::Full;
    let model = Model { matrix };

    let bounding = calc_bounding_box(&model).unwrap();
    assert_eq!(1, bounding.min_x);
    assert_eq!(2, bounding.max_x);
    assert_eq!(0, bounding.min_y);
    assert_eq!(1, bounding.max_y);
    assert_eq!(1, bounding.min_z);
    assert_eq!(2, bounding.max_z);
}
