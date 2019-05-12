extern crate clap;
extern crate dirs;
extern crate ensicoin_messages;
extern crate ensicoin_serializer;
extern crate serde;
extern crate serde_json;
extern crate sled;
#[macro_use]
extern crate lazy_static;
use ensicoin_serializer::{Serialize, Sha256Result};

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
        ).arg(
            Arg::with_name("clean").long("clean").help("Cleans a previous install at the location")
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

    let mut blockchain_dir = std::path::PathBuf::new();
    blockchain_dir.push(data_dir);
    blockchain_dir.push("blockchain");

    if args.is_present("clean") {
        let mut utxo_dir = std::path::PathBuf::new();
        utxo_dir.push(data_dir);
        utxo_dir.push("utxo");

        let mut rev_dir = std::path::PathBuf::new();
        rev_dir.push(data_dir);
        rev_dir.push("reverse_chain");

        let mut spent_tx_dir = std::path::PathBuf::new();
        spent_tx_dir.push(data_dir);
        spent_tx_dir.push("spent_tx");

        let mut stats_dir = std::path::PathBuf::new();
        stats_dir.push(data_dir);
        stats_dir.push("stats");

        match std::fs::remove_dir_all(utxo_dir)
            .and(std::fs::remove_dir_all(rev_dir))
            .and(std::fs::remove_dir_all(spent_tx_dir))
            .and(std::fs::remove_dir_all(stats_dir))
            .and(std::fs::remove_dir_all(blockchain_dir.clone()))
            .and(std::fs::remove_file(settings.clone()))
        {
            Ok(_) => (),
            Err(e) => {
                eprintln!("Can't clean data_dir: {}", e);
                return;
            }
        };
    };
    if settings.is_file() {
        eprintln!("Can't bootstrap there, already setup");
        return;
    };
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
            bits: 0x1e00f000,
        },
        txs: Vec::new(),
    };
    let genesis_hash = genesis
        .double_hash()
        .iter()
        .map(|b| format!("{:x}", b))
        .fold(String::new(), |mut acc, mut v| {
            acc.push_str(&mut v);
            acc
        });
    println!("Genesis hash: {}", &genesis_hash);
    println!("Genesis header: {:?}", genesis.header.serialize().to_vec());
    let blockchain_db = match sled::Db::start_default(blockchain_dir) {
        Ok(db) => db,
        Err(e) => {
            eprintln!("Can't open blockchain database: {}", e);
            return;
        }
    };
    if let Err(e) = blockchain_db.set(genesis.double_hash().to_vec(), genesis.serialize().to_vec())
    {
        eprintln!("Could not insert genesis block: {}", e);
    }
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
