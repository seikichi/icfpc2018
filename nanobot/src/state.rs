#![allow(dead_code)]

use common::*;
use model::Model;
use std::collections::HashMap;
use std::collections::HashSet;
use std::error::*;
use std::fmt;
use std::iter::Extend;
use union_find::*;

#[derive(Clone, Debug)]
pub struct State {
    energy: i64,
    harmonics: Harmonics,
    matrix: Vec<Vec<Vec<Voxel>>>,
    bots: Vec<Nanobot>,

    // grounded かどうかの判定に使う。
    // r*r*r 番目の要素は床を表す仮想の要素。
    connectivity: UnionFind,
    connectivity_is_dirty: bool,
    // dirtyフラグがたっているが、groundedであることは確実であるというフラグ
    must_be_grounded_on_dirty: bool,

    full_voxel_count: i32,
}

impl State {
    // returns inital state
    pub fn initial(r: usize) -> State {
        let bot = Nanobot::initial();
        State {
            energy: 0,
            harmonics: Harmonics::Low,
            matrix: vec![vec![vec![Voxel::Void; r]; r]; r],
            bots: vec![bot],
            connectivity: UnionFind::new(r * r * r + 1),
            connectivity_is_dirty: false,
            full_voxel_count: 0,
            must_be_grounded_on_dirty: true,
        }
    }
    pub fn initial_with_model(model: &Model) -> State {
        let r = model.matrix.len();
        let mut state = State::initial(r);
        state.matrix = model.matrix.clone();
        state.connectivity_is_dirty = true;
        state
    }
    pub fn end_check(&self, model: &Model) -> Result<(), Box<Error>> {
        assert!(self.matrix.len() == model.matrix.len());
        if self.bots.len() != 0 {
            let message = format!("Exist active nanobots");
            return Err(Box::new(SimulationError::new(message)));
        }
        if self.matrix != model.matrix {
            let message = format!("Halted with missing or excess filled coordinates");
            return Err(Box::new(SimulationError::new(message)));
        }
        Ok(())
    }
    pub fn get_energy(&self) -> i64 {
        self.energy
    }
    pub fn get_bot_count(&self) -> usize {
        self.bots.len()
    }
}

#[derive(Debug)]
pub struct SimulationError {
    message: String,
}

impl SimulationError {
    pub fn new(message: String) -> SimulationError {
        SimulationError {
            message: message.to_string(),
        }
    }
}

impl fmt::Display for SimulationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "SimulationError: {}", self.message)
    }
}

impl Error for SimulationError {
    fn cause(&self) -> Option<&Error> {
        None
    }
}

type VolatileCoordinates = HashSet<Position>;

pub struct UpdateOneOutput {
    pub vc: VolatileCoordinates,
    pub added_bots: Vec<Nanobot>,
    pub deleted_bot_bids: Vec<Bid>,
}

impl UpdateOneOutput {
    fn from_vc(vc: VolatileCoordinates) -> UpdateOneOutput {
        UpdateOneOutput {
            vc,
            added_bots: vec![],
            deleted_bot_bids: vec![],
        }
    }

    fn from_single_volatile_coordinate(p: Position) -> UpdateOneOutput {
        UpdateOneOutput::from_vc(single_volatile_coordinate(p))
    }
}

fn single_volatile_coordinate(p: Position) -> VolatileCoordinates {
    let mut vc = VolatileCoordinates::new();
    vc.insert(p);
    vc
}

fn couple_volatile_coordinates(p1: Position, p2: Position) -> VolatileCoordinates {
    let mut vc = VolatileCoordinates::new();
    vc.insert(p1);
    vc.insert(p2);
    vc
}

impl State {
    pub fn update_time_step(&mut self, commands: &[Command]) -> Result<(), Box<Error>> {
        assert_eq!(commands.len(), self.bots.len());
        assert!(self.bots.len() > 0);

        let r = self.matrix.len();

        self.energy += (r * r * r) as i64 * match self.harmonics {
            Harmonics::Low => 3,
            Harmonics::High => 30,
        };
        self.energy += self.bots.len() as i64 * 20;

        let mut vcs = VolatileCoordinates::new();
        let mut added_bots = vec![];
        let mut deleted_bot_bids = HashSet::new();

        for (i, command) in commands.iter().enumerate() {
            let output = self.update_one(i, command)?;

            let vc = output.vc;
            if !vcs.is_disjoint(&vc) {
                let message = format!(
                    "nanobot interfere : command={:?}, naonbot_index={}",
                    command, i
                );
                return Err(Box::new(SimulationError::new(message)));
            }
            vcs.extend(vc);

            added_bots.extend(output.added_bots);
            deleted_bot_bids.extend(output.deleted_bot_bids)
        }

        self.verify_fusion_commands(commands)?;
        self.verify_gvoid_commands(commands)?;
        self.verify_gfill_commands(commands)?;

        self.bots.retain(|bot| !deleted_bot_bids.contains(&bot.bid));
        self.bots.extend(added_bots);
        self.bots.sort();

        if self.harmonics == Harmonics::Low && self.does_floating_voxel_exist() {
            let message = format!("floating full voxel exists when harmonics is low");
            return Err(Box::new(SimulationError::new(message)));
        }

        Ok(())
    }

    fn verify_fusion_commands(&self, commands: &[Command]) -> Result<(), Box<Error>> {
        let mut fusionps = HashMap::<Position, Position>::new();
        for (i, c) in commands.iter().enumerate() {
            if let Command::FusionP(ncd) = c {
                let secondary_c = self.bots[i].pos + ncd;
                fusionps.insert(self.bots[i].pos, secondary_c);
            }
        }

        let mut fusions_cnt = 0;
        for (i, c) in commands.iter().enumerate() {
            if let Command::FusionS(ncd) = c {
                fusions_cnt += 1;
                let primary_c = self.bots[i].pos + ncd;
                match fusionps.get(&primary_c) {
                    Some(&p) if p == self.bots[i].pos => {}
                    Some(_) | None => {
                        let message = format!(
                            "FusionP and FusionS are not corresponding : fusions_naonbot_index={}",
                            i
                        );
                        return Err(Box::new(SimulationError::new(message)));
                    }
                }
            }
        }

        if fusionps.len() != fusions_cnt {
            let message = format!(
                "FusionP count is not equal FusionS count : FusionP count={} FusionS count={}",
                fusionps.len(),
                fusions_cnt
            );
            return Err(Box::new(SimulationError::new(message)));
        }

        Ok(())
    }

    fn verify_gvoid_commands(&self, commands: &[Command]) -> Result<(), Box<Error>> {
        let mut groups = HashMap::new();

        for (i, command) in commands.iter().enumerate() {
            if let Command::GVoid(ncd, fcd) = command {
                let c = self.bots[i].pos;
                let region = Region(c + ncd, c + ncd + fcd).canonical();

                //println!("c={}, region={:?}", c, region);

                let positions = groups.entry(region).or_insert_with(|| HashSet::new());
                if !positions.insert(c + ncd) {
                    let message = format!("duplicate vertex in GVoid: {}", c + ncd);
                    return Err(Box::new(SimulationError::new(message)));
                }
            }
        }

        for (region, group) in groups.iter() {
            if group.len() != (1 << region.dimension()) {
                let message = format!(
                    "lack of members to GVoid: len={}, dim={}",
                    group.len(),
                    region.dimension()
                );
                return Err(Box::new(SimulationError::new(message)));
            }
        }

        Ok(())
    }

    fn verify_gfill_commands(&self, commands: &[Command]) -> Result<(), Box<Error>> {
        let mut groups = HashMap::new();

        for (i, command) in commands.iter().enumerate() {
            if let Command::GFill(ncd, fcd) = command {
                let c = self.bots[i].pos;
                let region = Region(c + ncd, c + ncd + fcd).canonical();

                //println!("c={}, region={:?}", c, region);

                let positions = groups.entry(region).or_insert_with(|| HashSet::new());
                if !positions.insert(c + ncd) {
                    let message = format!("duplicate vertex in GFill: {}", c + ncd);
                    return Err(Box::new(SimulationError::new(message)));
                }
            }
        }

        for (region, group) in groups.iter() {
            if group.len() != (1 << region.dimension()) {
                let message = format!(
                    "lack of members to GFill: len={}, dim={}",
                    group.len(),
                    region.dimension()
                );
                return Err(Box::new(SimulationError::new(message)));
            }
        }

        Ok(())
    }

    fn does_floating_voxel_exist(&mut self) -> bool {
        if self.connectivity_is_dirty {
            if self.must_be_grounded_on_dirty {
                return false;
            }

            self.recalculate_connectivity()
        }

        self.does_floating_voxel_exist_with_cache()
    }

    fn does_floating_voxel_exist_with_cache(&mut self) -> bool {
        let r = self.matrix.len();
        self.connectivity.size(r * r * r) - 1 != self.full_voxel_count as usize
    }

    fn recalculate_connectivity(&mut self) {
        let r = self.matrix.len();

        self.connectivity = UnionFind::new(r * r * r + 1);
        self.full_voxel_count = 0;

        for (x, vx) in self.matrix.iter().enumerate() {
            for (y, vy) in vx.iter().enumerate() {
                for (z, &voxel) in vy.iter().enumerate() {
                    if voxel == Voxel::Full {
                        let p = Position::new(x as i32, y as i32, z as i32);
                        if y == 0 {
                            self.connectivity.union_set(p.index(r), r * r * r);
                        }
                        for pp in adjacent(p) {
                            if self.is_valid_coordinate(&pp) && self.voxel_at(pp) == Voxel::Full {
                                self.connectivity.union_set(p.index(r), pp.index(r));
                            }
                        }
                        self.full_voxel_count += 1;
                    }
                }
            }
        }

        self.connectivity_is_dirty = false;
        self.must_be_grounded_on_dirty = !self.does_floating_voxel_exist_with_cache();
    }

    pub fn update_one(
        &mut self,
        nanobot_index: usize,
        command: &Command,
    ) -> Result<UpdateOneOutput, Box<Error>> {
        let c = self.bots[nanobot_index].pos;

        match command {
            Command::Halt => {
                if c != Position::new(0, 0, 0) {
                    let message = format!(
                        "nanobot position is not origin: command=Halt, naonbot_index={}, c={}",
                        nanobot_index, c
                    );
                    return Err(Box::new(SimulationError::new(message)));
                }
                if self.bots.len() != 1 {
                    let message = format!(
                        "the number of nanobots is not 1: command=Halt, n_nanobots={}",
                        self.bots.len()
                    );
                    return Err(Box::new(SimulationError::new(message)));
                }
                if self.harmonics != Harmonics::Low {
                    let message = format!("harmonics is not Low: command=Halt");
                    return Err(Box::new(SimulationError::new(message)));
                }
                self.bots.pop();

                Ok(UpdateOneOutput::from_single_volatile_coordinate(c))
            }

            Command::Wait => Ok(UpdateOneOutput::from_single_volatile_coordinate(c)),

            Command::Flip => {
                self.harmonics = match self.harmonics {
                    Harmonics::Low => Harmonics::High,
                    Harmonics::High => Harmonics::Low,
                };
                Ok(UpdateOneOutput::from_single_volatile_coordinate(c))
            }

            Command::SMove(llcd) => {
                let vc = self.move_straight(llcd, nanobot_index, command)?;
                Ok(UpdateOneOutput::from_vc(vc))
            }

            Command::LMove(slcd1, slcd2) => {
                let mut vc1 = self.move_straight(slcd1, nanobot_index, command)?;
                let vc2 = self.move_straight(slcd2, nanobot_index, command)?;
                self.energy += 4;
                vc1.extend(&vc2);

                Ok(UpdateOneOutput::from_vc(vc1))
            }

            Command::Fill(ncd) => {
                let new_c = c + ncd;

                if !self.is_valid_coordinate(&new_c) {
                    let message = format!("nanobot is out of matrix: command=Fill, c={}", new_c);
                    return Err(Box::new(SimulationError::new(message)));
                }

                self.fill_voxel(new_c);

                let vc = couple_volatile_coordinates(c, new_c);
                Ok(UpdateOneOutput::from_vc(vc))
            }

            Command::Fission(ncd, m) => {
                let new_c = c + ncd;
                if !self.is_valid_coordinate(&new_c) {
                    let message = format!("nanobot is out of matrix: command=Fission, c={}", new_c);
                    return Err(Box::new(SimulationError::new(message)));
                }

                let mut bot = &mut self.bots[nanobot_index];
                if *m >= bot.seeds.len() {
                    let message = format!(
                        "too large m: command=Fission, nanobot_index={}, m={}, len={}",
                        nanobot_index,
                        m,
                        bot.seeds.len()
                    );
                    return Err(Box::new(SimulationError::new(message)));
                }

                let new_bot = bot.fission(ncd, *m);
                self.energy += 24;

                Ok(UpdateOneOutput {
                    vc: couple_volatile_coordinates(c, new_c),
                    added_bots: vec![new_bot],
                    deleted_bot_bids: vec![],
                })
            }

            Command::Void(ncd) => {
                let new_c = c + ncd;

                if !self.is_valid_coordinate(&new_c) {
                    let message = format!("nanobot is out of matrix: command=Void, c={}", new_c);
                    return Err(Box::new(SimulationError::new(message)));
                }

                self.erase_voxel(new_c);

                let vc = couple_volatile_coordinates(c, new_c);
                Ok(UpdateOneOutput::from_vc(vc))
            }

            Command::FusionP(ncd) => {
                let secondary_c = c + ncd;
                let secondary_bot_index =
                    self.find_bot_by_coordinate(secondary_c).ok_or_else(|| {
                        let message = format!(
                            "failed to find nanobot at the location: command={:?}, c={}",
                            command, secondary_c
                        );
                        return Box::new(SimulationError::new(message));
                    })?;
                let mut secondary_bot = self.bots[secondary_bot_index].clone();

                let bot = &mut self.bots[nanobot_index];
                bot.fusion(&mut secondary_bot);
                self.energy -= 24;

                Ok(UpdateOneOutput {
                    vc: couple_volatile_coordinates(c, secondary_c),
                    added_bots: vec![],
                    deleted_bot_bids: vec![secondary_bot.bid],
                })
            }

            Command::FusionS(_) => {
                // do nothing
                Ok(UpdateOneOutput::from_vc(VolatileCoordinates::new()))
            }

            Command::GVoid(ncd, fcd) => {
                let region = Region(c + ncd, c + ncd + fcd);
                if !self.is_valid_coordinate(&region.0) || !self.is_valid_coordinate(&region.1) {
                    let message = format!(
                        "nanobot is out of matrix: command=GVoid, c={}, ncd={:?}, fcd={:?}",
                        c, ncd, fcd
                    );
                    return Err(Box::new(SimulationError::new(message)));
                }
                if region.contains(c) {
                    let message = format!(
                        "nanobot is in the region: command=GVoid, c={}, ncd={:?}, fcd={:?}",
                        c, ncd, fcd
                    );
                    return Err(Box::new(SimulationError::new(message)));
                }

                if region != region.canonical() {
                    // canonical な region を持つ bot が代表してコマンドを実行するので
                    // それ以外の GVoid はエラーチェックのみ
                    return Ok(UpdateOneOutput::from_single_volatile_coordinate(c));
                }

                self.must_be_grounded_on_dirty = false;

                for p in region.iter() {
                    match self.voxel_at(p) {
                        Voxel::Full => {
                            self.set_voxel_at(p, Voxel::Void);
                            self.energy -= 12;
                            self.full_voxel_count -= 1;
                            self.connectivity_is_dirty = true;
                        }
                        Voxel::Void => {
                            self.energy += 3;
                        }
                    }
                }

                let mut vc = VolatileCoordinates::new();
                vc.insert(c);
                vc.extend(region.iter());

                Ok(UpdateOneOutput::from_vc(vc))
            }

            Command::GFill(ncd, fcd) => {
                let region = Region(c + ncd, c + ncd + fcd);
                if !self.is_valid_coordinate(&region.0) || !self.is_valid_coordinate(&region.1) {
                    let message = format!(
                        "nanobot is out of matrix: command=GFill, c={}, ncd={:?}, fcd={:?}",
                        c, ncd, fcd
                    );
                    return Err(Box::new(SimulationError::new(message)));
                }
                if region.contains(c) {
                    let message = format!(
                        "nanobot is in the region: command=GFill, c={}, ncd={:?}, fcd={:?}",
                        c, ncd, fcd
                    );
                    return Err(Box::new(SimulationError::new(message)));
                }

                if region != region.canonical() {
                    // canonical な region を持つ bot が代表してコマンドを実行するので
                    // それ以外の GFill はエラーチェックのみ
                    return Ok(UpdateOneOutput::from_single_volatile_coordinate(c));
                }

                for p in region.iter() {
                    self.fill_voxel(p);
                }

                let mut vc = VolatileCoordinates::new();
                vc.insert(c);
                vc.extend(region.iter());

                Ok(UpdateOneOutput::from_vc(vc))
            }
        }
    }

    fn voxel_at(&self, p: Position) -> Voxel {
        self.matrix[p.x as usize][p.y as usize][p.z as usize]
    }

    fn set_voxel_at(&mut self, p: Position, v: Voxel) {
        self.matrix[p.x as usize][p.y as usize][p.z as usize] = v
    }

    fn find_bot_by_coordinate(&self, p: Position) -> Option<usize> {
        for (i, bot) in self.bots.iter().enumerate() {
            if bot.pos == p {
                return Some(i);
            }
        }
        None
    }

    fn move_straight(
        &mut self,
        diff: &CD,
        nanobot_index: usize,
        command: &Command,
    ) -> Result<VolatileCoordinates, Box<Error>> {
        let c = self.bots[nanobot_index].pos;
        let new_c = c + diff;
        if !self.is_valid_coordinate(&new_c) {
            let message = format!(
                "nanobot is out of matrix: command={:?}, c={}",
                command, new_c
            );
            return Err(Box::new(SimulationError::new(message)));
        }
        for p in Region(c, new_c).iter() {
            if self.voxel_at(p) == Voxel::Full {
                let message = format!("nanobot hits full voxel : command={:?}, c={}", command, p);
                return Err(Box::new(SimulationError::new(message)));
            }
        }

        self.bots[nanobot_index].pos = new_c;
        self.energy += 2 * diff.manhattan_length() as i64;
        Ok(Region(c, new_c).iter().collect())
    }

    fn fill_voxel(&mut self, c: Position) {
        let r = self.matrix.len();
        self.must_be_grounded_on_dirty = false;

        match self.voxel_at(c) {
            Voxel::Void => {
                self.set_voxel_at(c, Voxel::Full);
                self.energy += 12;

                for p in adjacent(c) {
                    if self.is_valid_coordinate(&p) && self.voxel_at(p) == Voxel::Full {
                        self.connectivity.union_set(c.index(r), p.index(r));
                    }
                }

                if c.y == 0 {
                    self.connectivity.union_set(c.index(r), r * r * r);
                }
                self.full_voxel_count += 1;
            }
            Voxel::Full => self.energy += 6,
        }
    }

    fn erase_voxel(&mut self, c: Position) {
        match self.voxel_at(c) {
            Voxel::Full => {
                self.set_voxel_at(c, Voxel::Void);
                self.energy -= 12;
                self.full_voxel_count -= 1;

                self.connectivity_is_dirty = true;
                if !self.can_omit_connectivity_recalculation(c) {
                    self.must_be_grounded_on_dirty = false;
                }
            }
            Voxel::Void => {
                self.energy += 3;
            }
        }
    }

    fn can_omit_connectivity_recalculation(&mut self, c: Position) -> bool {
        let r = self.matrix.len() as i32;

        if !self.must_be_grounded_on_dirty {
            return false;
        }

        if c.y == 0 {
            self.voxel_at(c + &NCD::new(0, 1, 0)) == Voxel::Void
        } else {
            (c.y == r - 1 || self.voxel_at(c + &NCD::new(0, 1, 0)) == Voxel::Void) &&
                self.voxel_at(c + &NCD::new(0, -1, 0)) == Voxel::Full &&
                (c.x == 0 ||
                    self.voxel_at(c + &NCD::new(-1, 0, 0)) == Voxel::Void ||
                    self.voxel_at(c + &NCD::new(-1, -1, 0)) == Voxel::Full) &&
                (c.z == 0 ||
                    self.voxel_at(c + &NCD::new(0, 0, -1)) == Voxel::Void ||
                    self.voxel_at(c + &NCD::new(0, -1, -1)) == Voxel::Full) &&
                (c.x == r - 1 ||
                    self.voxel_at(c + &NCD::new(1, 0, 0)) == Voxel::Void ||
                    self.voxel_at(c + &NCD::new(1, -1, 0)) == Voxel::Full) &&
                (c.z == r - 1 ||
                    self.voxel_at(c + &NCD::new(0, 0, 1)) == Voxel::Void ||
                    self.voxel_at(c + &NCD::new(0, -1, 1)) == Voxel::Full)
        }
    }

    fn is_valid_coordinate(&self, p: &Position) -> bool {
        let r = self.matrix.len() as i32;
        if p.x < 0 || p.x >= r {
            return false;
        }
        if p.y < 0 || p.y >= r {
            return false;
        }
        if p.z < 0 || p.z >= r {
            return false;
        }
        true
    }

    fn is_grounded(&mut self, p: &Position) -> bool {
        assert!(!self.connectivity_is_dirty);

        let r = self.matrix.len();
        self.connectivity.find_set(p.index(r), r * r * r)
    }
}

#[test]
fn test_halt_command() {
    {
        let mut state = State::initial(3);
        let vc = state.update_one(0, &Command::Halt).unwrap().vc;
        assert_eq!(state.bots.len(), 0);
        assert_eq!(vc, single_volatile_coordinate(Position::zero()));
    }

    {
        let mut state = State::initial(3);
        state
            .update_one(0, &Command::SMove(LLCD::new(1, 0, 0)))
            .unwrap();
        let r = state.update_one(0, &Command::Halt);
        assert!(r.is_err());
    }

    {
        let mut state = State::initial(3);
        state.update_one(0, &Command::Flip).unwrap();
        let r = state.update_one(0, &Command::Halt);
        assert!(r.is_err());
    }

    {
        let mut state = State::initial(3);
        state
            .update_time_step(&vec![Command::Fission(NCD::new(1, 0, 0), 0)])
            .unwrap();

        let r = state.update_one(0, &Command::Halt);
        assert!(r.is_err());
    }
}

#[test]
fn test_flip_command() {
    {
        let mut state = State::initial(3);
        let vc = state.update_one(0, &Command::Flip).unwrap().vc;
        assert!(state.harmonics == Harmonics::High);
        assert_eq!(vc, single_volatile_coordinate(Position::zero()));
        state.update_one(0, &Command::Flip).unwrap();
        assert!(state.harmonics == Harmonics::Low);
    }
}

#[test]
fn test_smove_command() {
    {
        let mut state = State::initial(3);
        state
            .update_one(0, &Command::SMove(LLCD::new(1, 0, 0)))
            .unwrap();
        assert_eq!(state.bots[0].pos, Position::new(1, 0, 0));
        assert_eq!(state.energy, 2);
        let vc = state
            .update_one(0, &Command::SMove(LLCD::new(0, 2, 0)))
            .unwrap()
            .vc;
        assert_eq!(state.bots[0].pos, Position::new(1, 2, 0));
        assert_eq!(state.energy, 6);
        assert_eq!(
            vc,
            Region(Position::new(1, 0, 0), Position::new(1, 2, 0))
                .iter()
                .collect()
        );
        state
            .update_one(0, &Command::SMove(LLCD::new(0, 0, 1)))
            .unwrap();
        assert_eq!(state.bots[0].pos, Position::new(1, 2, 1));
        assert_eq!(state.energy, 8);
    }
    {
        let mut state = State::initial(3);
        let r = state.update_one(0, &Command::SMove(LLCD::new(0, 0, -1)));
        assert!(r.is_err());
    }
    {
        let mut state = State::initial(3);
        let r = state.update_one(0, &Command::SMove(LLCD::new(3, 0, 0)));
        assert!(r.is_err());
    }
    {
        let mut state = State::initial(3);
        state
            .update_one(0, &Command::Fill(NCD::new(1, 0, 0)))
            .unwrap();
        let r = state.update_one(0, &Command::SMove(LLCD::new(1, 0, 0)));
        assert!(r.is_err());
    }
}

#[test]
fn test_lmove_command() {
    {
        let mut state = State::initial(3);
        let vc = state
            .update_one(0, &Command::LMove(SLCD::new(1, 0, 0), SLCD::new(0, 1, 0)))
            .unwrap()
            .vc;
        let mut expected_vc = VolatileCoordinates::new();
        expected_vc.insert(Position::new(0, 0, 0));
        expected_vc.insert(Position::new(1, 0, 0));
        expected_vc.insert(Position::new(1, 1, 0));
        assert_eq!(state.bots[0].pos, Position::new(1, 1, 0));
        assert_eq!(state.energy, 8);
        assert_eq!(vc, expected_vc);
        state
            .update_one(0, &Command::LMove(SLCD::new(0, 0, 1), SLCD::new(0, 0, -1)))
            .unwrap();
        assert_eq!(state.bots[0].pos, Position::new(1, 1, 0));
        assert_eq!(state.energy, 16);
    }
    {
        let mut state = State::initial(3);
        let r = state.update_one(0, &Command::LMove(SLCD::new(0, 0, 4), SLCD::new(0, 0, 1)));
        assert!(r.is_err());
    }
    {
        let mut state = State::initial(3);
        let r = state.update_one(0, &Command::LMove(SLCD::new(0, 0, 1), SLCD::new(0, -3, 0)));
        assert!(r.is_err());
    }
}

#[test]
fn test_fill_command() {
    {
        let mut state = State::initial(3);
        assert_eq!(state.matrix[1][0][0], Voxel::Void);
        let vc = state
            .update_one(0, &Command::Fill(NCD::new(1, 0, 0)))
            .unwrap()
            .vc;
        assert_eq!(state.matrix[1][0][0], Voxel::Full);
        assert_eq!(state.energy, 12);
        assert_eq!(
            vc,
            couple_volatile_coordinates(Position::new(0, 0, 0), Position::new(1, 0, 0))
        );
        assert!(state.is_grounded(&Position::new(1, 0, 0)));

        state
            .update_one(0, &Command::Fill(NCD::new(1, 0, 0)))
            .unwrap();
        assert_eq!(state.energy, 18);

        state
            .update_one(0, &Command::Fill(NCD::new(1, 1, 0)))
            .unwrap();
        assert!(state.is_grounded(&Position::new(1, 1, 0)));
    }

    {
        let mut state = State::initial(3);
        let r = state.update_one(0, &Command::Fill(NCD::new(-1, 0, 0)));
        assert!(r.is_err());
    }

    {
        let mut state = State::initial(3);
        state
            .update_one(0, &Command::SMove(LLCD::new(2, 0, 0)))
            .unwrap();
        let r = state.update_one(0, &Command::Fill(NCD::new(1, 0, 0)));
        assert!(r.is_err());
    }

    {
        let mut state = State::initial(3);
        state
            .update_one(0, &Command::Fill(NCD::new(0, 1, 0)))
            .unwrap();
        assert!(!state.is_grounded(&Position::new(0, 1, 0)));
    }
}

#[test]
fn test_void_command() {
    {
        let mut state = State::initial(3);

        state
            .update_one(0, &Command::Fill(NCD::new(1, 0, 0)))
            .unwrap();
        assert_eq!(state.energy, 12);

        let vc = state
            .update_one(0, &Command::Void(NCD::new(1, 0, 0)))
            .unwrap()
            .vc;
        assert_eq!(state.voxel_at(Position::new(1, 0, 0)), Voxel::Void);
        assert_eq!(state.energy, 0);
        assert!(state.connectivity_is_dirty);
        assert_eq!(
            vc,
            couple_volatile_coordinates(Position::new(0, 0, 0), Position::new(1, 0, 0))
        );
    }

    {
        let mut state = State::initial(3);

        let vc = state
            .update_one(0, &Command::Void(NCD::new(1, 0, 0)))
            .unwrap()
            .vc;
        assert_eq!(state.energy, 3);
        assert_eq!(
            vc,
            couple_volatile_coordinates(Position::new(0, 0, 0), Position::new(1, 0, 0))
        );
    }
}

#[test]
fn test_fission_command() {
    {
        let mut state = State::initial(3);
        let output = state
            .update_one(0, &Command::Fission(NCD::new(1, 0, 0), 1))
            .unwrap();
        let mut expected_vc = VolatileCoordinates::new();
        expected_vc.insert(Position::new(0, 0, 0));
        expected_vc.insert(Position::new(1, 0, 0));
        assert_eq!(state.energy, 24);
        assert_eq!(state.bots.len(), 1);
        assert_eq!(state.bots[0].pos, Position::zero());
        assert_eq!(state.bots[0].bid, Bid(1));
        assert_eq!(
            state.bots[0].seeds,
            (4..41).map(|i| Bid(i)).collect::<Vec<Bid>>()
        );
        assert_eq!(output.vc, expected_vc);
        assert_eq!(output.added_bots.len(), 1);
        assert_eq!(output.added_bots[0].pos, Position::new(1, 0, 0));
        assert_eq!(output.added_bots[0].bid, Bid(2));
        assert_eq!(output.added_bots[0].seeds, vec![Bid(3)]);
    }
    {
        let mut state = State::initial(3);
        let r = state.update_one(0, &Command::Fission(NCD::new(-1, 0, 0), 1));
        assert!(r.is_err());
    }
    {
        let mut state = State::initial(3);
        let r = state.update_one(0, &Command::Fission(NCD::new(1, 0, 0), 39));
        assert!(r.is_err());
    }
    {
        let mut state = State::initial(3);
        let r = state.update_one(0, &Command::Fission(NCD::new(1, 0, 0), 0));
        assert!(r.is_ok());
    }
    {
        let mut state = State::initial(3);
        state.bots[0].seeds = vec![];
        let r = state.update_one(0, &Command::Fission(NCD::new(1, 0, 0), 0));
        assert!(r.is_err());
    }
}

#[test]
fn test_fusion_command() {
    {
        let mut state = State::initial(3);
        state
            .update_time_step(&vec![Command::Fission(NCD::new(1, 0, 0), 1)])
            .unwrap();
        assert_eq!(state.energy, 3 * 3 * 3 * 3 + 20 + 24);

        state
            .update_time_step(&vec![
                Command::FusionP(NCD::new(1, 0, 0)),
                Command::FusionS(NCD::new(-1, 0, 0)),
            ])
            .unwrap();

        assert_eq!(state.bots.len(), 1);
        assert_eq!(state.bots[0].bid, Bid(1));
        assert_eq!(
            state.bots[0].seeds,
            (2..41).map(|i| Bid(i)).collect::<Vec<Bid>>()
        );
        assert_eq!(state.energy, 3 * 3 * 3 * 3 * 2 + 20 + 40);
    }

    {
        let mut state = State::initial(3);
        let r = state.update_time_step(&vec![Command::FusionP(NCD::new(1, 0, 0))]);
        assert!(r.is_err());
    }

    {
        let mut state = State::initial(3);
        state
            .update_time_step(&vec![Command::Fission(NCD::new(1, 0, 0), 1)])
            .unwrap();
        let r = state.update_time_step(&vec![Command::FusionP(NCD::new(1, 0, 0)), Command::Wait]);
        assert!(r.is_err());
    }

    {
        let mut state = State::initial(3);
        state
            .update_time_step(&vec![Command::Fission(NCD::new(1, 0, 0), 1)])
            .unwrap();
        let r = state.update_time_step(&vec![Command::Wait, Command::FusionS(NCD::new(-1, 0, 0))]);
        assert!(r.is_err());
    }

    {
        let mut state = State::initial(3);
        state
            .update_time_step(&vec![Command::Fission(NCD::new(1, 0, 0), 5)])
            .unwrap();
        state
            .update_time_step(&vec![
                Command::Fission(NCD::new(1, 1, 0), 1),
                Command::Fission(NCD::new(-1, 1, 0), 1),
            ])
            .unwrap();
        let r = state.update_time_step(&vec![
            Command::FusionP(NCD::new(1, 0, 0)),
            Command::FusionS(NCD::new(-1, 0, 0)),
            Command::FusionP(NCD::new(1, 0, 0)),
            Command::FusionS(NCD::new(-1, 0, 0)),
        ]);
        assert!(r.is_ok());
    }

    {
        let mut state = State::initial(3);
        state
            .update_time_step(&vec![Command::Fission(NCD::new(1, 0, 0), 5)])
            .unwrap();
        state
            .update_time_step(&vec![
                Command::Fission(NCD::new(1, 1, 0), 1),
                Command::Fission(NCD::new(-1, 1, 0), 1),
            ])
            .unwrap();
        let r = state.update_time_step(&vec![
            Command::FusionP(NCD::new(1, 0, 0)),
            Command::FusionS(NCD::new(-1, 1, 0)),
            Command::FusionP(NCD::new(1, 0, 0)),
            Command::FusionS(NCD::new(-1, -1, 0)),
        ]);
        // println!("{:?}", r);
        assert!(r.is_err());
    }
}

#[test]
fn test_gvoid_commmand() {
    let original_state = {
        let mut state = State::initial(10);
        for y in 0..9 {
            for z in 0..10 {
                for x in 1..10 {
                    state.bots[0].pos = Position::new(x, y + 1, z);
                    let command = Command::Fill(NCD::new(0, -1, 0));
                    state.update_one(0, &command).unwrap();
                }
            }
        }
        state.bots[0].pos = Position::zero();
        state
    };

    {
        let mut state = original_state.clone();
        let prev_energy = state.energy;

        let gvoid = Command::GVoid(NCD::new(1, 0, 0), FCD::new(4, 5, 6));
        let vc = state.update_one(0, &gvoid).unwrap().vc;

        // GVoid で消した範囲が Void になっていることを verify
        let region = Region(Position::new(1, 0, 0), Position::new(5, 5, 6));
        for p in region.iter() {
            assert_eq!(state.voxel_at(p), Voxel::Void);
        }

        // 範囲外の点を代表していくつか verify しておく
        assert_eq!(state.voxel_at(Position::new(6, 1, 1)), Voxel::Full);
        assert_eq!(state.voxel_at(Position::new(1, 6, 1)), Voxel::Full);
        assert_eq!(state.voxel_at(Position::new(1, 1, 7)), Voxel::Full);

        // verify energy
        assert_eq!(state.energy, prev_energy - 12 * (4 + 1) * (5 + 1) * (6 + 1));

        // verify vc
        let mut expected_vc = VolatileCoordinates::new();
        expected_vc.insert(Position::zero());
        expected_vc.extend(region.iter());
        assert_eq!(vc, expected_vc);
    }

    {
        let mut state = original_state.clone();

        let gvoid = Command::GVoid(NCD::new(1, 0, 0), FCD::new(-1, 0, 0));
        let r = state.update_one(0, &gvoid);
        assert!(r.is_err());
    }
}

#[test]
fn test_gfill_commmand() {
    {
        let mut state = State::initial(10);

        let gfill = Command::GFill(NCD::new(1, 0, 0), FCD::new(4, 5, 6));
        let vc = state.update_one(0, &gfill).unwrap().vc;

        // GFill でつくった範囲が Full になっていることを verify
        let region = Region(Position::new(1, 0, 0), Position::new(5, 5, 6));
        for p in region.iter() {
            assert_eq!(state.voxel_at(p), Voxel::Full);
        }

        // 範囲外の点を代表していくつか verify しておく
        assert_eq!(state.voxel_at(Position::new(6, 1, 1)), Voxel::Void);
        assert_eq!(state.voxel_at(Position::new(1, 6, 1)), Voxel::Void);
        assert_eq!(state.voxel_at(Position::new(1, 1, 7)), Voxel::Void);

        // verify energy
        assert_eq!(state.energy, 12 * (4 + 1) * (5 + 1) * (6 + 1));

        // verify vc
        let mut expected_vc = VolatileCoordinates::new();
        expected_vc.insert(Position::zero());
        expected_vc.extend(region.iter());
        assert_eq!(vc, expected_vc);
    }

    {
        let mut state = State::initial(10);

        let gfill = Command::GFill(NCD::new(1, 0, 0), FCD::new(-1, 0, 0));
        let r = state.update_one(0, &gfill);
        assert!(r.is_err());
    }
}

#[test]
fn test_update_time_step() {
    {
        let mut state = State::initial(3);
        let commands = vec![Command::Wait];
        state.update_time_step(&commands).unwrap();
        let mut expected_energy = 3 * 3 * 3 * 3 + 20;
        assert_eq!(state.energy, expected_energy);

        let commands = vec![Command::Flip];
        state.update_time_step(&commands).unwrap();
        expected_energy += 3 * 3 * 3 * 3 + 20;
        assert_eq!(state.energy, expected_energy);

        let commands = vec![Command::Wait];
        state.update_time_step(&commands).unwrap();
        expected_energy += 3 * 3 * 3 * 30 + 20;
        assert_eq!(state.energy, expected_energy);
    }
}

#[test]
fn test_update_time_step_bot_order() {
    {
        let mut state = State::initial(3);
        state
            .update_time_step(&vec![Command::Fission(NCD::new(1, 0, 0), 5)])
            .unwrap();
        state
            .update_time_step(&vec![
                Command::Fission(NCD::new(0, 1, 0), 1),
                Command::Fission(NCD::new(1, 1, 0), 1),
            ])
            .unwrap();

        assert_eq!(
            state.bots.iter().map(|bot| bot.bid).collect::<Vec<_>>(),
            vec![Bid(1), Bid(2), Bid(3), Bid(8)]
        )
    }
}

#[test]
fn test_update_time_step_gvoid() {
    let original_state = {
        let mut state = State::initial(3);
        state
            .update_time_step(&vec![Command::Fission(NCD::new(1, 0, 0), 4)])
            .unwrap();
        state
            .update_time_step(&vec![
                Command::Fission(NCD::new(0, 1, 0), 1),
                Command::Fission(NCD::new(0, 1, 0), 1),
            ])
            .unwrap();

        // for bot in state.bots.iter() {
        //     println!("{:?}: {}", bot.bid, bot.pos);
        // }
        // -> Bid(1): (0, 0, 0)
        //    Bid(2): (1, 0, 0)
        //    Bid(3): (1, 1, 0)
        //    Bid(7): (0, 1, 0)

        state
    };

    {
        // 正常系
        let mut state = original_state.clone();
        state
            .update_time_step(&vec![
                Command::GVoid(NCD::new(0, 0, 1), FCD::new(1, 1, 0)),
                Command::GVoid(NCD::new(0, 0, 1), FCD::new(-1, 1, 0)),
                Command::GVoid(NCD::new(0, 0, 1), FCD::new(-1, -1, 0)),
                Command::GVoid(NCD::new(0, 0, 1), FCD::new(1, -1, 0)),
            ])
            .unwrap();
    }

    {
        // 異常系: 数が足りない
        let mut state = original_state.clone();
        let r = state.update_time_step(&vec![
            Command::GVoid(NCD::new(0, 0, 1), FCD::new(1, 1, 1)),
            Command::GVoid(NCD::new(0, 0, 1), FCD::new(-1, 1, 1)),
            Command::GVoid(NCD::new(0, 0, 1), FCD::new(-1, -1, 1)),
            Command::Wait,
        ]);
        assert!(r.is_err());
    }

    {
        // 異常系: 頂点がかぶっている
        let mut state = original_state.clone();
        let r = state.update_time_step(&vec![
            Command::GVoid(NCD::new(0, 0, 1), FCD::new(1, 1, 1)),
            Command::GVoid(NCD::new(0, 0, 1), FCD::new(-1, 1, 1)),
            Command::GVoid(NCD::new(0, 0, 1), FCD::new(-1, -1, 1)),
            Command::GVoid(NCD::new(0, -1, 1), FCD::new(1, 1, 1)),
        ]);
        assert!(r.is_err());
    }
}

#[test]
fn test_update_time_step_gfill() {
    let original_state = {
        let mut state = State::initial(3);
        state
            .update_time_step(&vec![Command::Fission(NCD::new(1, 0, 0), 4)])
            .unwrap();
        state
            .update_time_step(&vec![
                Command::Fission(NCD::new(0, 1, 0), 1),
                Command::Fission(NCD::new(0, 1, 0), 1),
            ])
            .unwrap();

        // for bot in state.bots.iter() {
        //     println!("{:?}: {}", bot.bid, bot.pos);
        // }
        // -> Bid(1): (0, 0, 0)
        //    Bid(2): (1, 0, 0)
        //    Bid(3): (1, 1, 0)
        //    Bid(7): (0, 1, 0)

        state
    };

    {
        // 正常系
        let mut state = original_state.clone();
        state
            .update_time_step(&vec![
                Command::GFill(NCD::new(0, 0, 1), FCD::new(1, 1, 0)),
                Command::GFill(NCD::new(0, 0, 1), FCD::new(-1, 1, 0)),
                Command::GFill(NCD::new(0, 0, 1), FCD::new(-1, -1, 0)),
                Command::GFill(NCD::new(0, 0, 1), FCD::new(1, -1, 0)),
            ])
            .unwrap();
    }

    {
        // 異常系: 数が足りない
        let mut state = original_state.clone();
        let r = state.update_time_step(&vec![
            Command::GFill(NCD::new(0, 0, 1), FCD::new(1, 1, 1)),
            Command::GFill(NCD::new(0, 0, 1), FCD::new(-1, 1, 1)),
            Command::GFill(NCD::new(0, 0, 1), FCD::new(-1, -1, 1)),
            Command::Wait,
        ]);
        assert!(r.is_err());
    }

    {
        // 異常系: 頂点がかぶっている
        let mut state = original_state.clone();
        let r = state.update_time_step(&vec![
            Command::GFill(NCD::new(0, 0, 1), FCD::new(1, 1, 1)),
            Command::GFill(NCD::new(0, 0, 1), FCD::new(-1, 1, 1)),
            Command::GFill(NCD::new(0, 0, 1), FCD::new(-1, -1, 1)),
            Command::GFill(NCD::new(0, -1, 1), FCD::new(1, 1, 1)),
        ]);
        assert!(r.is_err());
    }
}

#[test]
fn test_interfere_check() {
    {
        let mut state = State::initial(3);
        state
            .update_time_step(&vec![Command::Fission(NCD::new(1, 0, 0), 0)])
            .unwrap();
        let commands = vec![Command::Wait, Command::SMove(LLCD::new(-1, 0, 0))];
        let r = state.update_time_step(&commands);
        assert!(r.is_err());
    }

    {
        // xxx
        // xxx
        // 12x

        // x2x
        // 131
        // 12x
        let mut state = State::initial(3);
        state
            .update_time_step(&vec![Command::Fission(NCD::new(1, 0, 0), 0)])
            .unwrap();
        let commands = vec![
            Command::LMove(SLCD::new(0, 1, 0), SLCD::new(2, 0, 0)),
            Command::SMove(LLCD::new(0, 2, 0)),
        ];
        let r = state.update_time_step(&commands);
        assert!(r.is_err());
    }
}

#[test]
fn test_grounded_check() {
    {
        let mut state = State::initial(3);
        let r = state.update_time_step(&vec![Command::Fill(NCD::new(0, 1, 0))]);
        //println!("{:?}", r);
        assert!(r.is_err());
    }

    {
        let mut state = State::initial(3);
        state.update_time_step(&vec![Command::Flip]).unwrap();
        state
            .update_time_step(&vec![Command::Fill(NCD::new(0, 1, 0))])
            .unwrap();
    }

    {
        let mut state = State::initial(3);
        state
            .update_time_step(&vec![Command::Fill(NCD::new(0, 0, 1))])
            .unwrap();
    }

    {
        let mut state = State::initial(3);
        state
            .update_time_step(&vec![Command::Fill(NCD::new(1, 0, 0))])
            .unwrap();
        let r = state.update_time_step(&vec![Command::Fill(NCD::new(0, 1, 1))]);
        assert!(r.is_err());
    }

    {
        let mut state = State::initial(3);
        state
            .update_time_step(&vec![Command::Fill(NCD::new(1, 0, 0))])
            .unwrap();
        state
            .update_time_step(&vec![Command::Fill(NCD::new(1, 1, 0))])
            .unwrap();
        let r = state.update_time_step(&vec![Command::Void(NCD::new(1, 0, 0))]);
        assert!(r.is_err());
    }

    {
        let mut state = State::initial(3);
        state
            .update_time_step(&vec![Command::Fill(NCD::new(1, 0, 0))])
            .unwrap();
        state
            .update_time_step(&vec![Command::Fill(NCD::new(1, 1, 0))])
            .unwrap();
        let r = state.update_time_step(&vec![Command::Void(NCD::new(1, 0, 0))]);
        assert!(r.is_err());
    }

    {
        let mut state = State::initial(3);
        state
            .update_time_step(&vec![Command::Fill(NCD::new(1, 0, 0))])
            .unwrap();
        state
            .update_time_step(&vec![Command::Fill(NCD::new(1, 1, 0))])
            .unwrap();
        state
            .update_time_step(&vec![Command::Void(NCD::new(1, 1, 0))])
            .unwrap();
    }
}

#[test]
fn end_check_test() {
    {
        let mut state = State::initial(3);
        let model = Model::initial(3);
        state.update_time_step(&vec![Command::Halt]).unwrap();
        let r = state.end_check(&model);
        assert!(r.is_ok());
    }
    {
        let state = State::initial(3);
        let model = Model::initial(3);
        let r = state.end_check(&model);
        assert!(r.is_err());
    }
    {
        let mut state = State::initial(3);
        let mut model = Model::initial(3);
        model.matrix[1][0][1] = Voxel::Full;
        state.update_time_step(&vec![Command::Halt]).unwrap();
        let r = state.end_check(&model);
        assert!(r.is_err());
    }
}
