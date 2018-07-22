use ai::utils::*;
use ai::DisassembleAI;
use common::*;
use model::*;
use state::State;
use std::cmp::min;
use std::iter::repeat;

pub struct VoidAI {}

impl VoidAI {
    pub fn new() -> Self {
        VoidAI {}
    }
}

impl DisassembleAI for VoidAI {
    fn disassemble(&self, model: &Model) -> Vec<Command> {
        let bounding = match calc_bounding_box(model) {
            Some(b) => b,
            None => {
                return vec![Command::Halt];
            }
        };

        let mut state = State::initial_with_model(model);

        let x_size = (bounding.max_x - bounding.min_x + 1) as usize;
        let z_size = (bounding.max_z - bounding.min_z + 1) as usize;

        // TODO FIX ME;
        let xsplit = min(x_size, 8);
        let zsplit = min(z_size, 5);

        let mut harmonity_high = false;

        let mut commands = vec![];
        commands.extend(move_straight_y(bounding.max_y + 1));
        commands.extend(move_straight_x(bounding.min_x));
        commands.extend(move_straight_z(bounding.min_z));

        for m in commands.iter() {
            let v = vec![m.clone()];
            state.update_time_step(&v[..]).expect("failed to move");
        }

        for v in generate_devide_commands((x_size, z_size), (xsplit, zsplit)).into_iter() {
            state.update_time_step(&v[..]).expect("failed to divide");
            commands.extend(v);
        }

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
            let initial = Position::new(x, bounding.max_y + 1, bounding.min_z);
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

                let initial = Position::new(x, bounding.max_y + 1, z);
                let size = Position::new(x_width, bounding.max_y, z_width);
                commands_list.push(generate_region_commands(model, initial, size));
            }
        }

        let mut index = 0;
        loop {
            let mut all_wait = true;
            let mut step = vec![];
            for v in commands_list.iter() {
                step.push(if index >= v.len() {
                    Command::Wait
                } else {
                    all_wait = false;
                    v[index].clone()
                });
            }
            if all_wait {
                break;
            }

            if !harmonity_high {
                match state.update_time_step(&step[..]) {
                    Ok(_) => {}
                    Err(_) => {
                        harmonity_high = true;
                        commands.push(Command::Flip);
                        commands.extend(repeat(Command::Wait).take(commands_list.len() - 1));
                    }
                }
            }
            commands.extend(step);
            index += 1;
        }

        if harmonity_high {
            commands.push(Command::Flip);
            commands.extend(repeat(Command::Wait).take(commands_list.len() - 1));
        }

        commands.extend(
            generate_concur_commands((x_size, z_size), (xsplit, zsplit))
                .iter()
                .flat_map(|v| v.iter()),
        );
        commands.extend(move_straight_x(-bounding.min_x));
        commands.extend(move_straight_z(-bounding.min_z));
        commands.push(Command::Halt);
        commands
    }
}

fn generate_region_commands(model: &Model, initial: Position, size: Position) -> Vec<Command> {
    let mut commands = vec![];

    let mut x = initial.x;
    let mut z = initial.z;

    let ncd_y_1 = NCD::new(0, -1, 0);
    let llcd_y_1 = LLCD::new(0, -1, 0);

    let mut xdir = LLCD::new(1, 0, 0);
    let mut zdir = LLCD::new(0, 0, 1);

    for i in 0..size.y + 1 {
        let y = initial.y - i;
        for j in 0..size.z {
            for k in 0..size.x {
                if model.matrix[x as usize][(y - 1) as usize][z as usize] == Voxel::Full {
                    commands.push(Command::Void(ncd_y_1.clone()));
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
        commands.push(Command::SMove(llcd_y_1.clone()));
    }

    commands.extend(move_straight_x(initial.x - x));
    commands.extend(move_straight_z(initial.z - z));
    commands
}

#[test]
fn test_generate_region_with_3x3x3() {
    let mut matrix = vec![vec![vec![Voxel::Void; 3]; 3]; 3];
    matrix[1][0][1] = Voxel::Full;
    matrix[1][1][1] = Voxel::Full;
    let model = Model { matrix };

    let initial = Position::new(0, 2, 0);
    let size = Position::new(3, 1, 3);
    let commands = generate_region_commands(&model, initial, size);

    let x1 = Command::SMove(LLCD::new(1, 0, 0));
    let x_1 = Command::SMove(LLCD::new(-1, 0, 0));
    let z1 = Command::SMove(LLCD::new(0, 0, 1));
    let z_1 = Command::SMove(LLCD::new(0, 0, -1));
    let y_1 = Command::SMove(LLCD::new(0, -1, 0));
    let void = Command::Void(NCD::new(0, -1, 0));

    let expected = vec![
        x1.clone(),
        x1.clone(),
        z1.clone(),
        x_1.clone(),
        void.clone(),
        x_1.clone(),
        z1.clone(),
        x1.clone(),
        x1.clone(),
        y_1.clone(),
        x_1.clone(),
        x_1.clone(),
        z_1.clone(),
        x1.clone(),
        void.clone(),
        x1.clone(),
        z_1.clone(),
        x_1.clone(),
        x_1.clone(),
        y_1.clone(),
    ];

    assert_eq!(expected, commands);
}
