// use ai::config::Config;
// use ai::utils::*;
// use ai::AssembleAI;
use common::*;
use model::Model;
// use state::State;
// use std::cmp::min;
// use std::iter::repeat;

use std::cmp::{Ord, Ordering, PartialOrd};
use std::collections::{BinaryHeap, HashSet};
use std::ops::Deref;
// use std::iter::repeat;

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

fn find_void_path(
    source: &Model,
    target: &Model,
    region: &Region,
    initial: &Position,
) -> Vec<Position> {
    vec![]
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
    let region = Region(initial, Position::new(0, 2, 0));

    let actual = find_void_path(&source, &target, &region, &initial);
    let expected = vec![
        Position::new(0, 1, 1),
        Position::new(0, 1, 2),
        Position::new(1, 1, 2),
        Position::new(1, 0, 0),
        Position::new(0, 0, 0),
    ];
    assert_eq!(expected, actual);
}
