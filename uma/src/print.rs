use std::ops::Range;

use crossterm::style::Stylize;
use uma_core::{core::SourceFile, parser::ParseError};

fn digit_count(n: usize, radix: u32) -> usize {
    (f32::log((n + 1) as f32, radix as f32)).ceil() as usize
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

fn print_src_line(src: &SourceFile, line: usize, pad: usize) {
    let line_range = src.line_to_byte_range(line);
    let line_str = &src.contents()[line_range.start..line_range.end - 1];

    print_signcolumn(pad, Some(line + 1));
    eprintln!("{}", line_str);
}

fn print_underline(src: &SourceFile, line: usize, byte_range: &Range<usize>, pad: usize) {
    let line_range = src.line_to_byte_range(line);

    let under_start = line_range.start.max(byte_range.start);
    let under_end = line_range.end.min(byte_range.end);

    let under_col = src.count_chars(line_range.start..under_start);
    let under_len = src.count_chars(under_start..under_end);

    print_signcolumn(pad, None);
    eprintln!("{:under_col$}{}", "", "^".repeat(under_len).bold().red());
}

fn print_signcolumn(pad: usize, num: Option<usize>) {
    match num {
        Some(n) => eprint!("{}", format!("{:pad$} | ", n).bold().blue()),
        None => eprint!("{}", format!("{:pad$} | ", "").bold().blue()),
    }
}

fn print_location_line(src_path: &str, line: usize, col: usize, pad: usize) {
    eprintln!(
        "{:pad$}{} {}:{}:{}",
        "",
        "-->".bold().blue(),
        src_path,
        line + 1,
        col + 1
    );
}

fn print_row_separator(pad: usize) {
    eprintln!("{:pad$} {}", "", "|".bold().blue());
}

pub fn print_parse_error(src_path: &str, src: &SourceFile, err: &ParseError) {
    let byte_range = compute_err_byte_range(src, err);

    let (start_line, start_col) = src.byte_to_line(byte_range.start);
    let (end_line, _) = src.byte_to_line(byte_range.end - 1);

    let pad = digit_count(end_line + 1, 10);

    print_error_header(src, err);
    print_location_line(src_path, start_line, start_col, pad);
    print_row_separator(pad);

    for line in start_line..=end_line {
        print_src_line(src, line, pad);
        print_underline(src, line, &byte_range, pad);
    }
}
