use std::{
    fmt::Display,
    io::Write,
    num::{ParseFloatError, ParseIntError},
    sync::{
        atomic::{self, AtomicBool, Ordering, AtomicUsize},
        mpsc,
    }, time::Instant,
};

use crate::{
    board::{
        evaluation::{is_mate_score, parameters::EvalParams, MATE_SCORE, set_eval_params},
        Board,
    },
    definitions::{BLACK, WHITE, MEGABYTE},
    errors::{FenParseError, MoveParseError},
    search::parameters::{SearchParams, get_search_params, set_search_params},
    searchinfo::{SearchInfo, SearchLimit},
    threadlocal::ThreadData,
    NAME, VERSION, transpositiontable::TranspositionTable,
};

const UCI_DEFAULT_HASH_MEGABYTES: usize = 4;

enum UciError {
    ParseOption(String),
    ParseFen(FenParseError),
    ParseMove(MoveParseError),
    UnexpectedCommandTermination(String),
    InvalidFormat(String),
    UnknownCommand(String),
}

impl From<MoveParseError> for UciError {
    fn from(err: MoveParseError) -> Self {
        Self::ParseMove(err)
    }
}

impl From<FenParseError> for UciError {
    fn from(err: FenParseError) -> Self {
        Self::ParseFen(err)
    }
}

impl From<ParseFloatError> for UciError {
    fn from(pfe: ParseFloatError) -> Self {
        Self::ParseOption(pfe.to_string())
    }
}

impl From<ParseIntError> for UciError {
    fn from(pie: ParseIntError) -> Self {
        Self::ParseOption(pie.to_string())
    }
}

impl Display for UciError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ParseOption(s) => write!(f, "ParseOption: {s}"),
            Self::ParseFen(s) => write!(f, "ParseFen: {s}"),
            Self::ParseMove(s) => write!(f, "ParseMove: {s}"),
            Self::UnexpectedCommandTermination(s) => {
                write!(f, "UnexpectedCommandTermination: {s}")
            }
            Self::InvalidFormat(s) => write!(f, "InvalidFormat: {s}"),
            Self::UnknownCommand(s) => write!(f, "UnknownCommand: {s}"),
        }
    }
}

// position fen
// position startpos
// ... moves e2e4 e7e5 b7b8q
fn parse_position(text: &str, pos: &mut Board) -> Result<(), UciError> {
    let mut parts = text.split_ascii_whitespace();
    let command = parts.next().ok_or_else(|| {
        UciError::UnexpectedCommandTermination("No command in parse_position".into())
    })?;
    if command != "position" {
        return Err(UciError::InvalidFormat("Expected 'position'".into()));
    }
    let determiner = parts.next().ok_or_else(|| {
        UciError::UnexpectedCommandTermination("No determiner after \"position\"".into())
    })?;
    if determiner == "startpos" {
        pos.set_startpos();
        let moves = parts.next(); // skip "moves"
        if !(matches!(moves, Some("moves") | None)) {
            return Err(UciError::InvalidFormat(
                "Expected either \"moves\" or no content to follow \"startpos\".".into(),
            ));
        }
    } else {
        if determiner != "fen" {
            return Err(UciError::InvalidFormat(format!(
                "Unknown term after \"position\": {determiner}"
            )));
        }
        let mut fen = String::new();
        for part in &mut parts {
            if part == "moves" {
                break;
            }
            fen.push_str(part);
            fen.push(' ');
        }
        pos.set_from_fen(&fen)?;
    }
    for san in parts {
        pos.zero_height(); // stuff breaks really hard without this lmao
        let m = pos.parse_uci(san)?;
        pos.make_move_hce(m);
    }
    pos.zero_height();
    // eprintln!("{}", pos);
    Ok(())
}

fn parse_go(text: &str, info: &mut SearchInfo, pos: &mut Board) -> Result<(), UciError> {
    #![allow(clippy::too_many_lines)]
    let mut depth: Option<i32> = None;
    let mut moves_to_go: Option<u64> = None;
    let mut movetime: Option<u64> = None;
    let mut clocks: [Option<i64>; 2] = [None, None];
    let mut incs: [Option<i64>; 2] = [None, None];
    let mut nodes: Option<u64> = None;

    let mut parts = text.split_ascii_whitespace();
    let command = parts
        .next()
        .ok_or_else(|| UciError::UnexpectedCommandTermination("No command in parse_go".into()))?;
    if command != "go" {
        return Err(UciError::InvalidFormat("Expected \"go\"".into()));
    }

    while let Some(part) = parts.next() {
        match part {
            "depth" => depth = Some(part_parse("depth", parts.next())?),
            "movestogo" => moves_to_go = Some(part_parse("movestogo", parts.next())?),
            "movetime" => movetime = Some(part_parse("movetime", parts.next())?),
            "wtime" => clocks[pos.turn() as usize] = Some(part_parse("wtime", parts.next())?),
            "btime"  => clocks[1 ^ pos.turn() as usize] = Some(part_parse("btime", parts.next())?),
            "winc" => incs[pos.turn() as usize] = Some(part_parse("winc", parts.next())?),
            "binc" => incs[1 ^ pos.turn() as usize] = Some(part_parse("binc", parts.next())?),
            "infinite" => info.limit = SearchLimit::Infinite,
            "nodes" => nodes = Some(part_parse("nodes", parts.next())?),
            other => return Err(UciError::InvalidFormat(format!("Unknown term: {other}"))),
        }
    }

    if let Some(movetime) = movetime {
        info.limit = SearchLimit::Time(movetime);
    } else if let Some(depth) = depth {
        info.limit = SearchLimit::Depth(depth.into());
    } else if let Some(nodes) = nodes {
        info.limit = SearchLimit::Nodes(nodes);
    }

    if let [Some(our_clock), Some(their_clock)] = clocks {
        let [our_inc, their_inc] = [incs[0].unwrap_or(0), incs[1].unwrap_or(0)];
        let our_clock: u64 = our_clock.try_into().unwrap_or(0);
        let their_clock: u64 = their_clock.try_into().unwrap_or(0);
        let our_inc: u64 = our_inc.try_into().unwrap_or(0);
        let their_inc: u64 = their_inc.try_into().unwrap_or(0);
        let moves_to_go = moves_to_go.unwrap_or_else(|| pos.predicted_moves_left());
        let (time_window, max_time_window) = SearchLimit::compute_time_windows(our_clock, moves_to_go, our_inc);
        info.limit = SearchLimit::Dynamic {
            our_clock,
            their_clock,
            our_inc,
            their_inc,
            moves_to_go,
            max_time_window,
            time_window
        };
    } else if clocks.iter().chain(incs.iter()).any(Option::is_some) {
        return Err(UciError::InvalidFormat("at least one of [wtime, btime, winc, binc] provided, but not all.".into()));
    }

    info.start_time = Instant::now();

    Ok(())
}

fn part_parse<T>(target: &str, next_part: Option<&str>) -> Result<T, UciError>
where
    T: std::str::FromStr,
    <T as std::str::FromStr>::Err: std::fmt::Display,
{
    let next_part = next_part.ok_or_else(|| UciError::InvalidFormat(format!("nothing after \"{target}\"")))?;
    let value = next_part.parse();
    value.map_err(|e| UciError::InvalidFormat(format!("value for {target} is not a number: {e}, tried to parse {next_part}")))
}

struct SetOptions {
    pub search_config: SearchParams,
    pub hash_mb: Option<usize>,
    pub threads: Option<usize>,
}

fn parse_setoption(
    text: &str,
    _info: &mut SearchInfo,
    pre_config: SetOptions,
) -> Result<SetOptions, UciError> {
    use UciError::UnexpectedCommandTermination;
    let mut parts = text.split_ascii_whitespace();
    parts.next().unwrap();
    parts
        .next()
        .map(|s| {
            assert!(
                s == "name",
                "unexpected character after \"setoption\", expected \"name\", got {}",
                s
            );
        })
        .ok_or_else(|| UnexpectedCommandTermination("no name after setoption".into()))?;
    let opt_name = parts.next().ok_or_else(|| {
        UnexpectedCommandTermination("no option name given after \"setoption name\"".into())
    })?;
    parts.next().map_or_else(|| panic!("no value after \"setoption name {opt_name}\""), |s| assert!(s == "value", "unexpected character after \"setoption name {opt_name}\", expected \"value\", got {s}"));
    let opt_value = parts.next().ok_or_else(|| {
        UnexpectedCommandTermination(format!(
            "no option value given after \"setoption name {opt_name} value\""
        ))
    })?;
    let mut out = pre_config;
    let id_parser_pairs = out.search_config.ids_with_parsers();
    let mut found_match = false;
    for (param_name, mut parser) in id_parser_pairs {
        if param_name == opt_name {
            let res = parser(opt_value);
            if let Err(e) = res {
                return Err(UciError::InvalidFormat(e.to_string()));
            }
            found_match = true;
            break;
        }
    }
    if found_match {
        return Ok(out);
    }
    match opt_name {
        "Hash" => {
            let value = opt_value.parse()?;
            assert!(value > 0 && value <= 8192, "Hash value must be between 1 and 8192");
            out.hash_mb = Some(value);
        }
        "Threads" => {
            let value = opt_value.parse()?;
            assert!(value > 0 && value <= 512, "Threads value must be between 1 and 512");
            out.threads = Some(value);
        }
        "MultiPV" => {
            let value: usize = opt_value.parse()?;
            assert!(value > 0 && value <= 500, "MultiPV value must be between 1 and 500");
            MULTI_PV.store(value, Ordering::SeqCst);
        },
        _ => eprintln!("ignoring option {opt_name}"),
    }
    Ok(out)
}

static KEEP_RUNNING: AtomicBool = AtomicBool::new(true);

fn stdin_reader() -> mpsc::Receiver<String> {
    let (sender, reciever) = mpsc::channel();
    std::thread::Builder::new()
        .name("stdin-reader".into())
        .spawn(|| stdin_reader_worker(sender))
        .expect("Couldn't start stdin reader worker thread");
    reciever
}

fn stdin_reader_worker(sender: mpsc::Sender<String>) {
    let mut linebuf = String::with_capacity(128);
    while std::io::stdin().read_line(&mut linebuf).is_ok() {
        let cmd = linebuf.trim();
        if cmd.is_empty() {
            linebuf.clear();
            continue;
        }
        if sender.send(cmd.to_owned()).is_err() {
            break;
        }
        if !KEEP_RUNNING.load(atomic::Ordering::SeqCst) {
            break;
        }
        linebuf.clear();
    }
    std::mem::drop(sender);
}

pub fn format_score(score: i32, turn: u8) -> String {
    assert!(turn == WHITE || turn == BLACK);
    if is_mate_score(score) {
        let plies_to_mate = MATE_SCORE - score.abs();
        let moves_to_mate = (plies_to_mate + 1) / 2;
        if score > 0 {
            format!("mate {moves_to_mate}")
        } else {
            format!("mate -{moves_to_mate}")
        }
    } else {
        format!("cp {score}")
    }
}

fn print_uci_response(full: bool) {
    println!("id name {NAME} {VERSION}");
    println!("id author Cosmo");
    println!("option name Hash type spin default {UCI_DEFAULT_HASH_MEGABYTES} min 1 max 8192");
    println!("option name Threads type spin default 1 min 1 max 512");
    // println!("option name MultiPV type spin default 1 min 1 max 500");
    if full {
        for (id, default) in SearchParams::default().ids_with_values() {
            println!("option name {id} type spin default {default} min -999999 max 999999");
        }
    }
    println!("uciok");
}

pub static MULTI_PV: AtomicUsize = AtomicUsize::new(1);
pub fn is_multipv() -> bool {
    MULTI_PV.load(Ordering::SeqCst) > 1
}

pub fn main_loop(params: EvalParams) {
    let mut pos = Board::new();

    let mut tt = TranspositionTable::new();
    tt.resize(UCI_DEFAULT_HASH_MEGABYTES * MEGABYTE); // default hash size

    let mut info = SearchInfo::default();
    
    let global_stopped = AtomicBool::new(false);
    info.set_global_stopped(&global_stopped);

    unsafe { set_eval_params(params); }

    let stdin = std::sync::Mutex::new(stdin_reader());

    info.set_stdin(&stdin);

    let mut thread_data = Vec::new();
    thread_data.push(ThreadData::new(0));

    for td in &mut thread_data {
        td.alloc_tables();
    }

    loop {
        std::io::stdout().flush().unwrap();
        let line = stdin.lock().unwrap().recv().expect("Couldn't read from stdin");
        let input = line.trim();

        let res = match input {
            "\n" => continue,
            "uci" => {
                print_uci_response(false);
                Ok(())
            }
            "ucifull" => {
                print_uci_response(true);
                Ok(())
            }
            "isready" => {
                println!("readyok");
                Ok(())
            }
            "quit" => {
                info.quit = true;
                break;
            }
            "ucinewgame" => {
                let res = parse_position("position startpos\n", &mut pos);
                tt.clear();
                for td in &mut thread_data {
                    td.alloc_tables();
                }
                res
            }
            "eval" => {
                println!("{}", pos.evaluate::<true>(thread_data.first_mut().unwrap(), 0));
                Ok(())
            }
            input if input.starts_with("setoption") => {
                let pre_config = SetOptions { search_config: get_search_params().clone(), hash_mb: None, threads: None };
                let res = parse_setoption(input, &mut info, pre_config);
                match res {
                    Ok(conf) => {
                        unsafe { set_search_params(conf.search_config); }
                        if let Some(hash_mb) = conf.hash_mb {
                            let new_size = hash_mb * MEGABYTE;
                            tt.resize(new_size);
                        }
                        if let Some(threads) = conf.threads {
                            thread_data = (0..threads).map(ThreadData::new).collect();
                            for td in &mut thread_data {
                                td.alloc_tables();
                            }
                        }
                        Ok(())
                    }
                    err => err.map(|_| ()),
                }
            },
            input if input.starts_with("position") => {
                let res = parse_position(input, &mut pos);
                if res.is_ok() {
                    for t in &mut thread_data {
                        t.nnue.refresh_acc(&pos);
                    }
                }
                res
            }
            input if input.starts_with("go") => {
                let res = parse_go(input, &mut info, &mut pos);
                if res.is_ok() {
                    tt.increase_age();
                    pos.search_position::<true>(&mut info, &mut thread_data, tt.view());
                }
                res
            }
            _ => Err(UciError::UnknownCommand(input.to_string())),
        };

        if let Err(e) = res {
            eprintln!("Error: {e}");
        }

        if info.quit {
            // quit can be set true in parse_go
            break;
        }
    }
    KEEP_RUNNING.store(false, atomic::Ordering::SeqCst);
}
