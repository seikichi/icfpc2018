use ai::AI;
use common::*;
use model::*;
use std::cmp::{max, min};

pub struct SimpleAI {}

impl SimpleAI {
    pub fn new() -> Self {
        SimpleAI {}
    }
}

struct Bounding {
    minX: usize,
    maxX: usize,
    minY: usize,
    maxY: usize,
    minZ: usize,
    maxZ: usize,
}

fn calc_bounding_box(model: &Model) -> Option<Bounding> {
    let r = model.matrix.len();
    let mut minX = r;
    let mut maxX = 0;
    let mut minY = r;
    let mut maxY = 0;
    let mut minZ = r;
    let mut maxZ = 0;
    let mut found = false;
    for (x, plane) in model.matrix.iter().enumerate() {
        for (y, line) in plane.iter().enumerate() {
            for (z, voxel) in line.iter().enumerate() {
                if *voxel == Voxel::Void {
                    continue;
                }
                found = true;
                minX = min(x, minX);
                maxX = max(x, maxX);
                minY = min(y, minY);
                maxY = max(y, maxY);
                minZ = min(z, minZ);
                maxZ = max(z, maxZ);
            }
        }
    }
    if !found {
        return None;
    }
    Some(Bounding {
        minX,
        maxX,
        minY,
        maxY,
        minZ,
        maxZ,
    })
}

impl AI for SimpleAI {
    fn generate(&self, model: &Model) -> Vec<Command> {
        let mut commands = vec![];
        let bounding = match calc_bounding_box(model) {
            Some(b) => b,
            None => {
                return vec![Command::Halt];
            }
        };

        commands.push(Command::Flip);
        // start!
        // finish
        commands.push(Command::Flip);
        commands.push(Command::Halt);
        commands
    }
}

#[test]
fn generate_commands_for_empty_3x3() {
    let mut matrix = vec![vec![vec![Voxel::Void; 3]; 3]; 3];
    let model = Model { matrix };

    let ai = SimpleAI::new();
    let commands = ai.generate(&model);
    let expected = vec![Command::Halt];

    assert_eq!(expected, commands);
}

#[test]
#[ignore]
fn generate_commands_for_3x3_with_1_full_voxel() {
    let mut matrix = vec![vec![vec![Voxel::Void; 3]; 3]; 3];
    matrix[1][0][1] = Voxel::Full;
    let model = Model { matrix };

    let ai = SimpleAI::new();
    let commands = ai.generate(&model);
    let expected = vec![
        Command::Flip,
        // move to bounding box corner
        Command::SMove(LLCD::new(1, 0, 0)),
        // start filling (move -> fill)
        Command::SMove(LLCD::new(0, 0, 2)),
        Command::Fill(NCD::new(0, 0, -1)),
        // up
        Command::SMove(LLCD::new(0, 1, 0)),
        // back to origin
        Command::SMove(LLCD::new(-1, 0, 0)),
        Command::SMove(LLCD::new(0, 0, -2)),
        Command::SMove(LLCD::new(0, -1, 0)),
        // finish
        Command::Flip,
        Command::Halt,
    ];

    assert_eq!(expected, commands);
}
