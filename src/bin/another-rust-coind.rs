mod bootstrap;
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
extern crate simplelog;
#[macro_use]
extern crate lazy_static;

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

extern crate tokio_bus;

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
            if matches.is_present("clean") {
                if let Err(e) = bootstrap::clean(data_dir.clone()) {
                    eprintln!("Could not clean directory: {}", e);
                    return;
                }
            };
            let mut settings_path = data_dir.clone();
            settings_path.push("settings.json");
            let settings = match std::fs::File::open(settings_path.clone()) {
                Ok(s) => s,
                Err(e) => {
                    if e.kind() == std::io::ErrorKind::NotFound {
                        if let Err(e) = bootstrap::bootstrap(&data_dir) {
                            eprintln!("Error bootstraping: {}", e);
                        }
                    } else {
                        eprintln!("Cannot read settings file: {}", e);
                    }
                    return;
                }
            };
            let mut defaults: std::collections::HashMap<String, String> =
                serde_json::from_reader(settings).expect("Could no read settings file");

            let save = matches.is_present("save");
            let listen_port = if matches.is_present("port") {
                let val = matches.value_of("port").unwrap();
                if save {
                    defaults.insert(String::from("port"), String::from(val));
                };
                val
            } else if defaults.contains_key("port") {
                defaults.get("port").unwrap()
            } else {
                constants::DEFAULT_PORT
            }
            .parse()
            .unwrap();
            let prompt_port = if matches.is_present("prompt_port") {
                let val = matches.value_of("prompt_port").unwrap();
                if save {
                    defaults.insert(String::from("prompt_port"), String::from(val));
                };
                val
            } else if defaults.contains_key("prompt_port") {
                defaults.get("prompt_port").unwrap()
            } else {
                constants::DEFAULT_PROMPT
            }
            .parse()
            .unwrap();
            let grpc_port = if matches.is_present("grpc_port") {
                let val = matches.value_of("grpc_port").unwrap();
                if save {
                    defaults.insert(String::from("grpc_port"), String::from(val));
                };
                val
            } else if defaults.contains_key("grpc_port") {
                defaults.get("grpc_port").unwrap()
            } else {
                constants::DEFAULT_GRPC_PORT
            }
            .parse()
            .unwrap();
            let max_conn = if matches.is_present("max connections") {
                let val = matches.value_of("max connections").unwrap();
                if save {
                    defaults.insert(String::from("max connections"), String::from(val));
                };
                val
            } else if defaults.contains_key("max connections") {
                defaults.get("max connections").unwrap()
            } else {
                constants::DEFAULT_MAX_CONN
            }
            .parse()
            .unwrap();
            let grpc_localhost =
                matches.is_present("grpc_localhost") || defaults.contains_key("grpc_localhost");
            if save {
                let settings = match std::fs::File::open(settings_path) {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("Error saving settings: {}", e);
                        return;
                    }
                };
                serde_json::to_writer(settings, &defaults).unwrap();
            }
            Server::run(
                max_conn,
                &data_dir,
                listen_port,
                prompt_port,
                grpc_port,
                grpc_localhost,
            );
        }
        (_, _) => (),
    };
}
