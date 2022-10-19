use std::{
    fs::File,
    io::{BufRead, BufReader, BufWriter, Write},
    path::Path,
};

use crate::{
    board::Board,
    definitions::{depth::Depth, WHITE},
    searchinfo::{SearchInfo, SearchLimit},
    threadlocal::ThreadData,
};

use rayon::prelude::*;

fn batch_convert(depth: i32, fens: &[String], evals: &mut Vec<i32>) {
    let mut pos = Board::default();
    let mut t = vec![ThreadData::new()];
    pos.set_hash_size(16);
    pos.alloc_tables();
    for fen in fens {
        pos.set_from_fen(fen).unwrap();
        // no NNUE for generating training data.
        t.iter_mut().for_each(|thread_data| thread_data.nnue.refresh_acc(&pos));
        let mut info = SearchInfo {
            print_to_stdout: false,
            limit: SearchLimit::Depth(Depth::new(depth)),
            ..SearchInfo::default()
        };
        let (pov_score, _) = pos.search_position(&mut info, &mut t);
        let score = if pos.turn() == WHITE { pov_score } else { -pov_score };
        evals.push(score);
    }
}

pub fn wdl_to_nnue<P1: AsRef<Path>, P2: AsRef<Path>>(
    input_file: P1,
    output_file: P2,
    format: Format,
) -> std::io::Result<()> {
    let input_file = File::open(input_file)?;
    let output_file = File::create(output_file)?;
    let reader = BufReader::new(input_file);
    let (fens, outcomes) = match format {
        Format::OurTexel => from_our_texel_format(reader)?,
        Format::Marlinflow => from_marlinflow_format(reader)?,
    };
    let cores = num_cpus::get();
    let chunk_size = fens.len() / cores;
    let evals = fens
        .par_chunks(chunk_size)
        .map(|chunk| {
            let mut inner_evals = Vec::new();
            batch_convert(10, chunk, &mut inner_evals);
            inner_evals
        })
        .flatten()
        .collect::<Vec<_>>();
    let mut output = BufWriter::new(output_file);
    for ((fen, outcome), eval) in fens.into_iter().zip(outcomes).zip(&evals) {
        writeln!(output, "{fen} | {eval} | {outcome:.1}")?;
    }
    Ok(())
}

fn from_our_texel_format(mut reader: BufReader<File>) -> Result<(Vec<String>, Vec<f32>), std::io::Error> {
    let mut line = String::new();
    let mut fens = Vec::new();
    let mut outcomes = Vec::new();
    while reader.read_line(&mut line)? > 0 {
        let (fen, outcome) = line.trim().split_once(';').unwrap();
        fens.push(fen.to_string());
        outcomes.push(outcome.parse::<f32>().unwrap());
        line.clear();
    }
    Ok((fens, outcomes))
}

fn from_marlinflow_format(mut reader: BufReader<File>) -> Result<(Vec<String>, Vec<f32>), std::io::Error> {
    let mut line = String::new();
    let mut fens = Vec::new();
    let mut outcomes = Vec::new();
    while reader.read_line(&mut line)? > 0 {
        let (fen, rest) = line.trim().split_once('|').unwrap();
        let fen = fen.trim();
        let outcome = rest.trim().split_once('|').unwrap().1.trim();
        fens.push(fen.to_string());
        outcomes.push(outcome.parse::<f32>().unwrap());
        line.clear();
    }
    Ok((fens, outcomes))
}

#[derive(Debug, Clone, Copy)]
pub enum Format {
    OurTexel,
    Marlinflow,
}