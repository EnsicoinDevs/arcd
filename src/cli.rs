use clap::{App, Arg, SubCommand};

fn is_port(v: String) -> Result<(), String> {
    let n: Result<u16, std::num::ParseIntError> = v.parse();
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

pub fn build_cli() -> App<'static, 'static> {
    App::new("another-rust-coin")
        .version("0.0.1")
        .about("A rust node for the ensicoin protocol")
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
                .default_value("4224")
                .validator(is_port),
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
        .subcommand(
            SubCommand::with_name("initiate")
                .about("initiate a connection with a host and listens")
                .arg(
                    Arg::with_name("HOST_IP")
                        .required(true)
                        .help("The ip of the remote host")
                        .validator(is_ip),
                )
                .arg(
                    Arg::with_name("PORT")
                        .help("Remote port")
                        .default_value("4224")
                        .validator(is_port),
                ),
        )
}
