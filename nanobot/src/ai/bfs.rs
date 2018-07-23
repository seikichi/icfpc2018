extern crate rand;

use self::rand::Rng;
use self::rand::XorShiftRng;
use ai::config::Config;
use ai::AssembleAI;
use common::*;
use model::*;
use state::State;
use std::cmp::min;
use std::collections::BinaryHeap;
use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::VecDeque;

struct BotState {
    bot: Nanobot,
    fusions_bid: Option<Bid>,
    next_commands: VecDeque<Command>,
}
impl BotState {
    pub fn initial() -> Self {
        BotState {
            bot: Nanobot::initial(),
            fusions_bid: None,
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
    visited: HashSet<Position>, // candidatesとして入ったことがあるやつのリスト
    trace: Vec<Command>,
    added_bot_list: Vec<Nanobot>,
    deleted_bot_list: Vec<Bid>,
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
            visited: HashSet::new(),
            trace: vec![],
            added_bot_list: vec![],
            deleted_bot_list: vec![],
        }
    }
    fn is_all_bot_command_done(&self) -> bool {
        self.bots.iter().any(|b| b.next_commands.len() > 0)
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
            score += c.y * 1000;
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
    fn pos_smove_all(&self, pos: &Position, max_dist: i32) -> Vec<(Position, Command)> {
        assert!(max_dist >= 1);
        let max_dist = min(max_dist, 15);
        let mut ret = vec![];
        for dir in 0..6 {
            // yが高いほうが優先
            let dy = [1, 0, 0, 0, 0, -1];
            let dx = [0, 1, -1, 0, 0, 0];
            let dz = [0, 0, 0, 1, -1, 0];
            for dist in 1..max_dist + 1 {
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
    fn make_target_fill_command(&mut self, from: &Position, to: &Position) -> Option<Vec<Command>> {
        let tos = self.pos_ncd_all(from, to);
        if tos.len() == 0 {
            return None;
        }
        let mut ret = vec![];
        assert!(!self.volatiles.contains(to));
        self.volatiles.insert(*to); // Fillする位置を通るとassertに引っかかってしまうのでいったん除外する
        let (nto, mut commands) = match self.make_move_any_command(from, &tos) {
            None => {
                self.volatiles.remove(to);
                return None;
            }
            Some(v) => v,
        };
        self.volatiles.remove(to);
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
        let tos0 = tos[0];
        let tos = tos.iter().map(|p| *p).collect::<HashSet<Position>>();
        let mut que = BinaryHeap::<(i32, Position, i32)>::new();
        que.push((0, *from, 0));
        let mut parents = HashMap::<Position, (Position, Command)>::new(); // visit + 経路復元用
        parents.insert(*from, (*from, Command::Wait));
        let mut max_dist = 15;
        let max_cnt = (*from - &tos0).manhattan_length() / max_dist + 5;
        for to in tos.iter() {
            max_dist = min(max_dist, (*from - to).chessboard_length());
        }
        while let Some((_score, f, cnt)) = que.pop() {
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
            if cnt == max_cnt {
                continue;
            }
            // TODO lmoveも追加する
            let next = self.pos_smove_all(&f, max_dist);
            for &(t, command) in next.iter() {
                if parents.contains_key(&t) {
                    continue;
                }
                parents.insert(t, (f, command));
                let score = self.calc_astar_score(cnt + 1, &t, &tos0);
                que.push((score, t, cnt + 1));
            }
        }
        // どこにもたどり着けなかった場合
        None
    }
    // tos[0]を基準にする
    fn calc_astar_score(&self, cnt: i32, from: &Position, to: &Position) -> i32 {
        let mut score = cnt * 5;
        let d = (*from - to).manhattan_length() / 2;
        score += d;
        if from.y <= to.y {
            score += 2;
        }
        -score
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
    fn make_up_or_random_move_command(&mut self, from: &Position) -> Command {
        let llcd = LLCD::new(0, 1, 0);
        let to = *from + &llcd;
        if self.is_safe_coordinate(&to) {
            return Command::SMove(llcd);
        }
        self.make_random_move_command(from)
    }
    fn make_random_move_command(&mut self, from: &Position) -> Command {
        'outer_loop: for _ in 0..5 {
            let dir = self.rng.gen_range(0, 6);
            let dist = self.rng.gen_range(1, 5);
            let dx = [1, -1, 0, 0, 0, 0];
            let dy = [0, 0, 1, -1, 0, 0];
            let dz = [0, 0, 0, 0, 1, -1];
            let llcd = LLCD::new(dist * dx[dir], dist * dy[dir], dist * dz[dir]);
            if !self.is_safe_coordinate(&(*from + &llcd)) {
                continue;
            }
            for p in Region(*from, *from + &llcd).iter() {
                if p != *from && self.volatiles.contains(&p) {
                    continue 'outer_loop;
                }
            }
            return Command::SMove(llcd);
        }
        Command::Wait
    }
    fn make_fission_command(&mut self, from: &Position, seeds_cnt: usize) -> Command {
        let next = self.pos_ncd_all(from, from);
        if seeds_cnt == 0 || next.len() == 0 {
            return Command::Wait;
        }
        let to = next[self.rng.gen_range(0, next.len())];
        let ncd = to - from;
        let ncd = NCD::new(ncd.x, ncd.y, ncd.z);
        let m = (seeds_cnt - 1) / 2;
        Command::Fission(ncd, m)
    }
    // 適当に隣にいるやつを見つけてfusionする
    fn make_fusions_ncd_command(&mut self, from: &Position, s_index: usize) {
        for p_index in 0..self.bots.len() {
            if s_index == p_index || self.bots[p_index].next_commands.len() > 0 {
                continue;
            }
            let to = self.bots[p_index].bot.pos;
            let ncd = *from - &to;
            if ncd.manhattan_length() <= 2 && ncd.chessboard_length() == 1 {
                let ncd = NCD::new(ncd.x, ncd.y, ncd.z);
                self.make_fusion_command(p_index, s_index, &ncd);
                return;
            }
        }
    }
    fn make_fusion_command(&mut self, p_index: usize, s_index: usize, ncd: &NCD) {
        assert_eq!(self.bots[p_index].next_commands.len(), 0);
        assert!(self.bots[p_index].fusions_bid.is_none());
        for _ in 0..self.bots[s_index].next_commands.len() {
            // 同じタイミングに合わせる
            self.bots[p_index].next_commands.push_back(Command::Wait);
        }
        self.bots[p_index]
            .next_commands
            .push_back(Command::FusionP(*ncd));
        self.bots[p_index].fusions_bid = Some(self.bots[s_index].bot.bid);
        let rev_ncd = NCD::new(-ncd.x(), -ncd.y(), -ncd.z());
        self.bots[s_index]
            .next_commands
            .push_back(Command::FusionS(rev_ncd));
    }
    // 適当にfusion相手を見つけてfusionする:
    fn make_move_fusion_command(&mut self, from: &Position, s_index: usize) {
        assert_eq!(self.bots[s_index].next_commands.len(), 0);
        for p_index in 0..self.bots.len() {
            if p_index == s_index || self.bots[p_index].next_commands.len() != 0 {
                continue;
            }
            // p_index決定
            // p_indexの周りに行く命令を作る
            let to = self.bots[p_index].bot.pos;
            let tos = self.pos_ncd_all(from, &to);
            if tos.len() == 0 {
                return;
            }
            let (nto, commands) = match self.make_move_any_command(from, &tos) {
                None => {
                    return;
                }
                Some(v) => v,
            };
            // 命令を入れた後にfusionコマンドを入れる
            let mut commands = commands.into_iter().collect();
            self.set_volatiles(&from, &commands);
            self.bots[s_index].next_commands.append(&mut commands);
            let ncd = nto - &to;
            let ncd = NCD::new(ncd.x, ncd.y, ncd.z);
            self.make_fusion_command(p_index, s_index, &ncd);
            break;
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
        // println!("{:?} {:?}", from, command);
        // println!("{:?}", self.volatiles);
        // println!("{:?}", ps);
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
                || self.visited.contains(next)
            {
                continue;
            }
            self.candidates.push(*next);
            self.visited.insert(*next);
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
            Command::Fission(ncd, m) => {
                let new_bot = self.bots[bot_index].bot.fission(&ncd, m);
                self.added_bot_list.push(new_bot);
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
                let s_bid = self.bots[bot_index].fusions_bid.unwrap();
                for j in 0..self.bots.len() {
                    let mut s_bot = self.bots[j].bot.clone();
                    if self.bots[j].bot.bid == s_bid {
                        self.bots[bot_index].bot.fusion(&mut s_bot);
                        break;
                    }
                }
                self.bots[bot_index].fusions_bid = None;
                self.deleted_bot_list.push(s_bid);
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

        // Fission対応
        for i in 0..self.added_bot_list.len() {
            let mut bot_state = BotState::initial();
            bot_state.bot = self.added_bot_list[i].clone();
            self.bots.push(bot_state)
        }
        self.added_bot_list = vec![];
        // Fusion対応
        for &bid in self.deleted_bot_list.iter() {
            for i in 0..self.bots.len() {
                if self.bots[i].bot.bid == bid {
                    self.bots.remove(i);
                    break;
                }
            }
        }
        self.deleted_bot_list = vec![];
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
                    self.visited.insert(p);
                }
            }
        }
        while self.bots.len() < 7 {
            // 1ターンランダムムーブする
            for i in 0..self.bots.len() {
                let from = self.bots[i].bot.pos;
                let commands = vec![self.make_random_move_command(&from)]
                    .into_iter()
                    .collect();
                self.set_volatiles(&from, &commands);
                self.bots[i].next_commands = commands;
            }
            self.do_time_step_simulation();

            // fissionする
            for i in 0..self.bots.len() {
                let from = self.bots[i].bot.pos;
                let seed_cnt = self.bots[i].bot.seeds.len();
                let commands = vec![self.make_fission_command(&from, seed_cnt)]
                    .into_iter()
                    .collect();
                self.set_volatiles(&from, &commands);
                self.bots[i].next_commands = commands;
            }
            self.do_time_step_simulation();
        }
        // ブロック埋め
        let mut ng_count = 0;
        while self.candidates.len() > 0 || self.is_all_bot_command_done() {
            // println!("All Candidate: {}", self.visited.len());
            // :println!("Rest Candidate: {}", self.candidates.len());
            // 1 time step 実行
            for i in 0..self.bots.len() {
                if self.bots[i].next_commands.len() != 0 {
                    continue;
                }

                let from = self.bots[i].bot.pos;
                // candidateから1個取って処理する
                let to = self.select_one_candidate(&from);
                if to.is_some() {
                    let to = to.unwrap();
                    match self.make_target_fill_command(&from, &to) {
                        None => {
                            // 行けない
                            self.candidates.push(to);
                        }
                        Some(commands) => {
                            ng_count = 0;
                            let commands = commands.into_iter().collect();
                            // println!("{:?}", commands);
                            self.set_volatiles(&from, &commands);
                            self.bots[i].next_commands = commands;
                        }
                    }
                }

                // なんか失敗した場合
                if self.bots[i].next_commands.len() == 0 {
                    ng_count += 1;
                    // nanobotが無駄に多い場合で、隣にfusionできる相手がいる場合はfusionする
                    // 隣にいない場合は空間に余裕があるはずなので普通に処理する
                    self.make_fusions_ncd_command(&from, i);
                    if self.bots[i].next_commands.len() > 0 {
                        continue;
                    }
                }

                // やる事がない場合は上優先でランダムムーブする
                if self.bots[i].next_commands.len() == 0 {
                    let commands = vec![self.make_up_or_random_move_command(&from)]
                        .into_iter()
                        .collect();
                    self.set_volatiles(&from, &commands);
                    self.bots[i].next_commands = commands;
                }
            }
            self.do_time_step_simulation();
            // println!("{}", ng_count);
            if ng_count >= 1000 {
                // 詰んだっぽい
                // TODO
                println!("Give Up fill!");
                println!("All Candidate: {}", self.visited.len());
                println!("Rest Candidate: {}", self.candidates.len());
                return vec![];
            }
        }
        //  集合
        let mut ng_count = 0;
        while self.bots.len() > 1 {
            if ng_count > 100 {
                // 詰んだっぽい
                println!("Give Up fusion!");
                println!("All Candidate: {}", self.visited.len());
                println!("Rest Candidate: {}", self.candidates.len());
                return vec![];
            }
            for s_index in 0..self.bots.len() {
                let from = self.bots[s_index].bot.pos;
                self.make_fusions_ncd_command(&from, s_index);
            }
            for s_index in 0..self.bots.len() {
                if self.bots[s_index].next_commands.len() == 0 {
                    let from = self.bots[s_index].bot.pos;
                    self.make_move_fusion_command(&from, s_index);
                }
            }
            for i in 0..self.bots.len() {
                if self.bots[i].next_commands.len() == 0 {
                    ng_count += 1;
                    // 命令が入ってなかったらランダムムーブ
                    let from = self.bots[i].bot.pos;
                    let commands = vec![self.make_up_or_random_move_command(&from)]
                        .into_iter()
                        .collect();
                    self.set_volatiles(&from, &commands);
                    self.bots[i].next_commands = commands;
                }
            }
            self.do_time_step_simulation();
        }
        // (0, 0, 0)に戻る
        {
            let mut commands = self.make_return_command().unwrap();
            self.trace.append(&mut commands);
        }
        // println!("{} {:?}", self.trace.len(), self.trace);
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
    let bfs_ai = BfsAI::new(&config, &model, &model);
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
        let llcds = bfs_ai.pos_smove_all(&Position::new(30, 30, 30), 15);
        assert_eq!(llcds.len(), 90);
    }
    {
        let llcds = bfs_ai.pos_smove_all(&Position::new(1, 1, 1), 15);
        assert_eq!(llcds.len(), 48);
    }
    {
        let llcds = bfs_ai.pos_smove_all(&Position::new(98, 98, 98), 15);
        assert_eq!(llcds.len(), 48);
    }
    {
        bfs_ai.volatiles.insert(Position::new(28, 30, 30));
        let llcds = bfs_ai.pos_smove_all(&Position::new(30, 30, 30), 15);
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
fn make_random_move_command_test() {
    let model = Model::initial(30);
    let config = Config::new();
    let mut bfs_ai = BfsAI::new(&config, &model, &model);
    {
        let c = bfs_ai.make_random_move_command(&Position::zero());
        assert!(c != Command::Wait);
        let c = bfs_ai.make_random_move_command(&Position::zero());
        assert!(c != Command::Wait);
        let c = bfs_ai.make_random_move_command(&Position::zero());
        assert!(c != Command::Wait);
        let c = bfs_ai.make_random_move_command(&Position::zero());
        assert!(c != Command::Wait);
        let c = bfs_ai.make_random_move_command(&Position::zero());
        assert!(c != Command::Wait);
        let c = bfs_ai.make_random_move_command(&Position::zero());
        assert!(c != Command::Wait);
    }
    {
        bfs_ai.volatiles.insert(Position::new(1, 0, 0));
        bfs_ai.volatiles.insert(Position::new(0, 1, 0));
        bfs_ai.volatiles.insert(Position::new(0, 0, 1));
        let c = bfs_ai.make_random_move_command(&Position::zero());
        assert!(c == Command::Wait);
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
