use ai::AssembleAI;
use ai::config::Config;
use ai::utils::*;
use common::*;
use model::*;
use state::State;
use std::cmp::min;
use std::collections::HashMap;
use std::collections::HashSet;
use std::iter::repeat;

pub struct SquadAI {}

impl SquadAI {
    pub fn new(_config: &Config) -> Self {
        SquadAI {}
    }
}

struct Squad {
    nanobots: Vec<Nanobot>,

    // その squad が担当する長方形領域
    associated: Bounding,
}

#[derive(Debug, Copy, Clone, Hash, Eq, Ord, PartialOrd, PartialEq)]
struct BotCommand {
    time: usize,
    bid: Bid,
    command: Command,
}

impl BotCommand {
    fn new(bid: Bid, time: usize, command: Command) -> Self {
        Self { bid, time, command }
    }
}

impl Squad {}

#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Debug, Hash)]
enum SquadCommand {
    Move(Vec<Position>),
    GFill(NCD),
}

impl AssembleAI for SquadAI {
    fn assemble(&mut self, model: &Model) -> Vec<Command> {
        let bounding = match calc_bounding_box(model) {
            Some(b) => b,
            None => {
                return vec![Command::Halt];
            }
        };

        // squad を spawn する
        let (squads, bot_commands) = self.spawn_squads(&bounding, model);

        println!("{:?}", bot_commands);

        let mut command_lists = vec![vec![]; squads.len()];

        // 各フロアを埋めていく
        for floor in bounding.min_y..(bounding.max_y + 1) {
            for (i, squad) in squads.iter().enumerate() {
                let mut commands = self.fill_floor(squad, floor as usize, model);
                command_lists[i].append(&mut commands);
            }
        }

        // SquadCommandをBotCommandに戻す


        // hoge
        let commands = bot_commands_to_commands(&bot_commands);

        println!("commands={:?}", commands);
        commands
    }
}

fn bot_move_straight_x(len: i32, bot: &mut Nanobot, t: usize) -> Vec<BotCommand> {
    bot.pos = bot.pos + &Position::new(len, 0, 0);
    move_straight_x(len).iter().enumerate().map(|(i, c)|
        BotCommand::new(bot.bid, i + t, *c)).collect::<Vec<_>>()
}

fn bot_move_straight_z(len: i32, bot: &mut Nanobot, t: usize) -> Vec<BotCommand> {
    bot.pos = bot.pos + &Position::new(0, 0, len);
    move_straight_z(len).iter().enumerate().map(|(i, c)|
        BotCommand::new(bot.bid, i + t, *c)).collect::<Vec<_>>()
}

fn bot_fission(bot: &mut Nanobot, t: usize, ncd: NCD, m: usize) -> (Nanobot, Vec<BotCommand>) {
    let new_bot = bot.fission(&ncd, m);
    let bot_command1 = BotCommand::new(bot.bid, t, Command::Fission(ncd, m));
    // botのライフタイムを明確にするためにマーカーを置いておく
    let bot_command2 = BotCommand::new(new_bot.bid, t, Command::Wait);
    (new_bot, vec![bot_command1, bot_command2])
}

fn bot_commands_to_commands(bot_commands: &[BotCommand]) -> Vec<Command> {
    let mut ret = Vec::new();

    let mut current_bots = HashSet::new();
    current_bots.insert(Bid(1));

    let mut bot_commands = bot_commands.to_vec();
    bot_commands.sort();

    let mut t = 0;
    let mut i = 0;
    while i < bot_commands.len() {
        let mut command_map = HashMap::new();
        let mut next_bots = current_bots.clone();

        while i < bot_commands.len() && bot_commands[i].time == t {
            let c = bot_commands[i];
            if current_bots.contains(&c.bid) {
                println!("c={:?}", c);

                if command_map.insert(c.bid, c.command).is_some() {
                    panic!("duplicate key: t={}, bid={:?}, command={:?}", t, c.bid, c.command);
                }
                if let Command::FusionS(_) = c.command {
                    next_bots.remove(&c.bid);
                }
            } else {
                assert_eq!(c.command, Command::Wait);
                next_bots.insert(c.bid);
            }
            i += 1;
        }

        {
            let mut bids = current_bots.iter().collect::<Vec<_>>();
            bids.sort();
            for bid in bids {
                if let Some(command) = command_map.get(bid) {
                    ret.push(*command);
                } else {
                    ret.push(Command::Wait);
                }
            }
        }

        current_bots = next_bots.clone();
        t += 1;
    }

    ret
}

impl SquadAI {
    fn spawn_squads(&mut self, bounding: &Bounding, model: &Model) -> (Vec<Squad>, Vec<BotCommand>) {
        let mut nanobots = HashMap::new();
        nanobots.insert(Bid(1), Nanobot::initial());

        let mut bot_commands = Vec::new();
        let mut t = 0;

        {
            let mut bot = nanobots.get_mut(&Bid(1)).unwrap();

            let mut cmd1 = bot_move_straight_x(bounding.min_x, &mut bot, t);
            t += cmd1.len();
            bot_commands.append(&mut cmd1);
            println!("t={}, bot_commands={:?}", t, bot_commands);

            let mut cmd2 = bot_move_straight_z(bounding.min_z, &mut bot, t);
            t += cmd2.len();
            bot_commands.append(&mut cmd2);
            println!("t={}, bot_commands={:?}", t, bot_commands);
        }

        let unit_size = 4 as usize;
        let n_split = 10;
        let x_width = bounding.max_x - bounding.min_x + 1;
        let x_interval = (x_width + n_split - 1) / n_split;

        let mut right_most = Bid(1);
        let mut squads = Vec::new();

        for i in 0..n_split {
            let mut squad_member1 = nanobots.get_mut(&right_most).unwrap().clone();

            let next_t = if i != n_split - 1 {
                println!("squad_member1={:?}", squad_member1);
                let m = squad_member1.seeds.len() - unit_size;
                let (mut next_bot, mut commands) = bot_fission(
                    &mut squad_member1, t, NCD::new(1, 0, 0), m);
                nanobots.insert(next_bot.bid, next_bot.clone());
                bot_commands.append(&mut commands);
                right_most = next_bot.bid;
                t += 1;

                let mut smoves = bot_move_straight_x(x_interval - 1, &mut next_bot, t);
                let next_t = t + smoves.len();
                bot_commands.append(&mut smoves);

                next_t
            } else {
                0
            };

            let (mut squad_member2, mut commands2) = bot_fission(
                &mut squad_member1, t, NCD::new(0, 0, 1), 1);
            nanobots.insert(squad_member2.bid, squad_member2.clone());
            bot_commands.append(&mut commands2);
            t += 1;

            let (mut squad_member3, mut commands3) = bot_fission(
                &mut squad_member1, t, NCD::new(1, 0, 0), 0);
            nanobots.insert(squad_member3.bid, squad_member3.clone());
            bot_commands.append(&mut commands3);

            let (mut squad_member4, mut commands4) = bot_fission(
                &mut squad_member2, t, NCD::new(1, 0, 0), 0);
            nanobots.insert(squad_member4.bid, squad_member4.clone());
            bot_commands.append(&mut commands4);
            t += 1;

            squads.push(
                Squad {
                    associated: Bounding {
                        min_x: squad_member1.pos.x,
                        max_x: min(squad_member1.pos.x + x_interval, bounding.max_x),
                        min_z: bounding.min_z,
                        max_z: bounding.max_z,
                        min_y: bounding.min_y,
                        max_y: bounding.max_y,
                    },
                    nanobots: vec![
                        squad_member1,
                        squad_member2,
                        squad_member3,
                        squad_member4,
                    ],
                }
            );

            t = if next_t != 0 { next_t } else { t };
        }

        (squads, bot_commands)
    }

    // その squad が担当する領域を塗る
    fn fill_floor(&mut self, squad: &Squad, floor: usize, model: &Model) -> Vec<SquadCommand> {
        let mut commands = Vec::new();

        // greedy に長方形領域を塗っていく
        let r = model.matrix.len();
        let mut filled = vec![vec![false; r]; r];

        // FIXME: voxel::void な場所を塗ってしまう
        for x in squad.associated.min_x..(squad.associated.max_x + 1) {
            for z in squad.associated.min_z..(squad.associated.max_z + 1) {
                if model.matrix[x as usize][floor][z as usize] == Voxel::Full
                    && !filled[x as usize][z as usize] {
                    // (x, z) を左上とする長方形を塗る
                    let mut x_right = x;
                    for xx in x..min(x + 30, squad.associated.max_x) {
                        if model.matrix[xx as usize][floor][z as usize] != Voxel::Full
                            || filled[xx as usize][z as usize] {
                            break;
                        }
                        x_right = xx;
                    }
                    let mut z_bottom = z;
                    for zz in z..min(z + 30, squad.associated.max_z) {
                        if model.matrix[x as usize][floor][zz as usize] != Voxel::Full
                            || filled[x as usize][zz as usize] {
                            break;
                        }
                        z_bottom = zz;
                    }

                    commands.push(SquadCommand::Move(vec![
                        Position::new(x as i32, (floor + 1) as i32, z as i32),
                        Position::new(x_right as i32, (floor + 1) as i32, z as i32),
                        Position::new(x as i32, (floor + 1) as i32, z_bottom as i32),
                        Position::new(x_right as i32, (floor + 1) as i32, z_bottom as i32),
                    ]));
                    commands.push(SquadCommand::GFill(NCD::new(0, -1, 0)));

                    for xx in x..(x_right + 1) {
                        for zz in z..(z_bottom + 1) {
                            filled[xx as usize][zz as usize] = true;
                        }
                    }
                }
            }
        }

        commands
    }
}
