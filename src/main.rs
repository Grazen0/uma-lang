mod error;
mod parser;
mod scanner;

use std::{fs, path::PathBuf};

use clap::Parser;

use crate::{
    parser::{LangParser, ParseError},
    scanner::{ScanError, Scanner},
};

#[derive(clap::Parser)]
struct Args {
    /// Input file
    input: PathBuf,
}

fn find_line_start(s: &[u8], pos: usize) -> usize {
    let mut line_start = pos;

    while line_start > 0 && s[line_start - 1] != b'\n' {
        line_start -= 1;
    }

    line_start
}

fn print_source_higlight(src: &[u8], pos: usize) {
    let line_start = find_line_start(src, pos);
    let line_end = src[line_start..]
        .iter()
        .position(|&ch| ch == b'\n')
        .unwrap_or(src.len() - 1);

    let line = &src[line_start..line_start + line_end];
    println!("{}", String::from_utf8(Vec::from(line)).unwrap());
    println!("{}^", " ".repeat(pos - line_start));
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let input = fs::read_to_string(args.input)?;

    let mut scanner = Scanner::new(&input);
    let mut parser = LangParser::new(&mut scanner);

    let program = match parser.program() {
        Ok(p) => p,
        Err(e) => {
            let input_bytes = input.as_bytes();

            eprintln!("{e}");
            match e {
                ParseError::Scan(_) => {
                    todo!()
                }
                ParseError::UnexpectedToken { found } => {
                    print_source_higlight(
                        input_bytes,
                        found.map(|t| t.pos).unwrap_or(input_bytes.len() - 1),
                    );
                }
                ParseError::ExpectedToken { found, .. } => {
                    print_source_higlight(
                        input_bytes,
                        found.map(|t| t.pos).unwrap_or(input_bytes.len() - 1),
                    );
                }
            }

            return Ok(());
        }
    };

    parser.ensure_done()?;

    println!("{:#?}", program);
    Ok(())
}
