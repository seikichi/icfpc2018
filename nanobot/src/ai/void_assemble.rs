use ai::config::Config;
use ai::utils::*;
use ai::AssembleAI;
use common::*;
use model::Model;
use state::State;
use std::cmp::min;
use std::iter::repeat;

use std::cmp::{Ord, Ordering, PartialOrd};
use std::collections::{BinaryHeap, HashSet};
use std::ops::Deref;
// use std::iter::repeat;

pub struct VoidAssembleAI {
    dry_run_max_resolution: i32,
}

impl VoidAssembleAI {
    pub fn new(config: &Config) -> Self {
        VoidAssembleAI {
            dry_run_max_resolution: config.dry_run_max_resolution,
        }
    }
}

impl AssembleAI for VoidAssembleAI {
    fn assemble(&mut self, target: &Model) -> Vec<Command> {
        let bounding = match calc_bounding_box(target) {
            Some(b) => b,
            None => {
                return vec![Command::Halt];
            }
        };
        let r = target.matrix.len();
        let dry_run = r <= self.dry_run_max_resolution as usize;
        let mut state = State::initial(r);
        let mut harmonity_high = false;

        let x_size = (bounding.max_x - bounding.min_x + 1) as usize;
        let xsplit = min(x_size, 40);

        let mut commands = vec![];
        commands.extend(move_straight_x(bounding.min_x));
        commands.extend(move_straight_z(bounding.min_z));

        for m in commands.iter() {
            let v = vec![m.clone()];
            state.update_time_step(&v[..]).expect("failed to move");
        }

        let x_width_list = calc_width_list_by_density(
            &target,
            bounding.min_x as usize,
            bounding.max_x as usize,
            xsplit,
        );

        for v in generate_x_devide_commands(&x_width_list).into_iter() {
            state.update_time_step(&v[..]).expect("failed to devide");
            commands.extend(v);
        }

        let mut fill_commands_list: Vec<Vec<Command>> = vec![];
        let mut void_commands_list: Vec<Vec<Command>> = vec![];

        for i in 0..xsplit {
            let mut x = bounding.min_x;
            for j in 0..i {
                x += x_width_list[j];
            }
            let initial = Position::new(x, 0, bounding.min_z);
            let region = Region(
                Position::new(x, 0, bounding.min_z),
                Position::new(x + x_width_list[i] - 1, bounding.max_y, bounding.max_z),
            );
            let (fill, void) = generate_fill_and_void_commands(target, &initial, &region);
            fill_commands_list.push(fill);
            void_commands_list.push(void);
        }
        // fill
        {
            let mut index = 0;
            loop {
                let mut all_wait = true;
                let mut step = vec![];
                for v in fill_commands_list.iter() {
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
                state
                    .update_time_step(&step[..])
                    .expect("failed to ground fill");
                commands.extend(step);
                index += 1;
            }
        }
        // void
        {
            let mut index = 0;
            loop {
                let mut all_wait = true;
                let mut step = vec![];
                for v in void_commands_list.iter() {
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
                if dry_run {
                    let mut cloned = state.clone();
                    if !harmonity_high {
                        match cloned.update_time_step(&step[..]) {
                            Ok(_) => {
                                state = cloned;
                            }
                            Err(_) => {
                                harmonity_high = true;
                                let mut high = vec![Command::Flip];
                                high.extend(
                                    repeat(Command::Wait).take(void_commands_list.len() - 1),
                                );
                                state.update_time_step(&high[..]).unwrap();
                                state.update_time_step(&step[..]).unwrap();
                                commands.extend(high);
                            }
                        }
                    } else {
                        let mut low = vec![Command::Flip];
                        low.extend(repeat(Command::Wait).take(void_commands_list.len() - 1));

                        match cloned.update_time_step(&low[..]) {
                            Ok(_) => match cloned.update_time_step(&step[..]) {
                                Ok(_) => {
                                    harmonity_high = false;
                                    state = cloned;
                                    commands.extend(low);
                                }
                                Err(_) => {
                                    state.update_time_step(&step[..]).unwrap();
                                }
                            },
                            Err(_) => {
                                state.update_time_step(&step[..]).unwrap();
                            }
                        }
                    }
                } else {
                    if !harmonity_high {
                        match state.update_time_step(&step[..]) {
                            Ok(_) => {}
                            Err(_) => {
                                harmonity_high = true;
                                commands.push(Command::Flip);
                                commands.extend(
                                    repeat(Command::Wait).take(void_commands_list.len() - 1),
                                );
                            }
                        }
                    }
                }
                commands.extend(step);
                index += 1;
            }
        }
        if harmonity_high {
            let mut low = vec![Command::Flip];
            low.extend(repeat(Command::Wait).take(fill_commands_list.len() - 1));
            commands.extend(low);
        }
        // each nanobot (x_i, 0, min_z-1)
        commands.extend(
            generate_x_concur_commands(&x_width_list)
                .iter()
                .flat_map(|v| v.iter()),
        );
        // back to origin
        commands.extend(move_straight_x(-bounding.min_x));
        commands.extend(move_straight_z(-(bounding.min_z - 1)));
        commands.push(Command::Halt);
        // finish
        commands
    }
}

fn generate_fill_and_void_commands(
    target: &Model,
    initial: &Position,
    region: &Region,
) -> (Vec<Command>, Vec<Command>) {
    let r = target.matrix.len();
    let source = vec![vec![vec![Voxel::Void; r]; r]; r];
    let mut source = Model { matrix: source };
    let goal = Position::new(region.0.x, 0, region.0.z - 1);
    let mut cur = Position::new(initial.x, initial.y, initial.z);

    // fill
    let mut commands = vec![Command::SMove(LLCD::new(0, 1, 0))];
    {
        cur.y += 1;
        let path = find_fill_path(target, region, initial);
        if target.matrix[cur.x as usize][(cur.y - 1) as usize][cur.z as usize] == Voxel::Full {
            commands.push(Command::Fill(NCD::new(0, -1, 0)));
            source.matrix[cur.x as usize][(cur.y - 1) as usize][cur.z as usize] = Voxel::Full;
        }

        for next in path.into_iter() {
            let dx = next.x - cur.x;
            let dy = next.y + 1 - cur.y;
            let dz = next.z - cur.z;
            for _ in 0..dx.abs() {
                let d = if dx > 0 { 1 } else { -1 };
                commands.extend(move_straight_x(d));
                cur.x += d;

                if source.matrix[cur.x as usize][(cur.y - 1) as usize][cur.z as usize]
                    == Voxel::Void
                    && (cur.y != 1
                        || target.matrix[cur.x as usize][(cur.y - 1) as usize][cur.z as usize]
                            == Voxel::Full)
                {
                    commands.push(Command::Fill(NCD::new(0, -1, 0)));
                    source.matrix[cur.x as usize][(cur.y - 1) as usize][cur.z as usize] =
                        Voxel::Full;
                }
            }
            for _ in 0..dz.abs() {
                let d = if dz > 0 { 1 } else { -1 };
                commands.extend(move_straight_z(d));
                cur.z += d;

                if source.matrix[cur.x as usize][(cur.y - 1) as usize][cur.z as usize]
                    == Voxel::Void
                    && (cur.y != 1
                        || target.matrix[cur.x as usize][(cur.y - 1) as usize][cur.z as usize]
                            == Voxel::Full)
                {
                    commands.push(Command::Fill(NCD::new(0, -1, 0)));
                    source.matrix[cur.x as usize][(cur.y - 1) as usize][cur.z as usize] =
                        Voxel::Full;
                }
            }
            for _ in 0..dy.abs() {
                let d = if dy > 0 { 1 } else { -1 };

                if cur.y == 1
                    && source.matrix[cur.x as usize][(cur.y - 1) as usize][cur.z as usize]
                        == Voxel::Void
                {
                    commands.push(Command::Fill(NCD::new(0, -1, 0)));
                    source.matrix[cur.x as usize][(cur.y - 1) as usize][cur.z as usize] =
                        Voxel::Full;
                }

                commands.extend(move_straight_y(d));
                cur.y += d;

                if source.matrix[cur.x as usize][(cur.y - 1) as usize][cur.z as usize]
                    == Voxel::Void
                {
                    commands.push(Command::Fill(NCD::new(0, -1, 0)));
                    source.matrix[cur.x as usize][(cur.y - 1) as usize][cur.z as usize] =
                        Voxel::Full;
                }
            }
        }
    }
    // void
    let mut void_commands = vec![];
    {
        let mut path = find_void_path(&source, target, region, &cur);
        path.push(goal);

        for next in path.into_iter() {
            let dx = next.x - cur.x;
            let dy = next.y - cur.y;
            let dz = next.z - cur.z;
            for _ in 0..dx.abs() {
                let d = if dx > 0 { 1 } else { -1 };
                // Void if next is Full
                if source.matrix[(cur.x + d) as usize][cur.y as usize][cur.z as usize]
                    == Voxel::Full
                {
                    void_commands.push(Command::Void(NCD::new(d, 0, 0)));
                    source.matrix[(cur.x + d) as usize][cur.y as usize][cur.z as usize] =
                        Voxel::Void
                }
                // SMove
                void_commands.push(Command::SMove(LLCD::new(d, 0, 0)));
                cur.x += d;
                // Fill if should be Full but Void
                if target.matrix[(cur.x - d) as usize][cur.y as usize][cur.z as usize]
                    == Voxel::Full
                {
                    void_commands.push(Command::Fill(NCD::new(-d, 0, 0)));
                    source.matrix[(cur.x - d) as usize][cur.y as usize][cur.z as usize] =
                        Voxel::Full;
                }
            }
            for _ in 0..dy.abs() {
                let d = if dy > 0 { 1 } else { -1 };
                // Void if next is Full
                if source.matrix[cur.x as usize][(cur.y + d) as usize][cur.z as usize]
                    == Voxel::Full
                {
                    void_commands.push(Command::Void(NCD::new(0, d, 0)));
                    source.matrix[cur.x as usize][(cur.y + d) as usize][cur.z as usize] =
                        Voxel::Void
                }
                // SMove
                void_commands.push(Command::SMove(LLCD::new(0, d, 0)));
                cur.y += d;
                // Fill if should be Full but Void
                if target.matrix[cur.x as usize][(cur.y - d) as usize][cur.z as usize]
                    == Voxel::Full
                {
                    void_commands.push(Command::Fill(NCD::new(0, -d, 0)));
                    source.matrix[cur.x as usize][(cur.y - d) as usize][cur.z as usize] =
                        Voxel::Full;
                }
            }
            for _ in 0..dz.abs() {
                let d = if dz > 0 { 1 } else { -1 };
                // Void if next is Full
                if source.matrix[cur.x as usize][cur.y as usize][(cur.z + d) as usize]
                    == Voxel::Full
                {
                    void_commands.push(Command::Void(NCD::new(0, 0, d)));
                    source.matrix[cur.x as usize][cur.y as usize][(cur.z + d) as usize] =
                        Voxel::Void
                }
                // SMove
                void_commands.push(Command::SMove(LLCD::new(0, 0, d)));
                cur.z += d;
                // Fill if should be Full but Void
                if target.matrix[cur.x as usize][cur.y as usize][(cur.z - d) as usize]
                    == Voxel::Full
                {
                    void_commands.push(Command::Fill(NCD::new(0, 0, -d)));
                    source.matrix[cur.x as usize][cur.y as usize][(cur.z - d) as usize] =
                        Voxel::Full;
                }
            }
        }
    }
    (commands, void_commands)
}

fn calc_width_list_by_density(model: &Model, min_x: usize, max_x: usize, split: usize) -> Vec<i32> {
    let mut sum = 0;
    let mut plane_sum = vec![];
    let r = model.matrix.len();
    for x in min_x..=max_x {
        let mut s = 0;
        for y in 0..r {
            for z in 0..r {
                let voxel = &model.matrix[x as usize][y as usize][z as usize];
                if *voxel == Voxel::Void {
                    continue;
                }
                s += 1;
            }
        }
        sum += s;
        plane_sum.push(s);
    }

    let mut v = plane_sum.clone();
    let mut width_list = vec![1; v.len()];

    while width_list.len() > split {
        let mut index = 0;
        let mut min = sum;
        for i in 0..v.len() - 1 {
            if v[i] + v[i + 1] < min {
                min = v[i] + v[i + 1];
                index = i;
            }
        }

        v[index] += v[index + 1];
        width_list[index] += width_list[index + 1];
        v.remove(index + 1);
        width_list.remove(index + 1);
    }

    width_list
}

fn find_fill_path(target: &Model, region: &Region, initial: &Position) -> Vec<Position> {
    let mut path = vec![];
    let mut visited: HashSet<Position> = HashSet::new();
    let mut current = initial.clone();

    loop {
        let next = match find_nearest_full(&current, &visited, target, region) {
            Some(v) => v,
            None => break,
        };
        path.push(next);
        visited.insert(next);
        current = next;
    }
    path
}

fn find_void_path(
    source: &Model,
    target: &Model,
    region: &Region,
    initial: &Position,
) -> Vec<Position> {
    let mut path = vec![];
    let mut visited: HashSet<Position> = HashSet::new();
    let mut current = initial.clone();

    loop {
        let next = match find_nearest_wrong_fill(&current, &visited, source, target, region) {
            Some(v) => v,
            None => break,
        };
        path.push(next);
        visited.insert(next);
        current = next;
    }
    path
}

#[derive(Eq, PartialEq)]
struct QueueItem {
    position: Position,
    dist: i32,
}

impl Ord for QueueItem {
    fn cmp(&self, other: &QueueItem) -> Ordering {
        (-self.y, -self.dist, self.x, self.z).cmp(&(-other.y, -other.dist, other.x, other.z))
    }
}

impl PartialOrd for QueueItem {
    fn partial_cmp(&self, other: &QueueItem) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Deref for QueueItem {
    type Target = Position;
    fn deref(&self) -> &Position {
        &self.position
    }
}

fn find_nearest_full(
    current: &Position,
    visited: &HashSet<Position>,
    target: &Model,
    region: &Region,
) -> Option<Position> {
    let mut region_visited: HashSet<Position> = HashSet::new();
    let mut heap = BinaryHeap::new();
    heap.push(QueueItem {
        position: current.clone(),
        dist: 0,
    });
    let ds = vec![
        NCD::new(1, 0, 0),
        NCD::new(-1, 0, 0),
        NCD::new(0, 1, 0),
        NCD::new(0, 0, 1),
        NCD::new(0, 0, -1),
    ];

    while !heap.is_empty() {
        let next = heap.pop().unwrap();
        let voxel = target.matrix[next.x as usize][next.y as usize][next.z as usize];
        if voxel == Voxel::Full && !visited.contains(&next) {
            return Some(*next);
        }
        if region_visited.contains(&next) {
            continue;
        }
        region_visited.insert(next.clone());

        for d in ds.iter() {
            let position = *next + d;
            if !region.contains(position) {
                continue;
            }
            heap.push(QueueItem {
                position: position,
                dist: (*current - &position).manhattan_length(),
            });
        }
    }
    None
}

#[derive(Eq, PartialEq)]
struct VoidQueueItem {
    position: Position,
    dist: i32,
}

impl Ord for VoidQueueItem {
    fn cmp(&self, other: &VoidQueueItem) -> Ordering {
        (self.y, -self.dist, self.z, self.x).cmp(&(other.y, -other.dist, other.z, other.x))
    }
}

impl PartialOrd for VoidQueueItem {
    fn partial_cmp(&self, other: &VoidQueueItem) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Deref for VoidQueueItem {
    type Target = Position;
    fn deref(&self) -> &Position {
        &self.position
    }
}

fn find_nearest_wrong_fill(
    current: &Position,
    visited: &HashSet<Position>,
    source: &Model,
    target: &Model,
    region: &Region,
) -> Option<Position> {
    let mut region_visited: HashSet<Position> = HashSet::new();
    let mut heap = BinaryHeap::new();
    heap.push(VoidQueueItem {
        position: current.clone(),
        dist: 0,
    });
    let ds = vec![
        NCD::new(1, 0, 0),
        NCD::new(-1, 0, 0),
        NCD::new(0, -1, 0),
        NCD::new(0, 0, 1),
        NCD::new(0, 0, -1),
    ];

    while !heap.is_empty() {
        let next = heap.pop().unwrap();
        let sv = source.matrix[next.x as usize][next.y as usize][next.z as usize];
        let tv = target.matrix[next.x as usize][next.y as usize][next.z as usize];
        if sv == Voxel::Full && tv == Voxel::Void && !visited.contains(&next) {
            return Some(*next);
        }
        if region_visited.contains(&next) {
            continue;
        }
        region_visited.insert(next.clone());

        for d in ds.iter() {
            let position = *next + d;
            if !region.contains(position) {
                continue;
            }
            heap.push(VoidQueueItem {
                position: position,
                dist: (*current - &position).manhattan_length(),
            });
        }
    }
    None
}

#[test]
fn test_fill_path() {
    let mut matrix = vec![vec![vec![Voxel::Void; 3]; 3]; 3];
    // y = 0
    matrix[1][0][1] = Voxel::Full;
    matrix[1][0][2] = Voxel::Full;
    matrix[2][0][2] = Voxel::Full;
    // y = 1
    matrix[2][1][2] = Voxel::Full;
    matrix[0][1][0] = Voxel::Full;
    let model = Model { matrix };

    let initial = Position::new(0, 0, 0);
    let region = Region(initial, Position::new(2, 2, 2));

    let actual = find_fill_path(&model, &region, &initial);
    let expected = vec![
        Position::new(1, 0, 1),
        Position::new(1, 0, 2),
        Position::new(2, 0, 2),
        Position::new(2, 1, 2),
        Position::new(0, 1, 0),
    ];
    assert_eq!(expected, actual);
}

#[test]
fn test_void_path() {
    let mut source = vec![vec![vec![Voxel::Void; 3]; 3]; 3];
    // y = 0
    source[0][0][0] = Voxel::Full; // added
    source[1][0][0] = Voxel::Full; // added
    source[1][0][1] = Voxel::Full;
    source[1][0][2] = Voxel::Full;
    source[2][0][2] = Voxel::Full;
    // y = 1
    source[2][1][2] = Voxel::Full;
    source[1][1][2] = Voxel::Full; // added
    source[0][1][2] = Voxel::Full; // added
    source[0][1][1] = Voxel::Full; // added
    source[0][1][0] = Voxel::Full;
    let source = Model { matrix: source };

    let mut target = vec![vec![vec![Voxel::Void; 3]; 3]; 3];
    // y = 0
    target[1][0][1] = Voxel::Full;
    target[1][0][2] = Voxel::Full;
    target[2][0][2] = Voxel::Full;
    // y = 1
    target[2][1][2] = Voxel::Full;
    target[0][1][0] = Voxel::Full;
    let target = Model { matrix: target };

    let initial = Position::new(2, 2, 2);
    let region = Region(Position::new(0, 0, 0), Position::new(2, 2, 2));

    let actual = find_void_path(&source, &target, &region, &initial);
    let expected = vec![
        Position::new(1, 1, 2),
        Position::new(0, 1, 2),
        Position::new(0, 1, 1),
        Position::new(0, 0, 0),
        Position::new(1, 0, 0),
    ];
    assert_eq!(expected, actual);
}

fn generate_x_devide_commands(width_list: &Vec<i32>) -> Vec<Vec<Command>> {
    let mut commands = vec![];

    let ncd_x1 = NCD::new(1, 0, 0);
    for i in 0..(width_list.len() - 1) {
        let rest = width_list.len() - i - 1;

        let mut step = vec![];
        step.extend(repeat(Command::Wait).take(i));
        step.push(Command::Fission(ncd_x1.clone(), rest - 1));
        commands.push(step);

        let width = width_list[i];
        let x_moves = move_straight_x(width - 1);
        for m in x_moves.into_iter() {
            let mut step = vec![];
            step.extend(repeat(Command::Wait).take(i + 1));
            step.push(m);
            commands.push(step);
        }
    }

    commands
}

fn generate_x_concur_commands(width_list: &Vec<i32>) -> Vec<Vec<Command>> {
    let mut commands = vec![];
    // concur x axis
    let ncd_x1 = NCD::new(1, 0, 0);
    let ncd_x_1 = NCD::new(-1, 0, 0);
    for i in 0..(width_list.len() - 1) {
        let width = width_list[width_list.len() - i - 2];
        let x_moves = move_straight_x(-(width - 1));
        for m in x_moves.into_iter() {
            let mut step = vec![];
            step.extend(repeat(Command::Wait).take(width_list.len() - i - 1));
            step.push(m);
            commands.push(step);
        }
        // fusion
        let mut step = vec![];
        step.extend(repeat(Command::Wait).take(width_list.len() - i - 2));
        step.push(Command::FusionP(ncd_x1.clone()));
        step.push(Command::FusionS(ncd_x_1.clone()));
        commands.push(step);
    }

    commands
}
