extern crate clap;
extern crate dirs;
extern crate ensicoin_messages;
extern crate ensicoin_serializer;
extern crate serde;
extern crate serde_json;
extern crate sled;
#[macro_use]
extern crate lazy_static;
use ensicoin_serializer::Sha256Result;

use clap::{App, Arg, SubCommand};
use std::fs;

fn is_port(v: String) -> Result<(), String> {
    let n: Result<u16, std::num::ParseIntError> = v.parse();
    match n {
        Ok(i) if i > 1024 => Ok(()),
        Ok(_) => Err("port must be at least 1025".to_string()),
        Err(e) => Err(e.to_string()),
    }
}

fn is_u64(v: String) -> Result<(), String> {
    let n: Result<u64, std::num::ParseIntError> = v.parse();
    match n {
        Ok(_) => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

lazy_static! {
    static ref DATA_DIR: std::path::PathBuf = {
        let mut path = dirs::data_dir().unwrap();
        path.push(r"another-rust-coin");
        path
    };
}

fn build_cli() -> App<'static, 'static> {
    App::new("arc-bootstrap").about("Sets up default values for diverse parameters of the daemon, and setup the default state").author("Quentin Boyer <qbsecond@gmail.com>").arg(
            Arg::with_name("port")
                .long("port")
                .help("Set the listening port").takes_value(true)
                .validator(is_port),
        )
        .arg(
            Arg::with_name("prompt_port")
                .long("prompt_port")
                .validator(is_port).takes_value(true)
                .help("Port to connect the prompt to"),
        )
        .arg(
            Arg::with_name("grpc_port")
                .long("grpc_port")
                .help("Listening port for gRPC")
                .takes_value(true)
                .validator(is_port),
        )
        .arg(
            Arg::with_name("grpc_localhost")
                .long("grpc-localhost")
                .help("Restrict grpc to localhost"),
        )
        .arg(
            Arg::with_name("max connections")
                .long("max-connections").takes_value(true)
                .help("Specifies the maximum number of connections")
                .validator(is_u64),
        )
        .arg(
            Arg::with_name("datadir")
                .long("data")
                .help("Data root folder")
                .default_value(DATA_DIR.to_str().unwrap()),
        )
        .subcommand(
            SubCommand::with_name("completions")
                .about("Generates completion scripts for your shell")
                .arg(
                    Arg::with_name("SHELL")
                        .required(true)
                        .possible_values(&["bash", "fish", "zsh"])
                        .help("The shell to generate the script for"),
                ),
        )
}

fn bootstrap(args: clap::ArgMatches) {
    let data_dir = args.value_of("datadir").unwrap();
    let mut settings = std::path::PathBuf::from(data_dir);
    settings.push("settings.json");
    let settings = match fs::File::create(settings) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Can't bootstrap at that location: {}", e);
            return;
        }
    };

    let mut defaults = std::collections::HashMap::new();
    if args.is_present("port") {
        defaults.insert("port", args.value_of("port").unwrap());
    };
    if args.is_present("prompt_port") {
        defaults.insert("prompt_port", args.value_of("prompt_port").unwrap());
    };
    if args.is_present("grpc_localhost") {
        defaults.insert("grpc_localhost", "true");
    };
    if args.is_present("grpc_port") {
        defaults.insert("grpc_port", args.value_of("grpc_port").unwrap());
    };
    if args.is_present("max connections") {
        defaults.insert("max connections", args.value_of("max connections").unwrap());
    };
    serde_json::to_writer(settings, &defaults).unwrap();

    let genesis = ensicoin_messages::resource::Block {
        header: ensicoin_messages::resource::BlockHeader {
            version: 0,
            flags: vec!["ici cest limag".to_string()],
            prev_block: Sha256Result::from([0; 32]),
            merkle_root: Sha256Result::from([0; 32]),
            timestamp: 1566862920,
            nonce: 42,
            height: 0,
            bits: 1,
        },
        txs: Vec::new(),
    };
}

fn main() {
    let matches = build_cli().get_matches();

    match matches.subcommand() {
        ("completions", Some(sub_matches)) => {
            let shell = sub_matches.value_of("SHELL").unwrap();
            build_cli().gen_completions_to(
                "arc-bootstrap",
                shell.parse().unwrap(),
                &mut std::io::stdout(),
            );
        }
        (_, _) => bootstrap(matches),
    }
}
