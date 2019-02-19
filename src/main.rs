mod cli;
mod data;
mod network;
use network::Server;

#[macro_use]
extern crate clap;
extern crate simplelog;
#[macro_use]
extern crate log;

use std::io;
use std::str::FromStr;

fn main() {
    let matches = cli::build_cli().get_matches();

    let log_level = if matches.is_present("verbose") {
        simplelog::LevelFilter::Trace
    } else if matches.is_present("error") {
        simplelog::LevelFilter::Error
    } else {
        simplelog::LevelFilter::Info
    };
    simplelog::TermLogger::init(log_level, simplelog::Config::default()).unwrap();

    let listen_port = matches.value_of("port").unwrap().parse().unwrap();

    match matches.subcommand() {
        ("completions", Some(sub_matches)) => {
            let shell = sub_matches.value_of("SHELL").unwrap();
            cli::build_cli().gen_completions_to(
                "another-rust-coin",
                shell.parse().unwrap(),
                &mut io::stdout(),
            );
        }
        ("initiate", Some(sub_matches)) => {
            let mut server = Server::new(listen_port);
            server.initiate(
                std::net::IpAddr::from_str(sub_matches.value_of("HOST_IP").unwrap()).unwrap(),
                sub_matches.value_of("PORT").unwrap().parse().unwrap(),
            );
            server.listen();
        }
        ("", _) => {
            let mut server = Server::new(listen_port);
            server.listen();
        }
        (_, _) => (),
    };
}
