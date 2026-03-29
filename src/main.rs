mod core;
mod parser;
mod scanner;

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

fn print_parse_error(src_path: &str, src: &SourceFile, err: &ParseError) {
    eprintln!(
        "{}",
        format!("{}: {}", "error".red(), err.fmt_with_src(src.contents())).bold()
    );

    let byte_range = err.byte_range().unwrap_or_else(|| {
        let end = src.contents().len();
        end..end + 1
    });

    let start_line = src.find_line(byte_range.start);
    let start_range = src.line_range_bytes(start_line);
    let end_line = src.find_line(byte_range.end - 1);

    let col_pad = digit_count(end_line + 1, 10);

    eprintln!(
        "{:col_pad$}{} {}:{}:{}",
        "",
        "-->".bold().blue(),
        src_path,
        start_line + 1,
        byte_range.start - start_range.start + 1
    );

    eprintln!("{:col_pad$} {}", "", "|".bold().blue());

    for line in start_line..=end_line {
        let line_range = src.line_range_bytes(line);
        let line_str = &src.contents()[line_range.start..line_range.end - 1];

        eprintln!(
            "{} {}",
            format!("{:col_pad$} |", line + 1).bold().blue(),
            line_str
        );

        // TODO: refactor
        let (s, l) = if line == start_line {
            let s = src.contents()[line_range.start..byte_range.clone().start]
                .chars()
                .count();

            (
                s,
                usize::min(
                    src.contents()[byte_range.clone()].chars().count(),
                    line_str.chars().count() + 1 - s,
                ),
            )
        } else if line == end_line {
            (
                0,
                src.contents()[line_range.start..byte_range.clone().end]
                    .chars()
                    .count(),
            )
        } else {
            (0, line_str.chars().count() + 1)
        };

        eprintln!(
            "{:col_pad$} {} {:s$}{}",
            "",
            "|".bold().blue(),
            "",
            "^".repeat(l).bold().red()
        );
    }
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let src = SourceFile::from_path(&args.input)?;

    let mut scanner = Scanner::new(src.contents());
    let mut parser = LangParser::new(&mut scanner);

    let program = parser.program().unwrap_or_else(|e| {
        print_parse_error(&args.input, &src, &e);
        std::process::exit(1);
    });

    parser.ensure_done()?;

    println!("{:#?}", program);
    Ok(())
}
