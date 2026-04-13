mod print;

use clap::Parser;

use uma_core::{core::SourceFile, parser::UmaParser, scanner::Scanner};

#[derive(clap::Parser)]
struct Args {
    filename: String,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let src = SourceFile::from_path(&args.filename)?;

    let mut scanner = Scanner::new(src.contents());
    let mut parser = UmaParser::new(&mut scanner);

    let unit = match parser.translation_unit() {
        Ok(file) => file,
        Err(errors) => {
            for e in errors {
                print::print_parse_error(&args.filename, &src, &e);
            }

            std::process::exit(1);
        }
    };

    println!("AST: {:#?}", unit);
    Ok(())
}
