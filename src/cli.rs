use clap::{App, Arg, SubCommand};

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
                .default_value("4224"),
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
