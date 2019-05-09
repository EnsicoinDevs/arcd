mod cli;
mod constants;
mod data;
mod error;
mod manager;
mod network;
pub use cli::daemoncli;
pub use error::Error;

use network::Server;

extern crate bytes;

extern crate serde;
extern crate tokio_serde_json;

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
extern crate ensicoin_messages;

extern crate generic_array;
extern crate ripemd160;
extern crate secp256k1;
extern crate sha2;
extern crate typenum;

extern crate tower_grpc;
extern crate tower_hyper;

use std::io;

fn main() {
    let matches = daemoncli::build_cli().get_matches();

    let log_level = if matches.is_present("verbose") {
        simplelog::LevelFilter::Trace
    } else if matches.is_present("error") {
        simplelog::LevelFilter::Error
    } else {
        simplelog::LevelFilter::Info
    };
    simplelog::TermLogger::init(log_level, simplelog::Config::default()).unwrap();

    let listen_port = matches.value_of("port").unwrap().parse().unwrap();
    let prompt_port = matches.value_of("prompt_port").unwrap().parse().unwrap();

    let data_dir = std::path::PathBuf::from(matches.value_of("datadir").unwrap());
    std::fs::create_dir_all(&data_dir).unwrap();

    match matches.subcommand() {
        ("completions", Some(sub_matches)) => {
            let shell = sub_matches.value_of("SHELL").unwrap();
            daemoncli::build_cli().gen_completions_to(
                "another-rust-coin",
                shell.parse().unwrap(),
                &mut io::stdout(),
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
                prompt_port,
            );
            tokio::run(server);
        }
        (_, _) => (),
    };
}
