mod print;

use clap::Parser;

use uma_core::{core::SourceFile, interpreter::Interpreter, parser::UmaParser, scanner::Scanner};

#[derive(clap::Parser)]
struct Args {
    filename: String,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let src = SourceFile::from_path(&args.filename)?;

    let mut scanner = Scanner::new(&src);
    let mut parser = UmaParser::new(&mut scanner);

    let program = match parser.program_to_end() {
        Ok(program) => program,
        Err(errors) => {
            for e in errors {
                print::print_parse_error(&args.filename, &src, &e);
            }

            std::process::exit(1);
        }
    };

    let mut executor = Interpreter::new(&program)?;
    executor.execute()?;
    Ok(())
}
