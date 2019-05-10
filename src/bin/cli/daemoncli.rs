use crate::constants::{DEFAULT_PORT, DEFAULT_PROMPT, IP};
use clap::{App, Arg, SubCommand};

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

fn is_ip(v: String) -> Result<(), String> {
    let ip: Result<std::net::IpAddr, std::net::AddrParseError> = v.parse();
    match ip {
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

pub fn build_cli() -> App<'static, 'static> {
    app_from_crate!()
        .arg(
            Arg::with_name("verbose")
                .short("v")
                .long("verbose")
                .help("Set the output as verbose"),
        )
        .arg(
            Arg::with_name("error")
                .short("e")
                .long("error")
                .help("Suppress all output execpt error")
                .conflicts_with("verbose"),
        )
        .arg(
            Arg::with_name("port")
                .short("p")
                .long("port")
                .help("Set the listening port")
                .default_value(DEFAULT_PORT)
                .validator(is_port),
        )
        .arg(
            Arg::with_name("prompt_port")
                .long("prompt")
                .default_value(DEFAULT_PROMPT)
                .validator(is_port)
                .help("Port to connect the prompt to"),
        )
        .arg(
            Arg::with_name("grpc_port")
                .long("grpc")
                .short("g")
                .help("Listening port for gRPC")
                .takes_value(true)
                .default_value(crate::constants::DEFAULT_GRPC_PORT)
                .validator(is_port),
        )
        .arg(
            Arg::with_name("grpc_localhost")
                .long("grpc-localhost")
                .short("l")
                .help("Restrict grpc to localhost"),
        )
        .arg(
            Arg::with_name("max connections")
                .short("c")
                .long("connections")
                .help("Specifies the maximum number of connections")
                .default_value(crate::constants::DEFAULT_MAX_CONN)
                .validator(is_u64),
        )
        .arg(
            Arg::with_name("datadir")
                .short("d")
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
