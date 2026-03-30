mod codegen;
mod core;
mod parser;
mod scanner;

use std::ops::Range;

use clap::Parser;
use crossterm::style::Stylize;

use crate::{
    core::SourceFile,
    parser::{LangParser, ParseError},
    scanner::Scanner,
};

fn digit_count(n: usize, radix: u32) -> usize {
    (f32::log((n + 1) as f32, radix as f32)).ceil() as usize
}

#[derive(clap::Parser)]
struct Args {
    /// Input file
    input: String,
}

fn print_error_header(src: &SourceFile, err: &ParseError) {
    eprintln!(
        "{}",
        format!("{}: {}", "error".red(), err.fmt_with_src(src.contents())).bold()
    );
}

fn compute_err_byte_range(src: &SourceFile, err: &ParseError) -> Range<usize> {
    err.byte_range().unwrap_or_else(|| {
        let end = src.contents().len();
        end - 1..end
    })
}

fn print_src_line(src: &SourceFile, line: usize, col_pad: usize) {
    let line_range = src.line_range_bytes(line);
    let line_str = &src.contents()[line_range.start..line_range.end - 1];

    print_signcolumn(col_pad, Some(line + 1));
    eprintln!("{}", line_str);
}

fn print_underline(src: &SourceFile, line: usize, byte_range: &Range<usize>, col_pad: usize) {
    let line_range = src.line_range_bytes(line);

    let under_start = line_range.start.max(byte_range.start);
    let under_end = line_range.end.min(byte_range.end);

    let under_col = src.count_chars(line_range.start..under_start);
    let under_len = src.count_chars(under_start..under_end);

    print_signcolumn(col_pad, None);
    eprintln!("{:under_col$}{}", "", "^".repeat(under_len).bold().red());
}

fn print_signcolumn(col_pad: usize, num: Option<usize>) {
    match num {
        Some(n) => eprint!("{}", format!("{:col_pad$} | ", n).bold().blue()),
        None => eprint!("{}", format!("{:col_pad$} | ", "").bold().blue()),
    }
}

fn print_location_line(
    src_path: &str,
    src: &SourceFile,
    start_line: usize,
    byte_start: usize,
    col_pad: usize,
) {
    let start_range = src.line_range_bytes(start_line);

    eprintln!(
        "{:col_pad$}{} {}:{}:{}",
        "",
        "-->".bold().blue(),
        src_path,
        start_line + 1,
        byte_start - start_range.start + 1
    );
}

fn print_row_separator(col_pad: usize) {
    eprintln!("{:col_pad$} {}", "", "|".bold().blue());
}

fn print_parse_error(src_path: &str, src: &SourceFile, err: &ParseError) {
    let byte_range = compute_err_byte_range(src, err);

    let start_line = src.find_line(byte_range.start);
    let end_line = src.find_line(byte_range.end - 1);

    let col_pad = digit_count(end_line + 1, 10);

    print_error_header(src, err);
    print_location_line(src_path, src, start_line, byte_range.start, col_pad);
    print_row_separator(col_pad);

    for line in start_line..=end_line {
        print_src_line(src, line, col_pad);
        print_underline(src, line, &byte_range, col_pad);
    }
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let src = SourceFile::from_path(&args.input)?;

    let mut scanner = Scanner::new(src.contents());
    let mut parser = LangParser::new(&mut scanner);

    let program = parser.program().unwrap_or_else(|errors| {
        for e in errors {
            print_parse_error(&args.input, &src, &e);
        }

        std::process::exit(1);
    });

    println!("parse successful");
    // println!("{:#?}", program);
    Ok(())
}
