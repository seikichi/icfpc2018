extern crate rand;

use self::rand::Rng;
use self::rand::XorShiftRng;
use ai::config::Config;
use ai::AssembleAI;
use common::*;
use model::*;
use state::State;
use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::VecDeque;

struct BotState {
    bot: Nanobot,
    next_commands: VecDeque<Command>,
}
impl BotState {
    pub fn initial() -> Self {
        BotState {
            bot: Nanobot::initial(),
            next_commands: VecDeque::new(),
        }
    }
    pub fn nop(&mut self) {
        assert_eq!(self.next_commands.len(), 0);
        self.next_commands.push_back(Command::Wait);
    }
}

pub struct BfsAI {
    rng: XorShiftRng,
    state: State,
    current: Model,
    target: Model,
    bots: Vec<BotState>,
    volatiles: HashSet<Position>,
    candidates: Vec<Position>,
    trace: Vec<Command>,
}

impl BfsAI {
    #[allow(deprecated)]
    pub fn new(_config: &Config, source: &Model, target: &Model) -> Self {
        let mut volatiles = HashSet::new();
        volatiles.insert(Position::zero());
        // TODO sourceでFullなのをvolatilesに追加する
        BfsAI {
            rng: XorShiftRng::new_unseeded(),
            state: State::initial_with_model(source),
            current: source.clone(),
            target: target.clone(),
            bots: vec![BotState::initial()],
            volatiles: volatiles,
            candidates: vec![],
            trace: vec![],
        }
    }
    fn is_valid_coordinate(&self, p: &Position) -> bool {
        let r = self.target.matrix.len() as i32;
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
    // モデル内でかつ、volatilesしてなければtrue
    fn is_safe_coordinate(&self, p: &Position) -> bool {
        self.is_valid_coordinate(p) && !self.volatiles.contains(p)
    }
    // candidateから1つ選択
    fn select_one_candidate(&mut self, from: &Position) -> Option<Position> {
        let mut target = self.candidates.len();
        let mut best = 1 << 30;
        for (i, c) in self.candidates.iter().enumerate() {
            if self.volatiles.contains(c) {
                continue;
            }
            // TODO candidateの選択をもう少しましにする
            // groundからの距離が近いやつをなるべく優先する
            // 暫定でyが小さいやつを優先させる
            let mut score = ((*from - c).manhattan_length() + 2) / 5 * 100;
            score += c.y * 200;
            score += self.rng.gen_range(0, 130);
            if score < best {
                target = i;
                best = score;
            }
        }
        if target == self.candidates.len() {
            return None;
        }
        let ret = self.candidates[target];
        self.candidates.remove(target);
        Some(ret)
    }
    // posからvolatileしないncdの位置を全部返す
    // ただしfromはvolatileしていても許可
    fn pos_ncd_all(&self, from: &Position, to: &Position) -> Vec<Position> {
        let mut poss = vec![];
        for ncd in all_ncd().iter() {
            let new_c = *to + ncd;
            if new_c != *from && !self.is_safe_coordinate(&new_c) {
                continue;
            }
            poss.push(new_c);
        }
        poss
    }
    // posからSMoveで移動可能な位置とそのCommandを返す
    fn pos_smove_all(&self, pos: &Position) -> Vec<(Position, Command)> {
        let mut ret = vec![];
        for dir in 0..6 {
            let dx = [0, 0, 1, -1, 0, 0];
            let dy = [1, -1, 0, 0, 0, 0];
            let dz = [0, 0, 0, 0, 1, -1];
            for dist in 1..15 + 1 {
                let llcd = LLCD::new(dist * dx[dir], dist * dy[dir], dist * dz[dir]);
                let npos = *pos + &llcd;
                if !self.is_safe_coordinate(&npos) {
                    break;
                }
                ret.push((npos, Command::SMove(llcd)));
            }
        }
        ret
    }
    // // posからLMoveで移動可能な位置とそのCommandを返す
    // fn pos_lmove_all(&self, _pos: &Position) -> Vec<(Position, Command)> {
    //     // TODO
    //     unimplemented!();
    // }
    // SMove・LMoveの系列をbfsで作って移動してfillする
    fn make_target_fill_command(&self, from: &Position, to: &Position) -> Option<Vec<Command>> {
        let mut ret = vec![];
        let tos = self.pos_ncd_all(from, to);
        let (nto, mut commands) = match self.make_move_any_command(from, &tos) {
            None => {
                return None;
            }
            Some(v) => v,
        };
        ret.append(&mut commands);
        let mut commands = self.make_fill_command(&nto, to);
        ret.append(&mut commands);
        Some(ret)
    }
    // tosのいずれかに移動する系列をbfsで作る
    // 戻り値はついた場所とCommandの系列
    fn make_move_any_command(
        &self,
        from: &Position,
        tos: &Vec<Position>,
    ) -> Option<(Position, Vec<Command>)> {
        let tos = tos.iter().map(|p| *p).collect::<HashSet<Position>>();
        let mut que = VecDeque::<Position>::new();
        que.push_back(*from);
        let mut parents = HashMap::<Position, (Position, Command)>::new(); // visit + 経路復元用
        parents.insert(*from, (*from, Command::Wait));
        while let Some(f) = que.pop_front() {
            if tos.contains(&f) {
                // 経路復元してreverseで正順にしてコマンドの系列を返す
                let mut ret = vec![];
                let mut next = f;
                while next != *from {
                    let (prev, command) = parents[&next];
                    ret.push(command);
                    next = prev;
                }
                ret.reverse();
                return Some((f, ret));
            }
            // TODO lmoveも追加する
            let next = self.pos_smove_all(&f);
            for &(t, command) in next.iter() {
                if parents.contains_key(&t) {
                    continue;
                }
                parents.insert(t, (f, command));
                que.push_back(t);
            }
        }
        // どこにもたどり着けなかった場合
        None
    }
    // fromからtoをfillするコマンドを発行
    // fromとtoはncdの距離
    fn make_fill_command(&self, from: &Position, to: &Position) -> Vec<Command> {
        let cd = *to - from;
        vec![Command::Fill(NCD::new(cd.x, cd.y, cd.z))]
    }
    // 残り1体の時に(0, 0, 0)に戻ってHaltする
    fn make_return_command(&self) -> Option<Vec<Command>> {
        assert_eq!(self.bots.len(), 1);
        let from = self.bots[0].bot.pos;
        match self.make_move_any_command(&from, &vec![Position::zero()]) {
            None => None,
            Some((_, mut commands)) => {
                commands.push(Command::Halt);
                Some(commands)
            }
        }
    }
    fn simulate_move(&self, from: &Position, command: &Command) -> Position {
        match command {
            Command::SMove(llcd) => *from + llcd,
            Command::LMove(slcd1, slcd2) => *from + slcd1 + slcd2,
            _ => *from,
        }
    }
    fn set_volatiles(&mut self, from: &Position, commands: &VecDeque<Command>) {
        let mut pos = *from;
        for command in commands.iter() {
            self.set_volatile(&pos, command);
            pos = self.simulate_move(&pos, command);
        }
    }
    // fromからcommandを実行した時に新しくvolatileになる位置を設定する
    fn set_volatile(&mut self, from: &Position, command: &Command) {
        let ps = match command {
            Command::Halt => vec![],
            Command::Wait => vec![],
            Command::Flip => vec![],
            Command::SMove(llcd) => {
                let mut ret = vec![];
                let to = *from + llcd;
                for p in Region(*from, to).iter() {
                    if p == *from {
                        continue;
                    }
                    ret.push(p);
                }
                ret
            }
            Command::LMove(_slcd1, _slcd2) => {
                unimplemented!();
            }
            Command::Fission(ncd, _) => vec![*from + ncd],
            Command::Fill(ncd) => vec![*from + ncd],
            Command::Void(_) => vec![], // Fullのvoxelはもともとvolatile扱いにするので
            Command::FusionP(_) => vec![],
            Command::FusionS(_) => vec![],
            Command::GFill(_, _) => {
                unimplemented!();
            }
            Command::GVoid(_, _) => vec![],
        };
        for p in ps.iter() {
            assert!(!self.volatiles.contains(p));
            self.volatiles.insert(*p);
        }
    }
    // fromからCommandを実行した時にvolatileが解除される位置を設定する
    fn unset_volatile(&mut self, from: &Position, command: &Command) {
        let ps = match command {
            Command::Halt => vec![],
            Command::Wait => vec![],
            Command::Flip => vec![],
            Command::SMove(llcd) => {
                let mut ret = vec![];
                let to = *from + llcd;
                for p in Region(*from, to).iter() {
                    if p == to {
                        continue;
                    }
                    ret.push(p);
                }
                ret
            }
            Command::LMove(_slcd1, _slcd2) => {
                unimplemented!();
            }
            Command::Fission(_, _) => vec![],
            Command::Fill(_) => vec![], // Fullになるので何もしなくてよい
            Command::Void(ncd) => vec![*from + ncd],
            Command::FusionP(_) => vec![],
            Command::FusionS(_) => vec![*from],
            Command::GFill(_, _) => vec![],
            Command::GVoid(_ncd, _fcd) => {
                unimplemented!();
            }
        };
        // println!("{:?} {:?}", from, command);
        // println!("{:?}", self.volatiles);
        // println!("{:?}", ps);
        for p in ps.iter() {
            assert!(self.volatiles.contains(p));
            self.volatiles.remove(p);
        }
    }
    // posの位置ブロックがFillされたときに周りのVoxelでまだcandidateに入ってないのを入れる
    fn update_full_candidate(&mut self, pos: &Position) {
        for next in adjacent(*pos).iter() {
            if !self.is_valid_coordinate(next)
                || self.current.voxel_at(*next) == Voxel::Full
                || self.target.voxel_at(*next) == Voxel::Void
            {
                continue;
            }
            self.candidates.push(*next);
        }
    }
    // bot_indexのnext_commandsの先頭に入っているCommandを実行する
    fn do_command(&mut self, bot_index: usize) {
        let from = self.bots[bot_index].bot.pos;
        // println!("Commands: {:?}", self.bots[bot_index].next_commands);
        let command = self.bots[bot_index].next_commands.pop_front().unwrap();
        // println!("Do Command: {} {:?} {:?}", bot_index, from, command);
        self.unset_volatile(&from, &command);
        match command {
            Command::Halt => {
                // TODO
            }
            Command::Wait => {}
            Command::Flip => {
                unimplemented!();
            }
            Command::SMove(llcd) => {
                self.bots[bot_index].bot.pos = from + &llcd;
            }
            Command::LMove(slcd1, slcd2) => {
                self.bots[bot_index].bot.pos = from + &slcd1 + &slcd2;
            }
            Command::Fission(_, _) => {
                unimplemented!();
            }
            Command::Fill(ncd) => {
                let to = from + &ncd;
                self.current.set_voxel_at(to, Voxel::Full);
                self.update_full_candidate(&to);
            }
            Command::Void(ncd) => {
                let to = from + &ncd;
                self.current.set_voxel_at(to, Voxel::Full);
                // TODO update_void_candidateを実装する
                // TODO 穴ほって移動する場合にどうするか考える
            }
            Command::FusionP(_) => {
                unimplemented!();
            }
            Command::FusionS(_) => {
                // FusionPの方で処理するので何もしなくてよい
            }
            Command::GFill(_, _) => {
                unimplemented!();
            }
            Command::GVoid(_ncd, _fcd) => {
                unimplemented!();
            }
        }
    }
    // 実行するコマンドの列を集めてシミュレーション実行
    fn do_time_step_simulation(&mut self) {
        // simulatorに渡すコマンド列を作る
        let mut commands = vec![];
        for bot in &mut self.bots {
            commands.push((bot.bot.bid, bot.next_commands[0]));
        }
        commands.sort();
        let mut commands = commands.iter().map(|c| c.1).collect::<Vec<_>>();
        // simulatorを更新
        self.state.update_time_step(&commands[..]).unwrap();
        self.trace.append(&mut commands);
        // 自分自身の状態を更新
        for i in 0..self.bots.len() {
            self.do_command(i);
        }
    }
}

impl AssembleAI for BfsAI {
    fn assemble(&mut self, _model: &Model) -> Vec<Command> {
        let r = self.target.matrix.len();
        // 次に置ける候補
        for x in 0..r {
            for z in 0..r {
                let p = Position::new(x as i32, 0, z as i32);
                if self.target.voxel_at(p) == Voxel::Full {
                    self.candidates.push(p);
                }
            }
        }
        // TODO 分散
        // ブロック埋め
        while self.candidates.len() > 0 {
            // 1 time step 実行
            for i in 0..self.bots.len() {
                if self.bots[i].next_commands.len() != 0 {
                    continue;
                }
                let from = self.bots[i].bot.pos;
                let to = self.select_one_candidate(&from);
                if to.is_none() {
                    // TODO
                    // random move
                    unimplemented!();
                    continue;
                }
                match self.make_target_fill_command(&from, &to.unwrap()) {
                    None => {
                        // TODO
                        assert!(false);
                        // TODO candidateを元に戻すか保持する
                        self.bots[i].nop();
                    }
                    Some(commands) => {
                        let commands = commands.into_iter().collect();
                        // println!("{:?}", commands);
                        self.set_volatiles(&from, &commands);
                        self.bots[i].next_commands = commands;
                    }
                }
            }
            self.do_time_step_simulation();
        }
        // TODO 集合
        // (0, 0, 0)に戻る
        {
            // TODO simulation
            let mut commands = self.make_return_command().unwrap();
            self.trace.append(&mut commands);
        }
        self.trace.clone()
    }
}

#[test]
fn select_one_candidate_test() {
    let model = Model::initial(100);
    let config = Config::new();
    {
        let mut bfs_ai = BfsAI::new(&config, &model, &model);
        let c = Position::new(1, 1, 1);
        bfs_ai.candidates.push(c);
        let p = bfs_ai.select_one_candidate(&Position::zero()).unwrap();
        assert_eq!(p, c);
    }
    {
        let mut bfs_ai = BfsAI::new(&config, &model, &model);
        let c = Position::new(1, 1, 1);
        bfs_ai.candidates.push(c);
        bfs_ai.volatiles.insert(c);
        let result = bfs_ai.select_one_candidate(&c);
        assert!(result.is_none());
    }
}

#[test]
fn pos_ncd_all_test() {
    let model = Model::initial(100);
    let config = Config::new();
    let mut bfs_ai = BfsAI::new(&config, &model, &model);
    {
        let poss = bfs_ai.pos_ncd_all(&Position::zero(), &Position::new(1, 1, 1));
        assert_eq!(poss.len(), 18);
    }
    {
        let poss = bfs_ai.pos_ncd_all(&Position::zero(), &Position::zero());
        assert_eq!(poss.len(), 6);
    }
    {
        let poss = bfs_ai.pos_ncd_all(&Position::zero(), &Position::new(2, 0, 2));
        assert_eq!(poss.len(), 13);
    }
    {
        let poss = bfs_ai.pos_ncd_all(&Position::new(30, 30, 30), &Position::new(1, 0, 1));
        assert_eq!(poss.len(), 12);
    }
    {
        let poss = bfs_ai.pos_ncd_all(&Position::zero(), &Position::new(1, 0, 1));
        assert_eq!(poss.len(), 13);
    }
}
#[test]
fn pos_smove_all_test() {
    let model = Model::initial(100);
    let config = Config::new();
    let mut bfs_ai = BfsAI::new(&config, &model, &model);
    {
        let llcds = bfs_ai.pos_smove_all(&Position::new(30, 30, 30));
        assert_eq!(llcds.len(), 90);
    }
    {
        let llcds = bfs_ai.pos_smove_all(&Position::new(1, 1, 1));
        assert_eq!(llcds.len(), 48);
    }
    {
        let llcds = bfs_ai.pos_smove_all(&Position::new(98, 98, 98));
        assert_eq!(llcds.len(), 48);
    }
    {
        bfs_ai.volatiles.insert(Position::new(28, 30, 30));
        let llcds = bfs_ai.pos_smove_all(&Position::new(30, 30, 30));
        assert_eq!(llcds.len(), 76);
    }
}

#[test]
fn make_fill_command_test() {
    let model = Model::initial(3);
    let config = Config::new();
    let bfs_ai = BfsAI::new(&config, &model, &model);
    {
        let command = bfs_ai.make_fill_command(&Position::zero(), &Position::new(1, 0, 1));
        assert_eq!(command.len(), 1);
        assert_eq!(command[0], Command::Fill(NCD::new(1, 0, 1)));
    }
}

#[test]
fn make_move_any_command_test() {
    let model = Model::initial(10);
    let config = Config::new();
    let mut bfs_ai = BfsAI::new(&config, &model, &model);
    {
        let (pos, commands) = bfs_ai
            .make_move_any_command(&Position::zero(), &vec![Position::new(0, 0, 0)])
            .unwrap();
        assert_eq!(pos, Position::zero());
        assert_eq!(commands.len(), 0);
    }
    {
        let (pos, commands) = bfs_ai
            .make_move_any_command(
                &Position::zero(),
                &vec![Position::new(0, 0, 0), Position::new(3, 3, 3)],
            )
            .unwrap();
        assert_eq!(pos, Position::zero());
        assert_eq!(commands.len(), 0);
    }
    {
        let to = Position::new(3, 3, 3);
        let (pos, commands) = bfs_ai
            .make_move_any_command(&Position::zero(), &vec![to])
            .unwrap();
        assert_eq!(pos, to);
        assert_eq!(commands.len(), 3);
    }
    {
        let to = Position::new(3, 3, 3);
        for p in adjacent(to).iter() {
            bfs_ai.volatiles.insert(*p);
        }
        let result = bfs_ai.make_move_any_command(&Position::zero(), &vec![to]);
        assert!(result.is_none());
    }
}

#[test]
fn make_target_fill_command_test() {
    let model = Model::initial(10);
    let config = Config::new();
    let mut bfs_ai = BfsAI::new(&config, &model, &model);
    {
        let commands = bfs_ai
            .make_target_fill_command(&Position::zero(), &Position::new(1, 0, 1))
            .unwrap();
        assert_eq!(commands.len(), 1);
    }
    {
        let commands = bfs_ai
            .make_target_fill_command(&Position::zero(), &Position::new(1, 1, 1))
            .unwrap();
        assert_eq!(commands.len(), 2);
    }
}

#[test]
fn make_setunset_volatiles_test() {
    let model = Model::initial(30);
    let config = Config::new();
    let mut bfs_ai = BfsAI::new(&config, &model, &model);
    {
        let command = Command::SMove(LLCD::new(3, 0, 0));
        bfs_ai.set_volatile(&Position::zero(), &command);
        assert_eq!(bfs_ai.volatiles.len(), 4);
        bfs_ai.unset_volatile(&Position::zero(), &command);
        assert_eq!(bfs_ai.volatiles.len(), 1);
    }
}

#[test]
fn do_command_test() {
    let model = Model::initial(30);
    let config = Config::new();
    {
        let mut bfs_ai = BfsAI::new(&config, &model, &model);
        let mut commands = VecDeque::new();
        commands.push_back(Command::SMove(LLCD::new(7, 0, 0)));
        commands.push_back(Command::SMove(LLCD::new(0, 0, 7)));
        commands.push_back(Command::Fill(NCD::new(1, 0, 1)));
        let from = bfs_ai.bots[0].bot.pos;
        bfs_ai.set_volatiles(&from, &commands);
        assert_eq!(bfs_ai.volatiles.len(), 1 + 7 + 7 + 1);
        bfs_ai.bots[0].next_commands = commands;
        bfs_ai.do_command(0);
        assert_eq!(bfs_ai.bots[0].bot.pos, Position::new(7, 0, 0));
        assert_eq!(bfs_ai.bots[0].next_commands.len(), 2);
        assert_eq!(bfs_ai.volatiles.len(), 1 + 7 + 1);
        bfs_ai.do_command(0);
        assert_eq!(bfs_ai.bots[0].bot.pos, Position::new(7, 0, 7));
        assert_eq!(bfs_ai.bots[0].next_commands.len(), 1);
        assert_eq!(bfs_ai.volatiles.len(), 1 + 1);
        bfs_ai.do_command(0);
        assert_eq!(bfs_ai.bots[0].bot.pos, Position::new(7, 0, 7));
        assert_eq!(bfs_ai.bots[0].next_commands.len(), 0);
        assert_eq!(bfs_ai.current.voxel_at(Position::new(8, 0, 8)), Voxel::Full);
        assert_eq!(bfs_ai.volatiles.len(), 1 + 1);

        // invalid movement
        // let mut commands = VecDeque::new();
        // commands.push_back(Command::SMove(LLCD::new(1, 0, 0)));
        // commands.push_back(Command::Fill(NCD::new(-1, 0, 0)));
        // let from = bfs_ai.bots[0].bot.pos;
        // bfs_ai.set_volatiles(&from, &commands);
        // assert_eq!(bfs_ai.volatiles.len(), 2 + 1 + 1);
        // bfs_ai.bots[0].next_commands = commands;
        // bfs_ai.do_command(0);
        // assert_eq!(bfs_ai.bots[0].bot.pos, Position::new(8, 0, 7));
        // assert_eq!(bfs_ai.bots[0].next_commands.len(), 1);
        // assert_eq!(bfs_ai.volatiles.len(), 2 + 1);
        // bfs_ai.do_command(0);
        // assert_eq!(bfs_ai.bots[0].bot.pos, Position::new(8, 0, 7));
        // assert_eq!(bfs_ai.bots[0].next_commands.len(), 0);
        // assert_eq!(bfs_ai.current.voxel_at(Position::new(7, 0, 7)), Voxel::Full);
        // assert_eq!(bfs_ai.volatiles.len(), 2 + 1);
    }
}