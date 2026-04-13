use std::{
    io::{self, BufReader},
    net::TcpListener,
};

use clap::Parser;

use crate::server::Server;

mod jsonrpc;
mod server;
mod structs;

#[derive(Debug, Clone, Parser)]
struct Args {
    #[arg(long, default_value = "127.0.0.1")]
    host: String,

    #[arg(short, long, default_value_t = 8080)]
    port: u16,

    /// Use standard input/output instead of TCP
    #[arg(long)]
    stdio: bool,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    if args.stdio {
        let stdin = io::stdin().lock();
        let stdout = io::stdout().lock();

        let mut server = Server::new(stdin, stdout);
        server.run()
    } else {
        let addr = format!("{}:{}", args.host, args.port);
        let listener = TcpListener::bind(&addr)?;

        println!("Listening at {addr}...");
        let Some(stream) = listener.incoming().next().transpose()? else {
            return Ok(());
        };

        let reader = BufReader::new(stream.try_clone()?);
        let writer = stream;

        let mut server = Server::new(reader, writer);
        server.run()
    }
}
