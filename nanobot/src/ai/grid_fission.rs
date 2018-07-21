use ai::AI;
use common::*;
use model::*;

use std::iter::repeat;

pub struct GridFissionAI {}

impl GridFissionAI {
    pub fn new() -> Self {
        GridFissionAI {}
    }
}

impl AI for GridFissionAI {
    fn generate(&self, model: &Model) -> Vec<Command> {
        let mut commands = vec![];
        commands.extend(generate_devide_commands((17, 15), (4, 4)));
        commands.extend(generate_concur_commands((17, 15), (4, 4)));
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
    commands.push(Command::SMove(LLCD::new(len % SMOVE_MAX, 0, 0)));
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
    commands.push(Command::SMove(LLCD::new(0, len % SMOVE_MAX, 0)));
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
    commands.push(Command::SMove(LLCD::new(0, 0, len % SMOVE_MAX)));
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

// fn generate_region_commands(model: &Model, region: &Region) -> Vec<Command> {
//     vec![]
// }

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
