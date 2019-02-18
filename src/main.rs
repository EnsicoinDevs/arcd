mod cli;
mod data;
mod network;
use network::Server;

extern crate clap;
extern crate simplelog;
#[macro_use]
extern crate log;

use clap::Shell;
use clap::{App, Arg, SubCommand};
use std::io;
use std::str::FromStr;

fn main() {
    let matches = cli::build_cli().get_matches();
    match matches.subcommand() {
        ("completions", Some(sub_matches)) => {
            let shell = sub_matches.value_of("SHELL").unwrap();
            cli::build_cli().gen_completions_to(
                "another-rust-coin",
                shell.parse().unwrap(),
                &mut io::stdout(),
            );
        }
        (_, _) => unimplemented!(), // for brevity
    }
    //simplelog::TermLogger::init(simplelog::LevelFilter::Info, simplelog::Config::default());
    //let mut server = Server::new();
    //server.initiate(std::net::IpAddr::from_str("78.248.188.120").unwrap(), 4224);
    //server.listen();
}
