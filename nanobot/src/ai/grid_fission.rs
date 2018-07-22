use ai::config::Config;
use ai::utils::*;
use ai::AssembleAI;
use common::*;
use model::*;

use std::cmp::min;
use std::iter::repeat;

pub struct GridFissionAI {}

impl GridFissionAI {
    pub fn new(_config: &Config) -> Self {
        GridFissionAI {}
    }
}

impl AssembleAI for GridFissionAI {
    fn assemble(&self, model: &Model) -> Vec<Command> {
        let bounding = match calc_bounding_box(model) {
            Some(b) => b,
            None => {
                return vec![Command::Halt];
            }
        };
        let x_size = (bounding.max_x - bounding.min_x + 1) as usize;
        let z_size = (bounding.max_z - bounding.min_z + 1) as usize;

        // TODO FIX ME;
        let xsplit = min(x_size, 4);
        let zsplit = min(z_size, 5);

        let mut commands = vec![];
        commands.extend(move_straight_x(bounding.min_x));
        commands.extend(move_straight_z(bounding.min_z));
        commands.push(Command::Flip);
        commands.extend(generate_devide_commands((x_size, z_size), (xsplit, zsplit)));

        let mut commands_list: Vec<Vec<Command>> = vec![];

        let mut x_width_list = vec![];
        let mut z_width_list = vec![];
        for i in 0..xsplit {
            let width = (x_size / xsplit) as i32 + if i < x_size % xsplit { 1 } else { 0 };
            x_width_list.push(width);
        }
        for i in 0..zsplit {
            let width = (z_size / zsplit) as i32 + if i < z_size % zsplit { 1 } else { 0 };
            z_width_list.push(width);
        }

        // 1st line
        for i in 0..xsplit {
            let mut x = bounding.min_x;
            for j in 0..i {
                x += x_width_list[j];
            }
            let initial = Position::new(x, 0, bounding.min_z);
            let size = Position::new(x_width_list[i], bounding.max_y, z_width_list[0]);
            commands_list.push(generate_region_commands(model, initial, size));
        }
        // others
        for ri in 0..xsplit {
            let i = xsplit - ri - 1;
            let mut x = bounding.min_x;
            for j in 0..i {
                x += x_width_list[j];
            }
            let x_width = x_width_list[i];

            for j in 1..zsplit {
                let mut z = bounding.min_z;
                for k in 0..j {
                    z += z_width_list[k];
                }
                let z_width = z_width_list[j];

                let initial = Position::new(x, 0, z);
                let size = Position::new(x_width, bounding.max_y, z_width);
                commands_list.push(generate_region_commands(model, initial, size));
            }
        }

        let mut index = 0;
        loop {
            let mut all_wait = true;
            for v in commands_list.iter() {
                commands.push(if index >= v.len() {
                    Command::Wait
                } else {
                    all_wait = false;
                    v[index].clone()
                });
            }
            index += 1;
            if all_wait {
                break;
            }
        }

        commands.extend(generate_concur_commands((x_size, z_size), (xsplit, zsplit)));
        commands.extend(move_straight_x(-bounding.min_x));
        commands.extend(move_straight_z(-bounding.min_z));
        commands.extend(move_straight_y(-(bounding.max_y + 1)));
        commands.push(Command::Flip);
        commands.push(Command::Halt);
        commands
    }
}

const SMOVE_MAX: i32 = 15;

fn move_straight_x(len: i32) -> Vec<Command> {
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

fn move_straight_y(len: i32) -> Vec<Command> {
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

fn move_straight_z(len: i32) -> Vec<Command> {
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

fn generate_devide_commands(size: (usize, usize), split: (usize, usize)) -> Vec<Command> {
    let mut commands = vec![];

    let ncd_x1 = NCD::new(1, 0, 0);
    for i in 0..(split.0 - 1) {
        let rest = split.1 * (split.0 - i - 1);
        commands.extend(repeat(Command::Wait).take(i));
        commands.push(Command::Fission(ncd_x1.clone(), rest - 1));
        let width = (size.0 / split.0) as i32 + if i < size.0 % split.0 { 1 } else { 0 };
        let x_moves = move_straight_x(width - 1);
        for m in x_moves.into_iter() {
            commands.extend(repeat(Command::Wait).take(i + 1));
            commands.push(m);
        }
    }

    let ncd_z1 = NCD::new(0, 0, 1);
    for i in 0..(split.1 - 1) {
        let fussion = Command::Fission(ncd_z1.clone(), split.1 - i - 2);
        let width = (size.1 / split.1) as i32 + if i < size.1 % split.1 { 1 } else { 0 };
        let z_moves = move_straight_z(width - 1);

        if i == 0 {
            commands.extend(repeat(fussion).take(split.0));
            for m in z_moves.iter() {
                commands.extend(repeat(Command::Wait).take(split.0));
                commands.extend(repeat(m.clone()).take(split.0));
            }
        } else {
            commands.extend(repeat(Command::Wait).take(split.0));
            for _ in 0..split.0 {
                commands.extend(repeat(Command::Wait).take(i - 1));
                commands.push(fussion.clone());
            }

            for m in z_moves.iter() {
                commands.extend(repeat(Command::Wait).take(split.0));
                for _ in 0..split.0 {
                    commands.extend(repeat(Command::Wait).take(i));
                    commands.push(m.clone());
                }
            }
        }
    }

    commands
}

fn generate_concur_commands(size: (usize, usize), split: (usize, usize)) -> Vec<Command> {
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
                commands.extend(repeat(Command::Wait).take(split.0));
                commands.extend(repeat(m.clone()).take(split.0));
            }
            commands.extend(repeat(Command::FusionP(ncd_z1.clone())).take(split.0));
            commands.extend(repeat(Command::FusionS(ncd_z_1.clone())).take(split.0));
        } else {
            let z_rest = split.1 - i - 1; // ignore first line

            for m in z_moves.iter() {
                commands.extend(repeat(Command::Wait).take(split.0));
                for _ in 0..split.0 {
                    commands.extend(repeat(Command::Wait).take(z_rest - 1));
                    commands.push(m.clone());
                }
            }

            commands.extend(repeat(Command::Wait).take(split.0));
            for _ in 0..split.0 {
                commands.extend(repeat(Command::Wait).take(z_rest - 2));
                commands.push(Command::FusionP(ncd_z1.clone()));
                commands.push(Command::FusionS(ncd_z_1.clone()));
            }
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
            commands.extend(repeat(Command::Wait).take(split.0 - i - 1));
            commands.push(m);
        }
        // fusion
        commands.extend(repeat(Command::Wait).take(split.0 - i - 2));
        commands.push(Command::FusionP(ncd_x1.clone()));
        commands.push(Command::FusionS(ncd_x_1.clone()));
    }

    commands
}

fn generate_region_commands(model: &Model, initial: Position, size: Position) -> Vec<Command> {
    let mut commands = vec![];

    let mut x = initial.x;
    let mut z = initial.z;

    let ncd_y_1 = NCD::new(0, -1, 0);
    let llcd_y1 = LLCD::new(0, 1, 0);

    let mut xdir = LLCD::new(1, 0, 0);
    let mut zdir = LLCD::new(0, 0, 1);

    for i in 0..size.y + 1 {
        let y = i + 1;
        commands.push(Command::SMove(llcd_y1.clone()));
        for j in 0..size.z {
            for k in 0..size.x {
                if model.matrix[x as usize][(y - 1) as usize][z as usize] == Voxel::Full {
                    commands.push(Command::Fill(ncd_y_1.clone()));
                }

                if k != size.x - 1 {
                    commands.push(Command::SMove(xdir.clone()));
                    x += xdir.x;
                }
            }
            xdir = LLCD::new(-1 * xdir.x, 0, 0);

            if j != size.z - 1 {
                commands.push(Command::SMove(zdir.clone()));
                z += zdir.z;
            }
        }
        zdir = LLCD::new(0, 0, -1 * zdir.z);
    }

    commands.extend(move_straight_x(initial.x - x));
    commands.extend(move_straight_z(initial.z - z));
    commands
}

#[test]
fn test_generate_devide_commands_with_1x1() {
    let commands = generate_devide_commands((5, 4), (1, 1));
    let expect: Vec<Command> = vec![];
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
        // step
        Command::Fission(ncd_x1.clone(), 2),
        // step
        Command::Wait,
        Command::SMove(llcd_x1.clone()),
        // step
        Command::Fission(ncd_z1.clone(), 1),
        Command::Fission(ncd_z1.clone(), 1),
        // step
        Command::Wait,
        Command::Wait,
        Command::SMove(llcd_z1.clone()),
        Command::SMove(llcd_z1.clone()),
        // step
        Command::Wait,
        Command::Wait,
        Command::Fission(ncd_z1.clone(), 0),
        Command::Fission(ncd_z1.clone(), 0),
        // Step
        Command::Wait,
        Command::Wait,
        Command::Wait,
        Command::SMove(llcd_z1.clone()),
        Command::Wait,
        Command::SMove(llcd_z1.clone()),
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
        // step
        Command::Fission(ncd_x1.clone(), 1),
        // step
        Command::Wait,
        Command::SMove(llcd_x1.clone()),
        // step
        Command::Fission(ncd_z1.clone(), 0),
        Command::Fission(ncd_z1.clone(), 0),
        // step
        Command::Wait,
        Command::Wait,
        Command::SMove(llcd_z2.clone()),
        Command::SMove(llcd_z2.clone()),
    ];

    assert_eq!(expect, commands);
}

#[test]
fn test_generate_devide_commands_with_3x2() {
    let ncd_x1 = NCD::new(1, 0, 0);
    let ncd_z1 = NCD::new(0, 0, 1);

    let commands = generate_devide_commands((3, 3), (3, 3));
    let expect = vec![
        // step
        Command::Fission(ncd_x1.clone(), 5),
        // step
        Command::Wait,
        Command::Fission(ncd_x1.clone(), 2),
        // step
        Command::Fission(ncd_z1.clone(), 1),
        Command::Fission(ncd_z1.clone(), 1),
        Command::Fission(ncd_z1.clone(), 1),
        // step
        Command::Wait,
        Command::Wait,
        Command::Wait,
        Command::Fission(ncd_z1.clone(), 0),
        Command::Fission(ncd_z1.clone(), 0),
        Command::Fission(ncd_z1.clone(), 0),
    ];

    assert_eq!(expect, commands);
}

#[test]
fn test_generate_concur_commands_with_1x1() {
    let commands = generate_concur_commands((5, 4), (1, 1));
    let expect: Vec<Command> = vec![];
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
    let expect: Vec<Command> = vec![
        // step
        Command::Wait,
        Command::Wait,
        Command::SMove(llcd_z_2.clone()),
        Command::SMove(llcd_z_2.clone()),
        // step
        Command::FusionP(ncd_z1.clone()),
        Command::FusionP(ncd_z1.clone()),
        Command::FusionS(ncd_z_1.clone()),
        Command::FusionS(ncd_z_1.clone()),
        // step
        Command::Wait,
        Command::SMove(llcd_x_1.clone()),
        // step
        Command::FusionP(ncd_x1.clone()),
        Command::FusionS(ncd_x_1.clone()),
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
    let expect: Vec<Command> = vec![
        // step
        Command::Wait,
        Command::Wait,
        Command::Wait,
        Command::FusionP(ncd_z1.clone()),
        Command::FusionS(ncd_z_1.clone()),
        Command::FusionP(ncd_z1.clone()),
        Command::FusionS(ncd_z_1.clone()),
        Command::FusionP(ncd_z1.clone()),
        Command::FusionS(ncd_z_1.clone()),
        // step
        Command::FusionP(ncd_z1.clone()),
        Command::FusionP(ncd_z1.clone()),
        Command::FusionP(ncd_z1.clone()),
        Command::FusionS(ncd_z_1.clone()),
        Command::FusionS(ncd_z_1.clone()),
        Command::FusionS(ncd_z_1.clone()),
        // step
        Command::Wait,
        Command::FusionP(ncd_x1.clone()),
        Command::FusionS(ncd_x_1.clone()),
        // step
        Command::FusionP(ncd_x1.clone()),
        Command::FusionS(ncd_x_1.clone()),
    ];
    assert_eq!(expect, commands);
}

#[test]
#[ignore]
fn test_generate_region_with_3x3x3() {
    let mut matrix = vec![vec![vec![Voxel::Void; 3]; 3]; 3];
    matrix[1][0][1] = Voxel::Full;
    matrix[1][1][1] = Voxel::Full;
    let model = Model { matrix };

    let initial = Position::new(0, 0, 0);
    let size = Position::new(3, 1, 3);
    let commands = generate_region_commands(&model, initial, size);

    let x1 = Command::SMove(LLCD::new(1, 0, 0));
    let x_1 = Command::SMove(LLCD::new(-1, 0, 0));
    let z1 = Command::SMove(LLCD::new(0, 0, 1));
    let z_1 = Command::SMove(LLCD::new(0, 0, -1));
    let y1 = Command::SMove(LLCD::new(0, 1, 0));
    let fill = Command::Fill(NCD::new(0, -1, 0));

    let expected = vec![
        y1.clone(),
        x1.clone(),
        x1.clone(),
        z1.clone(),
        x_1.clone(),
        fill.clone(),
        x_1.clone(),
        z1.clone(),
        x1.clone(),
        x1.clone(),
        y1.clone(),
        x_1.clone(),
        x_1.clone(),
        z_1.clone(),
        x1.clone(),
        fill.clone(),
        x1.clone(),
        z_1.clone(),
        x_1.clone(),
        x_1.clone(),
        y1.clone(),
    ];

    assert_eq!(expected, commands);
}
