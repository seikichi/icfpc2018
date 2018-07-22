use common::*;
use model::*;
use std::cmp::{max, min};
use std::iter::repeat;

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

const SMOVE_MAX: i32 = 15;

pub fn move_straight_x(len: i32) -> Vec<Command> {
    if len == 0 {
        return vec![];
    }
    let mut commands = vec![];
    let dir = if len > 0 { 1 } else { -1 };
    let c = Command::SMove(LLCD::new(dir * SMOVE_MAX, 0, 0));
    commands.extend(repeat(c).take((len.abs() / SMOVE_MAX) as usize));
    if len % SMOVE_MAX != 0 {
        commands.push(Command::SMove(LLCD::new(len % SMOVE_MAX, 0, 0)));
    }
    commands
}

pub fn move_straight_y(len: i32) -> Vec<Command> {
    if len == 0 {
        return vec![];
    }
    let mut commands = vec![];
    let dir = if len > 0 { 1 } else { -1 };
    let c = Command::SMove(LLCD::new(0, dir * SMOVE_MAX, 0));
    commands.extend(repeat(c).take((len.abs() / SMOVE_MAX) as usize));
    if len % SMOVE_MAX != 0 {
        commands.push(Command::SMove(LLCD::new(0, len % SMOVE_MAX, 0)));
    }
    commands
}

pub fn move_straight_z(len: i32) -> Vec<Command> {
    if len == 0 {
        return vec![];
    }
    let mut commands = vec![];
    let dir = if len > 0 { 1 } else { -1 };
    let c = Command::SMove(LLCD::new(0, 0, dir * SMOVE_MAX));
    commands.extend(repeat(c).take((len.abs() / SMOVE_MAX) as usize));
    if len % SMOVE_MAX != 0 {
        commands.push(Command::SMove(LLCD::new(0, 0, len % SMOVE_MAX)));
    }
    commands
}

pub fn generate_devide_commands(size: (usize, usize), split: (usize, usize)) -> Vec<Vec<Command>> {
    let mut commands = vec![];

    let ncd_x1 = NCD::new(1, 0, 0);
    for i in 0..(split.0 - 1) {
        let rest = split.1 * (split.0 - i - 1);

        let mut step = vec![];
        step.extend(repeat(Command::Wait).take(i));
        step.push(Command::Fission(ncd_x1.clone(), rest - 1));
        commands.push(step);

        let width = (size.0 / split.0) as i32 + if i < size.0 % split.0 { 1 } else { 0 };
        let x_moves = move_straight_x(width - 1);
        for m in x_moves.into_iter() {
            let mut step = vec![];
            step.extend(repeat(Command::Wait).take(i + 1));
            step.push(m);
            commands.push(step);
        }
    }

    let ncd_z1 = NCD::new(0, 0, 1);
    for i in 0..(split.1 - 1) {
        let fussion = Command::Fission(ncd_z1.clone(), split.1 - i - 2);
        let width = (size.1 / split.1) as i32 + if i < size.1 % split.1 { 1 } else { 0 };
        let z_moves = move_straight_z(width - 1);

        if i == 0 {
            commands.push(repeat(fussion).take(split.0).collect());

            for m in z_moves.iter() {
                let mut step = vec![];
                step.extend(repeat(Command::Wait).take(split.0));
                step.extend(repeat(m.clone()).take(split.0));
                commands.push(step);
            }
        } else {
            let mut step = vec![];
            step.extend(repeat(Command::Wait).take(split.0));
            for _ in 0..split.0 {
                step.extend(repeat(Command::Wait).take(i - 1));
                step.push(fussion.clone());
            }
            commands.push(step);

            for m in z_moves.iter() {
                let mut step = vec![];
                step.extend(repeat(Command::Wait).take(split.0));
                for _ in 0..split.0 {
                    step.extend(repeat(Command::Wait).take(i));
                    step.push(m.clone());
                }
                commands.push(step);
            }
        }
    }

    commands
}

pub fn generate_concur_commands(size: (usize, usize), split: (usize, usize)) -> Vec<Vec<Command>> {
    let mut commands = vec![];

    // concur z axis
    let ncd_z1 = NCD::new(0, 0, 1);
    let ncd_z_1 = NCD::new(0, 0, -1);
    for i in 0..(split.1 - 1) {
        let width = (size.1 / split.1) as i32 + if (split.1 - i - 2) < size.1 % split.1 {
            1
        } else {
            0
        };
        let z_moves = move_straight_z(-(width - 1));

        if i == split.1 - 2 {
            // last
            for m in z_moves.iter() {
                let mut step = vec![];
                step.extend(repeat(Command::Wait).take(split.0));
                step.extend(repeat(m.clone()).take(split.0));
                commands.push(step);
            }
            let mut step = vec![];
            step.extend(repeat(Command::FusionP(ncd_z1.clone())).take(split.0));
            step.extend(repeat(Command::FusionS(ncd_z_1.clone())).take(split.0));
            commands.push(step);
        } else {
            let z_rest = split.1 - i - 1; // ignore first line

            for m in z_moves.iter() {
                let mut step = vec![];
                step.extend(repeat(Command::Wait).take(split.0));
                for _ in 0..split.0 {
                    step.extend(repeat(Command::Wait).take(z_rest - 1));
                    step.push(m.clone());
                }
                commands.push(step);
            }

            let mut step = vec![];
            step.extend(repeat(Command::Wait).take(split.0));
            for _ in 0..split.0 {
                step.extend(repeat(Command::Wait).take(z_rest - 2));
                step.push(Command::FusionP(ncd_z1.clone()));
                step.push(Command::FusionS(ncd_z_1.clone()));
            }
            commands.push(step);
        }
    }

    // concur x axis
    let ncd_x1 = NCD::new(1, 0, 0);
    let ncd_x_1 = NCD::new(-1, 0, 0);
    for i in 0..(split.0 - 1) {
        let width = (size.0 / split.0) as i32 + if (split.0 - i - 2) < size.0 % split.0 {
            1
        } else {
            0
        };
        let x_moves = move_straight_x(-(width - 1));
        for m in x_moves.into_iter() {
            let mut step = vec![];
            step.extend(repeat(Command::Wait).take(split.0 - i - 1));
            step.push(m);
            commands.push(step);
        }
        // fusion
        let mut step = vec![];
        step.extend(repeat(Command::Wait).take(split.0 - i - 2));
        step.push(Command::FusionP(ncd_x1.clone()));
        step.push(Command::FusionS(ncd_x_1.clone()));
        commands.push(step);
    }

    commands
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

#[test]
fn test_generate_devide_commands_with_1x1() {
    let commands = generate_devide_commands((5, 4), (1, 1));
    let expect: Vec<Vec<Command>> = vec![];
    assert_eq!(expect, commands);
}

#[test]
fn test_generate_devide_commands_with_2x3() {
    let ncd_x1 = NCD::new(1, 0, 0);
    let ncd_z1 = NCD::new(0, 0, 1);

    let llcd_x1 = LLCD::new(1, 0, 0);
    let llcd_z1 = LLCD::new(0, 0, 1);

    let commands = generate_devide_commands((4, 6), (2, 3));
    let expect = vec![
        vec![Command::Fission(ncd_x1.clone(), 2)],
        vec![Command::Wait, Command::SMove(llcd_x1.clone())],
        vec![
            Command::Fission(ncd_z1.clone(), 1),
            Command::Fission(ncd_z1.clone(), 1),
        ],
        vec![
            Command::Wait,
            Command::Wait,
            Command::SMove(llcd_z1.clone()),
            Command::SMove(llcd_z1.clone()),
        ],
        vec![
            Command::Wait,
            Command::Wait,
            Command::Fission(ncd_z1.clone(), 0),
            Command::Fission(ncd_z1.clone(), 0),
        ],
        vec![
            Command::Wait,
            Command::Wait,
            Command::Wait,
            Command::SMove(llcd_z1.clone()),
            Command::Wait,
            Command::SMove(llcd_z1.clone()),
        ],
    ];

    assert_eq!(expect, commands);
}

#[test]
fn test_generate_devide_commands_with_2x2() {
    let ncd_x1 = NCD::new(1, 0, 0);
    let ncd_z1 = NCD::new(0, 0, 1);

    let llcd_x1 = LLCD::new(1, 0, 0);
    let llcd_z2 = LLCD::new(0, 0, 2);

    let commands = generate_devide_commands((3, 5), (2, 2));
    let expect = vec![
        vec![Command::Fission(ncd_x1.clone(), 1)],
        vec![Command::Wait, Command::SMove(llcd_x1.clone())],
        vec![
            Command::Fission(ncd_z1.clone(), 0),
            Command::Fission(ncd_z1.clone(), 0),
        ],
        vec![
            Command::Wait,
            Command::Wait,
            Command::SMove(llcd_z2.clone()),
            Command::SMove(llcd_z2.clone()),
        ],
    ];

    assert_eq!(expect, commands);
}

#[test]
fn test_generate_devide_commands_with_3x2() {
    let ncd_x1 = NCD::new(1, 0, 0);
    let ncd_z1 = NCD::new(0, 0, 1);

    let commands = generate_devide_commands((3, 3), (3, 3));
    let expect = vec![
        vec![Command::Fission(ncd_x1.clone(), 5)],
        vec![Command::Wait, Command::Fission(ncd_x1.clone(), 2)],
        vec![
            Command::Fission(ncd_z1.clone(), 1),
            Command::Fission(ncd_z1.clone(), 1),
            Command::Fission(ncd_z1.clone(), 1),
        ],
        vec![
            Command::Wait,
            Command::Wait,
            Command::Wait,
            Command::Fission(ncd_z1.clone(), 0),
            Command::Fission(ncd_z1.clone(), 0),
            Command::Fission(ncd_z1.clone(), 0),
        ],
    ];

    assert_eq!(expect, commands);
}

#[test]
fn test_generate_concur_commands_with_1x1() {
    let commands = generate_concur_commands((5, 4), (1, 1));
    let expect: Vec<Vec<Command>> = vec![];
    assert_eq!(expect, commands);
}

#[test]
fn test_generate_concur_commands_with_2x2() {
    let ncd_x1 = NCD::new(1, 0, 0);
    let ncd_x_1 = NCD::new(-1, 0, 0);
    let ncd_z1 = NCD::new(0, 0, 1);
    let ncd_z_1 = NCD::new(0, 0, -1);

    let llcd_x_1 = LLCD::new(-1, 0, 0);
    let llcd_z_2 = LLCD::new(0, 0, -2);

    let commands = generate_concur_commands((3, 5), (2, 2));
    let expect = vec![
        vec![
            Command::Wait,
            Command::Wait,
            Command::SMove(llcd_z_2.clone()),
            Command::SMove(llcd_z_2.clone()),
        ],
        vec![
            Command::FusionP(ncd_z1.clone()),
            Command::FusionP(ncd_z1.clone()),
            Command::FusionS(ncd_z_1.clone()),
            Command::FusionS(ncd_z_1.clone()),
        ],
        vec![Command::Wait, Command::SMove(llcd_x_1.clone())],
        vec![
            Command::FusionP(ncd_x1.clone()),
            Command::FusionS(ncd_x_1.clone()),
        ],
    ];
    assert_eq!(expect, commands);
}

#[test]
fn test_generate_concur_commands_with_3x3() {
    let ncd_x1 = NCD::new(1, 0, 0);
    let ncd_x_1 = NCD::new(-1, 0, 0);
    let ncd_z1 = NCD::new(0, 0, 1);
    let ncd_z_1 = NCD::new(0, 0, -1);

    let commands = generate_concur_commands((3, 3), (3, 3));
    let expect = vec![
        vec![
            Command::Wait,
            Command::Wait,
            Command::Wait,
            Command::FusionP(ncd_z1.clone()),
            Command::FusionS(ncd_z_1.clone()),
            Command::FusionP(ncd_z1.clone()),
            Command::FusionS(ncd_z_1.clone()),
            Command::FusionP(ncd_z1.clone()),
            Command::FusionS(ncd_z_1.clone()),
        ],
        vec![
            Command::FusionP(ncd_z1.clone()),
            Command::FusionP(ncd_z1.clone()),
            Command::FusionP(ncd_z1.clone()),
            Command::FusionS(ncd_z_1.clone()),
            Command::FusionS(ncd_z_1.clone()),
            Command::FusionS(ncd_z_1.clone()),
        ],
        vec![
            Command::Wait,
            Command::FusionP(ncd_x1.clone()),
            Command::FusionS(ncd_x_1.clone()),
        ],
        vec![
            Command::FusionP(ncd_x1.clone()),
            Command::FusionS(ncd_x_1.clone()),
        ],
    ];
    assert_eq!(expect, commands);
}
