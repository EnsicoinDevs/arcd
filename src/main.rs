mod bootstrap;
mod constants;
mod data;
mod error;
mod manager;
mod network;
pub mod utils;
pub use error::Error;

use network::Server;

extern crate bytes;

extern crate ron;
extern crate serde;

extern crate futures;
extern crate tokio;
extern crate tokio_timer;

extern crate structopt;
#[macro_use]
extern crate log;
extern crate simplelog;

extern crate dirs;

extern crate sled;

extern crate ensicoin_serializer;
//#[macro_use]
extern crate ensicoin_messages;
#[macro_use]
extern crate ensicoin_serializer_derive;

extern crate generic_array;
extern crate num_bigint;
extern crate ripemd160;
extern crate secp256k1;
extern crate sha2;
extern crate typenum;

extern crate tower_grpc;
extern crate tower_hyper;

extern crate tokio_bus;

extern crate tokio_signal;

extern crate reqwest;

use std::fs::File;
use std::io::prelude::*;
use std::str::FromStr;
use structopt::StructOpt;

#[derive(Debug)]
enum LogLevel {
    Debug,
    Error,
    Trace,
    Info,
}

impl FromStr for LogLevel {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let lower: &str = &s.to_ascii_lowercase();
        match lower {
            "debug" => Ok(Self::Debug),
            "error" => Ok(Self::Error),
            "trace" => Ok(Self::Trace),
            "info" => Ok(Self::Info),
            s => Err(format!("Unknown log level: {}", s)),
        }
    }
}

#[derive(StructOpt)]
#[structopt(name = "arcd", about = "An ensicoin node in rust")]
struct Config {
    #[structopt(short, long, default_value = "info")]
    // Sets the log level (can be "trace","debug", "info", "error")
    pub log: LogLevel,
    #[structopt(long)]
    // Cleans all data from previous executions
    pub clean: bool,
    #[structopt(short, long)]
    // Saves the parameters as defaults for the next execution
    pub save: bool,
    #[structopt(long)]
    // Uses a saved config
    pub use_config: bool,
    #[structopt(flatten)]
    pub server_config: ServerConfig,
}

#[derive(StructOpt, serde::Serialize, serde::Deserialize)]
pub struct ServerConfig {
    #[structopt(short = "c", long = "connections", default_value = "")]
    // Sets the maximum number of connections
    pub max_connections: u64,
    #[structopt(long)]
    // Changes the default directory
    pub data_dir: Option<std::path::PathBuf>,
    #[structopt(short, long, default_value = "4224")]
    // Port listening for connections
    pub port: u16,
    #[structopt(long)]
    // RON credentials for matrix
    pub matrix_creds: Option<std::path::PathBuf>,
    #[structopt(long, short, default_value = "4225")]
    // Port listening for gRPC requests
    pub grpc_port: u16,
    #[structopt(long)]
    // Restrict gRPC requests to localhost
    pub grpc_localhost: bool,
}

fn main() {
    let mut config = Config::from_args();

    let log_level = match config.log {
        LogLevel::Debug => simplelog::LevelFilter::Debug,
        LogLevel::Info => simplelog::LevelFilter::Info,
        LogLevel::Error => simplelog::LevelFilter::Error,
        LogLevel::Trace => simplelog::LevelFilter::Trace,
    };
    simplelog::TermLogger::init(
        log_level,
        simplelog::Config::default(),
        simplelog::TerminalMode::Mixed,
    )
    .unwrap();

    if config.server_config.data_dir.is_none() {
        let mut path = dirs::data_dir().unwrap();
        path.push(r"another-rust-coin");
        config.server_config.data_dir = Some(path);
    };
    let data_dir = config.server_config.data_dir.clone().unwrap();

    std::fs::create_dir_all(&data_dir).unwrap();
    if let Some(s) = data_dir.to_str() {
        info!("Using {} as data directory", s);
    }

    if config.clean {
        if let Err(e) = bootstrap::clean(data_dir.clone()) {
            eprintln!("Could not clean directory: {}", e);
            return;
        }
    };
    let mut settings_path = data_dir.clone();
    settings_path.push("settings.ron");
    let mut settings_file = match File::open(settings_path) {
        Ok(f) => f,
        Err(e) => {
            warn!("Settings file could not be opened: {}", e);
            return;
        }
    };
    if config.save {
        let mut pretty = ron::ser::PrettyConfig::default();
        pretty.depth_limit = 4;
        pretty.separate_tuple_members = true;
        let config_string = match ron::ser::to_string_pretty(&config.server_config, pretty) {
            Ok(s) => s,
            Err(e) => {
                warn!("Could not serialize config: {}", e);
                return;
            }
        };
        if let Err(e) = settings_file.write(config_string.as_bytes()) {
            warn!("Could not write config file: {}", e)
        }
    };
    if config.use_config {
        match ron::de::from_reader(settings_file) {
            Ok(s) => config.server_config = s,
            Err(e) => warn!("Could not use config file: {}", e),
        }
    }

    if let Err(e) = Server::run(config.server_config) {
        error!("Server could not be launched: {}", e)
    };
}
