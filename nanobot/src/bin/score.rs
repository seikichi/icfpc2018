extern crate getopts;
extern crate nanobot_lib;

use getopts::Options;
use nanobot_lib::common::read_trace_file;
use nanobot_lib::model::Model;
use nanobot_lib::state::State;
use std::env;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

enum OutputFormat {
    Text,
    Json,
}

fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {} [options]", program);
    print!("{}", opts.usage(&brief));
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();

    let mut opts = Options::new();
    opts.optopt("", "source", "set source model", "FILE");
    opts.optopt("", "target", "set target model", "FILE");
    opts.reqopt("", "trace", "set trace", "FILE");
    opts.optflag("h", "help", "print this help menu");
    opts.optflag("j", "json", "print in JSON format");
    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => {
            print_usage(&program, opts);
            panic!(f.to_string())
        }
    };
    if matches.opt_present("h") {
        print_usage(&program, opts);
        return;
    }
    if !matches.opt_present("source") && !matches.opt_present("target") {
        print_usage(&program, opts);
        panic!("source or target file should be selected");
    }
    let output_format = if matches.opt_present("j") {
        OutputFormat::Json
    } else {
        OutputFormat::Text
    };

    // Load
    let trace = read_trace_file(Path::new(&matches.opt_str("trace").unwrap())).unwrap();
    let mut r = 0;
    let source_model = if matches.opt_present("source") {
        let f = File::open(Path::new(&matches.opt_str("source").unwrap())).expect("file not found");
        let mut f = BufReader::new(f);
        let model = Model::new(&mut f).expect("failed to open model");
        r = model.matrix.len();
        Some(model)
    } else {
        None
    };
    let target_model = if matches.opt_present("target") {
        let f = File::open(Path::new(&matches.opt_str("target").unwrap())).expect("file not found");
        let mut f = BufReader::new(f);
        let model = Model::new(&mut f).expect("failed to open model");
        r = model.matrix.len();
        Some(model)
    } else {
        None
    };
    let source_model = match source_model {
        Some(model) => model,
        None => Model::initial(r),
    };
    let target_model = match target_model {
        Some(model) => model,
        None => Model::initial(r),
    };
    if source_model.matrix.len() != r || target_model.matrix.len() != r {
        panic!("source_model and target_model size are not same")
    }
    let mut state = State::initial_with_model(&source_model);

    // Simulate
    let mut offset = 0;
    while offset < trace.len() {
        let bot_cnt = state.get_bot_count();
        match state.update_time_step(&trace[offset..offset + bot_cnt]) {
            Ok(_) => {}
            Err(err) => {
                panic!(err.to_string());
            }
        }
        offset += bot_cnt;
    }
    match state.end_check(&target_model) {
        Ok(_) => {}
        Err(err) => {
            println!("Failure::");
            panic!("{}", err.to_string());
        }
    }

    match output_format {
        OutputFormat::Text => {
            println!("Success:: ");
            println!("Time:      ?");
            println!("Commands:  {}", trace.len());
            println!("Energy:    {}", state.get_energy());
            println!("ClockTime: ?ms");
        }
        OutputFormat::Json => {
            println!("{{\"commands\": {}, \"energy\": {}}}", trace.len(), state.get_energy());
        }
    }
}
