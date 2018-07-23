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
        BfsAI {
            rng: XorShiftRng::new_unseeded(),
            state: State::initial_with_model(source),
            current: source.clone(),
            target: target.clone(),
            bots: vec![BotState::initial()],
            volatiles: HashSet::new(),
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
    fn select_one_candidate(&mut self, from: &Position) -> Position {
        let mut target = self.candidates.len();
        let mut best = 1 << 30;
        for (i, c) in self.candidates.iter().enumerate() {
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
        assert!(target != self.candidates.len());
        let ret = self.candidates[target];
        self.candidates.remove(target);
        ret
    }
    // posからvolatileしないncdの位置を全部返す
    fn pos_ncd_all(&self, pos: &Position) -> Vec<Position> {
        let mut poss = vec![];
        for ncd in all_ncd().iter() {
            let new_c = *pos + ncd;
            if !self.is_safe_coordinate(&new_c) {
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
            let dx = [1, -1, 0, 0, 0, 0];
            let dy = [0, 0, 1, -1, 0, 0];
            let dz = [1, 0, 0, 0, 1, -1];
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
    // SMove・LMoveの系列をbfsで作って移動してfilする
    fn make_target_fill_command(&self, from: &Position, to: &Position) -> Option<Vec<Command>> {
        let mut ret = vec![];
        let tos = self.pos_ncd_all(to);
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
        while let Some(f) = que.pop_back() {
            if tos.contains(&f) {
                // 経路復元してreverseで正順にしてコマンドの系列を返す
                let mut ret = vec![];
                let mut next = f;
                while next != f {
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
        match self.make_move_any_command(&from, &vec![Position::new(0, 0, 0)]) {
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
        for p in ps.iter() {
            assert!(self.volatiles.contains(p));
            self.volatiles.remove(p);
        }
    }
    // posの位置ブロックがFillされたときに周りのVoxelでまだcandidateに入ってないのを入れる
    fn update_full_candidate(&mut self, pos: &Position) {
        for next in adjacent(*pos).iter() {
            if self.current.voxel_at(*next) == Voxel::Full
                || self.target.voxel_at(*next) == Voxel::Void
            {
                continue;
            }
            self.candidates.push(*next);
        }
        unimplemented!();
    }
    // bot_indexのnext_commandsの先頭に入っているCommandを実行する
    fn do_command(&mut self, bot_index: usize) {
        let from = self.bots[bot_index].bot.pos;
        let command = self.bots[bot_index].next_commands[0];
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
            commands.push((bot.bot.bid, bot.next_commands.pop_front().unwrap()));
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
                match self.make_target_fill_command(&from, &to) {
                    None => {
                        // TODO
                        assert!(false);
                        // TODO candidateを元に戻すか保持する
                        self.bots[i].nop();
                    }
                    Some(commands) => {
                        let commands = commands.into_iter().collect();
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
