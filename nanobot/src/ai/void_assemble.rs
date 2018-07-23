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

pub struct VoidAssembleAI {}

impl VoidAssembleAI {
    pub fn new(_config: &Config) -> Self {
        VoidAssembleAI {}
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

        let x_size = (bounding.max_x - bounding.min_x + 1) as usize;
        let z_size = (bounding.max_z - bounding.min_z + 1) as usize;
        let xsplit = min(x_size, 20);

        let mut commands = vec![];
        commands.extend(move_straight_x(bounding.min_x));
        commands.extend(move_straight_z(bounding.min_z));

        for m in commands.iter() {
            let v = vec![m.clone()];
        }

        for v in generate_devide_commands((x_size, z_size), (xsplit, 1)).into_iter() {
            commands.extend(v);
        }

        let mut fill_commands_list: Vec<Vec<Command>> = vec![];
        let mut void_commands_list: Vec<Vec<Command>> = vec![];
        let x_width_list = calc_width_list(x_size, xsplit);

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
                commands.extend(step);
                index += 1;
            }
        }
        // each nanobot (x_i, min_z-1)
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

    // fill
    let mut commands = vec![
        Command::SMove(LLCD::new(0, 1, 0)),
        Command::Fill(NCD::new(0, -1, 0)),
    ];
    {
        let path = find_fill_path(target, region, initial);
        let mut cur = Position::new(initial.x, initial.y + 1, initial.z);
        source.matrix[cur.x as usize][(cur.y - 1) as usize][cur.z as usize] = Voxel::Full;

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
                {
                    commands.push(Command::Fill(NCD::new(0, -1, 0)));
                    source.matrix[cur.x as usize][(cur.y - 1) as usize][cur.z as usize] =
                        Voxel::Full;
                }
            }
            for _ in 0..dy.abs() {
                commands.extend(move_straight_y(1));
                cur.y += 1;

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
    (commands, vec![])
}

fn calc_width_list(size: usize, split: usize) -> Vec<i32> {
    let mut list = vec![];
    for i in 0..split {
        let width = (size / split) as i32 + if i < size % split { 1 } else { 0 };
        list.push(width);
    }
    list
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
                println!("{:?}", position);
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
