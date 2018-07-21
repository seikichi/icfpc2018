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
    minX: i32,
    maxX: i32,
    minY: i32,
    maxY: i32,
    minZ: i32,
    maxZ: i32,
}

fn calc_bounding_box(model: &Model) -> Option<Bounding> {
    let r = model.matrix.len() as i32;
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
                minX = min(x as i32, minX);
                maxX = max(x as i32, maxX);
                minY = min(y as i32, minY);
                maxY = max(y as i32, maxY);
                minZ = min(z as i32, minZ);
                maxZ = max(z as i32, maxZ);
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

fn move_simple(from: (i32, i32), to: (i32, i32)) -> Vec<Command> {
    const MAX: i32 = 15;
    let mut commands = vec![];
    let xdiff = to.0 - from.0;
    if xdiff != 0 {
        for _ in 0..xdiff / MAX {
            commands.push(Command::SMove(LLCD::new(MAX, 0, 0)));
        }
        commands.push(Command::SMove(LLCD::new(xdiff % MAX, 0, 0)));
    }

    let zdiff = to.1 - from.1;
    if zdiff != 0 {
        for _ in 0..zdiff / MAX {
            commands.push(Command::SMove(LLCD::new(0, 0, MAX)));
        }
        commands.push(Command::SMove(LLCD::new(0, 0, zdiff % MAX)));
    }

    commands
}

fn move_down(len: i32) -> Vec<Command> {
    const MAX: i32 = 15;
    let mut commands = vec![];
    if len != 0 {
        for _ in 0..len / MAX {
            commands.push(Command::SMove(LLCD::new(0, -MAX, 0)));
        }
        commands.push(Command::SMove(LLCD::new(0, -(len % MAX), 0)));
    }
    commands
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
        for y in 0..bounding.maxY + 1 {
            let cur_y = y;
            commands.append(&mut move_simple((0, 0), (bounding.minX, bounding.minZ - 1)));
            for x in 0..(bounding.maxX - bounding.minX + 1) {
                let cur_x = bounding.minX + x;
                let dir = if x % 2 == 0 { 1 } else { -1 };
                commands.push(Command::SMove(LLCD::new(0, 0, dir)));

                for z in 0..(bounding.maxZ - bounding.minZ + 1) {
                    let cur_z = if dir == 1 {
                        bounding.minZ + z
                    } else {
                        bounding.maxZ - z
                    };
                    commands.push(Command::SMove(LLCD::new(0, 0, dir)));
                    if model.matrix[cur_x as usize][cur_y as usize][cur_z as usize] == Voxel::Full {
                        commands.push(Command::Fill(NCD::new(0, 0, -dir)));
                    }
                }
                commands.push(Command::SMove(LLCD::new(1, 0, 0)));
            }

            commands.push(Command::SMove(LLCD::new(0, 1, 0)));
            // back to (x, z) = (0, 0)
            let cur_x = bounding.maxX + 1;
            let cur_z = if (bounding.maxX - bounding.minX + 1) % 2 == 0 {
                bounding.minZ - 1
            } else {
                bounding.maxZ + 1
            };
            commands.append(&mut move_simple((cur_x, cur_z), (0, 0)));
        }
        commands.append(&mut move_down(bounding.maxY + 1));

        // finish
        commands.push(Command::Flip);
        commands.push(Command::Halt);
        commands
    }
}

#[test]
fn generate_commands_for_empty_3x3() {
    let matrix = vec![vec![vec![Voxel::Void; 3]; 3]; 3];
    let model = Model { matrix };

    let ai = SimpleAI::new();
    let commands = ai.generate(&model);
    let expected = vec![Command::Halt];

    assert_eq!(expected, commands);
}

#[test]
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
        Command::SMove(LLCD::new(0, 0, 1)),
        Command::SMove(LLCD::new(0, 0, 1)),
        Command::Fill(NCD::new(0, 0, -1)),
        Command::SMove(LLCD::new(1, 0, 0)),
        // up
        Command::SMove(LLCD::new(0, 1, 0)),
        // back to origin
        Command::SMove(LLCD::new(-2, 0, 0)),
        Command::SMove(LLCD::new(0, 0, -2)),
        Command::SMove(LLCD::new(0, -1, 0)),
        // finish
        Command::Flip,
        Command::Halt,
    ];

    assert_eq!(expected, commands);
}
