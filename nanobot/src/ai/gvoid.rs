use ai::config::Config;
use ai::DisassembleAI;
use ai::utils::*;
use common::*;
use model::Model;
use state::State;
use std::cmp::max;
use std::iter::repeat;

pub struct GvoidAI {
    _dry_run_max_resolution: i32,
}

impl GvoidAI {
    pub fn new(config: &Config) -> Self {
        GvoidAI {
            _dry_run_max_resolution: config.dry_run_max_resolution,
        }
    }
}

impl DisassembleAI for GvoidAI {
    fn disassemble(&mut self, model: &Model) -> Vec<Command> {
        let bounding = match calc_bounding_box(model) {
            Some(b) => b,
            None => { return vec![Command::Halt]; }
        };

        let x_size = (bounding.max_x - bounding.min_x + 1) as usize;
        let y_size = (bounding.max_y - bounding.min_y + 1) as usize;
        let z_size = (bounding.max_z - bounding.min_z + 1) as usize;

        let grid_index_x = generate_grid_index(x_size);
        let grid_index_y = generate_grid_index(y_size);
        let grid_index_z = generate_grid_index(z_size);

        let mut state = State::initial_with_model(model);
        let mut commands: Vec<Vec<Command>> = vec![];

        let step = vec![Command::Flip];
        state.update_time_step(&step).expect("failed to move x");
        commands.push(step.clone());

        // go to (min_x, max_y + 1, min_z)
        for m in move_straight_x(bounding.min_x) {
            state.update_time_step(&vec![m.clone()]).expect("failed to move x");
            commands.push(vec![m.clone()]);
        }
        for m in move_straight_y(bounding.max_y + 1) {
            state.update_time_step(&vec![m.clone()]).expect("failed to move y");
            commands.push(vec![m.clone()]);
        }
        for m in move_straight_z(bounding.min_z) {
            state.update_time_step(&vec![m.clone()]).expect("failed to move z");
            commands.push(vec![m.clone()]);
        }

        // x-z plane fission
        for ms in generate_xz_fission_commands(&grid_index_x, &grid_index_z) {
            state.update_time_step(&ms[..]).expect("failed in x-z fission");
            commands.push(ms.clone());
        }

        for z_index in 0..(grid_index_z.len() - 1) {

            // 3d-GVoid
            let z_size = grid_index_z[z_index + 1] - grid_index_z[z_index] + 1;
            commands.extend(generate_3d_gvoid_commands(&grid_index_x, &grid_index_y, z_size, &mut state));
//            for ms in generate_3d_gvoid_commands(&grid_index_x, &grid_index_y, z_size) {
//                state.update_time_step(&ms[..]).expect("failted in 3d-GVoid");
//                commands.push(ms.clone());
//            }

            if z_index < grid_index_z.len() - 2 {
                // go up to the top
                for i in move_straight_y(bounding.max_y + 1) {
                    let mut step = vec![];
                    step.extend(repeat(i.clone()).take(grid_index_x.len() * 2));
                    state.update_time_step(&step).expect("failed to move up to the top");
                    commands.push(step);
                }

                // move next position
                let z_size_1 = grid_index_z[z_index + 1] - grid_index_z[z_index];
                let z_size_2 = grid_index_z[z_index + 2] - grid_index_z[z_index + 1];
                let move_z_1 = move_straight_z(z_size_1 as i32);
                let move_z_2 = move_straight_z(z_size_2 as i32);

                for i in 0..(max(move_z_1.len(), move_z_2.len())) {
                    let mut step = vec![];
                    if i < move_z_1.len() {
                        step.extend(repeat(move_z_1[i].clone()).take(grid_index_x.len()));
                    } else {
                        step.extend(repeat(Command::Wait).take(grid_index_x.len()));
                    }

                    if i < move_z_2.len() {
                        step.extend(repeat(move_z_2[i].clone()).take(grid_index_x.len()));
                    } else {
                        step.extend(repeat(Command::Wait).take(grid_index_x.len()));
                    }
                    commands.push(step);
                }
            }
        }

        // x-z plane fusion
        for ms in generate_xz_fusion_commands(&grid_index_x, &grid_index_z) {
            state.update_time_step(&ms[..]).expect("failed in x-z fission");
            commands.push(ms.clone());
        }

        // move to (0,0,0)
        for m in move_straight_x(-bounding.min_x) {
            state.update_time_step(&vec![m.clone()]).expect("failed to move x");
            commands.push(vec![m.clone()]);
        }
        for m in move_straight_y(-bounding.min_y) {
            state.update_time_step(&vec![m.clone()]).expect("failed to move y");
            commands.push(vec![m.clone()]);
        }
        for m in move_straight_z(-bounding.max_z) {
            state.update_time_step(&vec![m.clone()]).expect("failed to move z");
            commands.push(vec![m.clone()]);
        }

        commands.push(vec![Command::Flip]);
        commands.push(vec![Command::Halt]);

        let mut result = vec![];
        result.extend(commands.iter().flat_map(|v| v.iter()));
        result
    }
}

fn generate_3d_gvoid_commands(grid_index_x: &[usize], grid_index_y: &[usize], z_size: usize, state: &mut State) -> Vec<Vec<Command>> {
    let mut commands = vec![];

    let grid_num_x = grid_index_x.len();

    for i in 0..(grid_index_y.len() - 1) {
        // Void -> Fission
        let mut step = vec![];
        step.extend(repeat(Command::Void(NCD::new(0, -1, 0))).take(grid_num_x * 2));
        state.update_time_step(&step[..]).expect("failed");
        commands.push(step);

        let mut step = vec![];
        step.extend(repeat(Command::Fission(NCD::new(0, -1, 0), 0)).take(grid_num_x * 2));
        state.update_time_step(&step[..]).expect("failed");
        commands.push(step);

        // top: wait, bottom void->smove
        let width_y = (grid_index_y[i + 1] - grid_index_y[i] + 1) as i32;
        for _y in 0..(width_y - 1) {
            let mut step = vec![];
            step.extend(repeat(Command::Wait).take(grid_num_x));
            for _j in 0..grid_num_x {
                step.push(Command::Wait);
                step.extend(repeat(Command::Void(NCD::new(0, -1, 0))).take(2));
            }
            state.update_time_step(&step[..]).expect("failed");
            commands.push(step);

            let mut step = vec![];
            step.extend(repeat(Command::Wait).take(grid_num_x));
            for _j in 0..grid_num_x {
                step.push(Command::Wait);
                step.extend(repeat(Command::SMove(LLCD::new(0, -1, 0))).take(2));
            }
            state.update_time_step(&step[..]).expect("failed");
            commands.push(step);
        }

        // top: move one down, bottom: wait
        let mut step = vec![];
        step.extend(repeat(Command::SMove(LLCD::new(0, -1, 0))).take(grid_num_x));
        for _j in 0..grid_num_x {
            step.push(Command::SMove(LLCD::new(0, -1, 0)));
            step.extend(repeat(Command::Wait).take(2));
        }
        state.update_time_step(&step[..]).expect("failed");
        commands.push(step);


        // GVoid y-z plane
        if z_size > 3 {
            let mut step = vec![];
            // top
            step.extend(repeat(Command::GVoid(NCD::new(0, 0, 1), FCD::new(0, -(width_y - 1), z_size as i32 - 3))).take(grid_num_x));

            for _j in 0..grid_num_x {
                step.push(Command::GVoid(NCD::new(0, 0, -1), FCD::new(0, -(width_y - 1), -(z_size as i32 - 3))));
                step.push(Command::GVoid(NCD::new(0, 0, -1), FCD::new(0, width_y - 1, -(z_size as i32 - 3))));
                step.push(Command::GVoid(NCD::new(0, 0, 1), FCD::new(0, width_y - 1, z_size as i32 - 3)));
            }
            println!("{:?}", step);
            state.update_time_step(&step[..]).expect("failed");
            commands.push(step);
        }

        // 3d-GVoid
        for j in 0..(grid_num_x - 1) {
            let width_x = (grid_index_x[j + 1] - grid_index_x[j] + 1) as i32;
            if width_x > 3 {
                let mut step = vec![];
                // top
                step.extend(repeat(Command::Wait).take(j));
                step.push(Command::GVoid(NCD::new(1, 0, 0), FCD::new(width_x - 3, -(width_y - 1), z_size as i32 - 1)));
                step.push(Command::GVoid(NCD::new(-1, 0, 0), FCD::new(-(width_x - 3), -(width_y - 1), z_size as i32 - 1)));
                step.extend(repeat(Command::Wait).take(grid_num_x - j - 2));

                step.extend(repeat(Command::Wait).take(3 * (grid_num_x - j - 2)));

                step.push(Command::GVoid(NCD::new(-1, 0, -0), FCD::new(-(width_x - 3), -(width_y - 1), -(z_size as i32 - 1))));
                step.push(Command::GVoid(NCD::new(-1, 0, -0), FCD::new(-(width_x - 3), width_y - 1, -(z_size as i32 - 1))));
                step.push(Command::GVoid(NCD::new(-1, 0, 0), FCD::new(-(width_x - 3), width_y - 1, z_size as i32 - 1)));

                step.push(Command::GVoid(NCD::new(1, 0, -0), FCD::new(width_x - 3, -(width_y - 1), -(z_size as i32 - 1))));
                step.push(Command::GVoid(NCD::new(1, 0, -0), FCD::new(width_x - 3, width_y - 1, -(z_size as i32 - 1))));
                step.push(Command::GVoid(NCD::new(1, 0, 0), FCD::new(width_x - 3, width_y - 1, z_size as i32 - 1)));

                step.extend(repeat(Command::Wait).take(3 * j));
                state.update_time_step(&step[..]).expect("failed");
                commands.push(step);
            }
        }

        // top: SMove to bottom, bottom: Wait
        for com in move_straight_y(-(width_y - 2)) {
            let mut step = vec![];
            step.extend(repeat(com.clone()).take(grid_num_x));
            for _j in 0..grid_num_x {
                step.push(com.clone());
                step.extend(repeat(Command::Wait).take(2));
            }
            state.update_time_step(&step[..]).expect("failed");
            commands.push(step);
        }

        // fusion to top
        let mut step = vec![];
        step.extend(repeat(Command::FusionP(NCD::new(0, -1, 0))).take(grid_num_x));
        for _j in 0..grid_num_x {
            step.push(Command::FusionP(NCD::new(0, -1, 0)));
            step.extend(repeat(Command::FusionS(NCD::new(0, 1, 0))).take(2));
        }
        state.update_time_step(&step[..]).expect("failed");
        commands.push(step);
    }
    // move one down
    let mut step = vec![];
    step.extend(repeat(Command::SMove(LLCD::new(0, -1, 0))).take(grid_num_x * 2));
    state.update_time_step(&step[..]).expect("failed");
    commands.push(step);
    commands
}

fn generate_xz_fission_commands(grid_index_x: &[usize], grid_index_z: &[usize]) -> Vec<Vec<Command>> {
    let grid_num_x = grid_index_x.len();

    let mut commands = vec![];

    // fission in x coordinate
    for i in 0..(grid_num_x - 1) {
        let rest = 4 * (grid_num_x - i - 1);
        let mut step = vec![];
        step.extend(repeat(Command::Wait).take(i));
        step.push(Command::Fission(NCD::new(1, 0, 0), rest - 1));
        commands.push(step);

        let width = grid_index_x[i + 1] - grid_index_x[i] + 1;
        for m in move_straight_x(width as i32 - 2) {
            let mut step = vec![];
            step.extend(repeat(Command::Wait).take(i + 1));
            step.push(m.clone());
            commands.push(step);
        }
    }

    // fission in z coordinate
    let mut step: Vec<Command> = vec![];
    step.extend(repeat(Command::Fission(NCD::new(0, 0, 1), 1)).take(grid_num_x));
    commands.push(step);

    let width = grid_index_z[1] - grid_index_z[0] + 1;
    for m in move_straight_z(width as i32 - 2) {
        let mut step: Vec<Command> = vec![];
        step.extend(repeat(Command::Wait).take(grid_num_x));
        step.extend(repeat(m.clone()).take(grid_num_x));
        commands.push(step);
    }
    commands
}

fn generate_xz_fusion_commands(grid_index_x: &[usize], grid_index_z: &[usize]) -> Vec<Vec<Command>> {
    let grid_num_x = grid_index_x.len();
    let grid_num_z = grid_index_z.len();

    let mut commands = vec![];

    // fusion in z coordinate
    let width = grid_index_z[grid_num_z - 1] - grid_index_z[grid_num_z - 2] + 1;

    for m in move_straight_z(width as i32 - 2) {
        let mut step: Vec<Command> = vec![];
        step.extend(repeat(m.clone()).take(grid_num_x));
        step.extend(repeat(Command::Wait).take(grid_num_x));
        commands.push(step);
    }

    let mut step: Vec<Command> = vec![];
    step.extend(repeat(Command::FusionS(NCD::new(0, 0, 1))).take(grid_num_x));
    step.extend(repeat(Command::FusionP(NCD::new(0, 0, -1))).take(grid_num_x));
    commands.push(step);


    // fusion in x coordinate
    for ri in 0..(grid_num_x - 1) {
        let i = grid_num_x - ri - 1;
        let width = grid_index_x[i] - grid_index_x[i - 1] + 1;
        for m in move_straight_x(-(width as i32 - 2)) {
            let mut step = vec![];
            step.push(m.clone());
            step.extend(repeat(Command::Wait).take(i));
            commands.push(step);
        }

        let mut step = vec![];
        step.push(Command::FusionS(NCD::new(-1, 0, 0)));
        step.push(Command::FusionP(NCD::new(1, 0, 0)));
        step.extend(repeat(Command::Wait).take(i - 1));
        commands.push(step);
    }
    commands
}

fn generate_grid_index(width: usize) -> Vec<usize> {
    let mut result = vec![];
    for i in 0..width {
        if i % 30 == 0 {
            result.push(i);
        }
    }
    if (width - 1) % 30 != 0 {
        result.push(width - 1);
    }
    result
}

#[test]
#[ignore]
fn test_generate_3d_gvoid_commands_1_grid() {
    let grid_index_x = vec![0, 3];
    let grid_index_y = vec![0, 3];
    let z_size = 4;

    let mut state = State::initial(4);

    let actual = generate_3d_gvoid_commands(&grid_index_x, &grid_index_y, z_size, &mut state);
    let expected = vec![
        vec![
            Command::Void(NCD::new(0, -1, 0)),
            Command::Void(NCD::new(0, -1, 0)),
            Command::Void(NCD::new(0, -1, 0)),
            Command::Void(NCD::new(0, -1, 0)),
        ],
        vec![
            Command::Fission(NCD::new(0, -1, 0), 0),
            Command::Fission(NCD::new(0, -1, 0), 0),
            Command::Fission(NCD::new(0, -1, 0), 0),
            Command::Fission(NCD::new(0, -1, 0), 0),
        ],
        vec![
            Command::Wait, //group 1
            Command::Wait, //group 2

            //group 2
            Command::Wait, // top
            Command::Void(NCD::new(0, -1, 0)),
            Command::Void(NCD::new(0, -1, 0)),

            //group 1
            Command::Wait, // top
            Command::Void(NCD::new(0, -1, 0)),
            Command::Void(NCD::new(0, -1, 0)),
        ],
        vec![
            Command::Wait,
            Command::Wait,
            Command::Wait, // top
            Command::SMove(LLCD::new(0, -1, 0)),
            Command::SMove(LLCD::new(0, -1, 0)),
            Command::Wait, // top
            Command::SMove(LLCD::new(0, -1, 0)),
            Command::SMove(LLCD::new(0, -1, 0)),
        ],
        vec![
            Command::Wait,
            Command::Wait,
            Command::Wait, // top
            Command::Void(NCD::new(0, -1, 0)),
            Command::Void(NCD::new(0, -1, 0)),
            Command::Wait, // top
            Command::Void(NCD::new(0, -1, 0)),
            Command::Void(NCD::new(0, -1, 0)),
        ],
        vec![
            Command::Wait,
            Command::Wait,
            Command::Wait, // top
            Command::SMove(LLCD::new(0, -1, 0)),
            Command::SMove(LLCD::new(0, -1, 0)),
            Command::Wait, // top
            Command::SMove(LLCD::new(0, -1, 0)),
            Command::SMove(LLCD::new(0, -1, 0)),
        ],
        vec![
            Command::Wait,
            Command::Wait,
            Command::Wait, // top
            Command::Void(NCD::new(0, -1, 0)),
            Command::Void(NCD::new(0, -1, 0)),
            Command::Wait, // top
            Command::Void(NCD::new(0, -1, 0)),
            Command::Void(NCD::new(0, -1, 0)),
        ],
        vec![
            Command::Wait,
            Command::Wait,
            Command::Wait, // top
            Command::SMove(LLCD::new(0, -1, 0)),
            Command::SMove(LLCD::new(0, -1, 0)),
            Command::Wait, // top
            Command::SMove(LLCD::new(0, -1, 0)),
            Command::SMove(LLCD::new(0, -1, 0)),
        ],
        vec![
            Command::SMove(LLCD::new(0, -1, 0)),
            Command::SMove(LLCD::new(0, -1, 0)),
            Command::SMove(LLCD::new(0, -1, 0)),
            Command::Wait,
            Command::Wait,
            Command::SMove(LLCD::new(0, -1, 0)),
            Command::Wait,
            Command::Wait,
        ],
        // y-z plane
        vec![
            Command::GVoid(NCD::new(0, 0, 1), FCD::new(0, -3, 1)),
            Command::GVoid(NCD::new(0, 0, 1), FCD::new(0, -3, 1)),
            Command::GVoid(NCD::new(0, 0, -1), FCD::new(0, -3, -1)),
            Command::GVoid(NCD::new(0, 0, -1), FCD::new(0, 3, -1)),
            Command::GVoid(NCD::new(0, 0, 1), FCD::new(0, 3, 1)),
            Command::GVoid(NCD::new(0, 0, -1), FCD::new(0, -3, -1)),
            Command::GVoid(NCD::new(0, 0, -1), FCD::new(0, 3, -1)),
            Command::GVoid(NCD::new(0, 0, 1), FCD::new(0, 3, 1)),
        ],
        // block
        vec![
            Command::GVoid(NCD::new(1, 0, 0), FCD::new(1, -3, 3)),
            Command::GVoid(NCD::new(-1, 0, 0), FCD::new(-1, -3, 3)),
            Command::GVoid(NCD::new(-1, 0, 0), FCD::new(-1, -3, -3)),
            Command::GVoid(NCD::new(-1, 0, 0), FCD::new(-1, 3, -3)),
            Command::GVoid(NCD::new(-1, 0, 0), FCD::new(-1, 3, 3)),
            Command::GVoid(NCD::new(1, 0, 0), FCD::new(1, -3, -3)),
            Command::GVoid(NCD::new(1, 0, 0), FCD::new(1, 3, -3)),
            Command::GVoid(NCD::new(1, 0, 0), FCD::new(1, 3, 3)),
        ],
        // move
        vec![
            Command::SMove(LLCD::new(0, -2, 0)),
            Command::SMove(LLCD::new(0, -2, 0)),
            Command::SMove(LLCD::new(0, -2, 0)),
            Command::Wait,
            Command::Wait,
            Command::SMove(LLCD::new(0, -2, 0)),
            Command::Wait,
            Command::Wait,
        ],
        // fusion
        vec![
            Command::FusionP(NCD::new(0, -1, 0)),
            Command::FusionP(NCD::new(0, -1, 0)),
            Command::FusionP(NCD::new(0, -1, 0)),
            Command::FusionS(NCD::new(0, 1, 0)),
            Command::FusionS(NCD::new(0, 1, 0)),
            Command::FusionP(NCD::new(0, -1, 0)),
            Command::FusionS(NCD::new(0, 1, 0)),
            Command::FusionS(NCD::new(0, 1, 0)),
        ],
        // smove
        vec![
            Command::SMove(LLCD::new(0, -1, 0)),
            Command::SMove(LLCD::new(0, -1, 0)),
            Command::SMove(LLCD::new(0, -1, 0)),
            Command::SMove(LLCD::new(0, -1, 0)),
        ],
    ];

    assert_eq!(expected, actual);
}

#[test]
fn test_generate_xz_fission_commands() {
    let grid_index_x = vec![0, 2];
    let grid_index_z = vec![0, 2];

    let actual = generate_xz_fission_commands(&grid_index_x, &grid_index_z);

    let expected = vec![
        vec![
            Command::Fission(NCD::new(1, 0, 0), 3)
        ],
        vec![
            Command::Wait,
            Command::SMove(LLCD::new(1, 0, 0))
        ],
        vec![
            Command::Fission(NCD::new(0, 0, 1), 1),
            Command::Fission(NCD::new(0, 0, 1), 1)
        ],
        vec![
            Command::Wait,
            Command::Wait,
            Command::SMove(LLCD::new(0, 0, 1)),
            Command::SMove(LLCD::new(0, 0, 1))
        ],
    ];

    assert_eq!(expected, actual);
}

#[test]
fn test_generate_xz_fusion_commands() {
    let grid_index_x = vec![0, 2];
    let grid_index_z = vec![0, 2];

    let actual = generate_xz_fusion_commands(&grid_index_x, &grid_index_z);

    let expected = vec![
        vec![
            Command::SMove(LLCD::new(0, 0, 1)),
            Command::SMove(LLCD::new(0, 0, 1)),
            Command::Wait,
            Command::Wait,
        ],
        vec![
            Command::FusionS(NCD::new(0, 0, 1)),
            Command::FusionS(NCD::new(0, 0, 1)),
            Command::FusionP(NCD::new(0, 0, -1)),
            Command::FusionP(NCD::new(0, 0, -1)),
        ],
        vec![
            Command::SMove(LLCD::new(-1, 0, 0)),
            Command::Wait,
        ],
        vec![
            Command::FusionS(NCD::new(-1, 0, 0)),
            Command::FusionP(NCD::new(1, 0, 0)),
        ],
    ];

    assert_eq!(expected, actual);
}


#[test]
fn test_generate_grid_index() {
    let actual = generate_grid_index(250);
    let expected = vec![
        0, 30, 60, 90, 120, 150, 180, 210, 240, 249
    ];

    assert_eq!(expected, actual);
}
