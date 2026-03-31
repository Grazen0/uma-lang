mod core;
mod parser;
mod pretty;
mod scanner;

use std::ops::Range;

use clap::Parser;
use crossterm::style::Stylize;

use crate::{
    core::SourceText,
    parser::{ParseError, UmaParser},
    scanner::Scanner,
};

fn digit_count(n: usize, radix: u32) -> usize {
    (f32::log((n + 1) as f32, radix as f32)).ceil() as usize
}

#[derive(clap::Parser)]
struct Args {
    /// Input file
    input: String,

    /// Output the file formatted
    #[arg(short, long)]
    format: bool,
}

fn print_error_header(src: &SourceText, err: &ParseError) {
    eprintln!(
        "{}",
        format!("{}: {}", "error".red(), err.fmt_with_src(src.contents())).bold()
    );
}

fn compute_err_byte_range(src: &SourceText, err: &ParseError) -> Range<usize> {
    err.byte_range().unwrap_or_else(|| {
        let end = src.contents().len();
        end - 1..end
    })
}

fn print_src_line(src: &SourceText, line: usize, col_pad: usize) {
    let line_range = src.line_range_bytes(line);
    let line_str = &src.contents()[line_range.start..line_range.end - 1];

    print_signcolumn(col_pad, Some(line + 1));
    eprintln!("{}", line_str);
}

fn print_underline(src: &SourceText, line: usize, byte_range: &Range<usize>, col_pad: usize) {
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
    src: &SourceText,
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

fn print_parse_error(src_path: &str, src: &SourceText, err: &ParseError) {
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
    let src = SourceText::from_path(&args.input)?;

    let mut scanner = Scanner::new(src.contents());
    let mut parser = UmaParser::new(&mut scanner);

    let source_file = match parser.source_file() {
        Ok(file) => file,
        Err(errors) => {
            for e in errors {
                print_parse_error(&args.input, &src, &e);
            }

            std::process::exit(1);
        }
    };

    if args.format {
        println!("{}", source_file.pretty_print());
    }

    Ok(())
}
