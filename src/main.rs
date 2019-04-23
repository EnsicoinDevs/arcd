mod cli;
mod constants;
mod data;
mod manager;
mod network;

use network::Server;

extern crate bytes;

extern crate futures;
extern crate tokio;
extern crate tokio_timer;

#[macro_use]
extern crate clap;
#[macro_use]
extern crate log;
#[macro_use]
extern crate lazy_static;

extern crate simplelog;

extern crate dirs;

extern crate sled;

extern crate ensicoin_serializer;
#[macro_use]
extern crate ensicoin_serializer_derive;

extern crate generic_array;
extern crate ripemd160;
extern crate secp256k1;
extern crate sha2;
extern crate typenum;

use crate::data::message::Message;
use std::io;
use std::str::FromStr;

extern crate cpuprofiler;

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

    let data_dir = std::path::PathBuf::from(matches.value_of("datadir").unwrap());
    std::fs::create_dir_all(&data_dir).unwrap();

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
            let server = Server::new(
                matches
                    .value_of("max connections")
                    .unwrap()
                    .parse()
                    .unwrap(),
                &data_dir,
                listen_port,
            );
            let sender = server.get_sender();
            tokio::run(server);
            crate::network::Connection::initiate(
                std::net::IpAddr::from_str(sub_matches.value_of("HOST_IP").unwrap()).unwrap(),
                sub_matches.value_of("PORT").unwrap().parse().unwrap(),
                sender,
            );
        }
        ("", _) => {
            let server = Server::new(
                matches
                    .value_of("max connections")
                    .unwrap()
                    .parse()
                    .unwrap(),
                &data_dir,
                listen_port,
            );
            tokio::run(server);
        }
        (_, _) => (),
    };
}
