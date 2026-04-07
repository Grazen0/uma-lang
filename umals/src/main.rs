use std::io;

use crate::server::Server;

mod jsonrpc;
mod server;
mod structs;

fn main() -> anyhow::Result<()> {
    let stdin = io::stdin().lock();
    let stdout = io::stdout().lock();
    let mut server = Server::new(stdin, stdout);
    server.run()
}
