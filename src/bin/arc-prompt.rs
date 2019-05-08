mod constants;
mod data;
mod error;
use crate::data::intern_messages::PromptMessage;
pub use error::Error;

extern crate ensicoin_messages;
extern crate ensicoin_serializer;
extern crate rustyline;
#[macro_use]
extern crate ensicoin_serializer_derive;
#[macro_use]
extern crate log;
#[macro_use]
extern crate lazy_static;

extern crate serde_json;

use std::borrow::Cow;
use std::io::prelude::*;

use rustyline::completion::Completer;
use rustyline::config::OutputStreamType;
use rustyline::error::ReadlineError;
use rustyline::highlight::{Highlighter, MatchingBracketHighlighter};
use rustyline::hint::{Hinter, HistoryHinter};
use rustyline::{CompletionType, Config, Context, EditMode, Editor, Helper};

static COLORED_PROMPT: &'static str = "\x1b[1;32m>>\x1b[0m ";

static PROMPT: &'static str = ">> ";
struct MyHelper(MatchingBracketHighlighter, HistoryHinter);

lazy_static! {
    static ref MESSAGES: Vec<String> = vec![
        "Connect".to_string(),
        "Transaction".to_string(),
        "Help".to_string(),
        "Exit".to_string()
    ];
}

impl Completer for MyHelper {
    type Candidate = String;

    fn complete(
        &self,
        line: &str,
        _pos: usize,
        _ctx: &Context<'_>,
    ) -> Result<(usize, Vec<String>), ReadlineError> {
        Ok(
            if line.starts_with(|c: char| c.to_ascii_lowercase() == 'c') {
                (0, vec!["Connect".to_string()])
            } else if line.starts_with(|c: char| c.to_ascii_lowercase() == 't') {
                (0, vec!["Transaction".to_string()])
            } else if line.starts_with(|c: char| c.to_ascii_lowercase() == 'h') {
                (0, vec!["Help".to_string()])
            } else if line.starts_with(|c: char| c.to_ascii_lowercase() == 'e') {
                (0, vec!["Exit".to_string()])
            } else {
                (0, MESSAGES.to_vec())
            },
        )
    }
}

impl Hinter for MyHelper {
    fn hint(&self, line: &str, pos: usize, ctx: &Context<'_>) -> Option<String> {
        self.1.hint(line, pos, ctx)
    }
}

impl Highlighter for MyHelper {
    fn highlight_prompt<'p>(&self, prompt: &'p str) -> Cow<'p, str> {
        if prompt == PROMPT {
            Cow::Borrowed(COLORED_PROMPT)
        } else {
            Cow::Borrowed(prompt)
        }
    }

    fn highlight_hint<'h>(&self, hint: &'h str) -> Cow<'h, str> {
        Cow::Owned("\x1b[1m".to_owned() + hint + "\x1b[m")
    }

    fn highlight<'l>(&self, line: &'l str, pos: usize) -> Cow<'l, str> {
        self.0.highlight(line, pos)
    }

    fn highlight_char(&self, line: &str, pos: usize) -> bool {
        self.0.highlight_char(line, pos)
    }
}

impl Helper for MyHelper {}

fn send_message(socket: &mut std::net::TcpStream, message: PromptMessage) {
    let serialized = serde_json::to_vec(&message).expect("Could not serialize message");
    let len = serialized.len() as u32;
    socket
        .write(&len.to_be_bytes())
        .expect("Error sending message");
    socket.write(&serialized).expect("Error sending message");
}

fn main() {
    let mut socket =
        match std::net::TcpStream::connect(format!("127.0.0.1:{}", constants::DEFAULT_PROMPT)) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Can't connect to daemon: {}", e);
                return;
            }
        };

    let config = Config::builder()
        .history_ignore_space(true)
        .completion_type(CompletionType::List)
        .edit_mode(EditMode::Vi)
        .output_stream(OutputStreamType::Stdout)
        .build();
    let h = MyHelper(MatchingBracketHighlighter::new(), HistoryHinter {});
    let mut rl = Editor::with_config(config);
    rl.set_helper(Some(h));
    loop {
        let readline = rl.readline(PROMPT);
        match readline {
            Ok(line) => {
                rl.add_history_entry(line.as_ref());
                let trim_line = line.trim();
                if line.starts_with("Connect") {
                    let v: Vec<&str> = line.split(' ').collect();
                    if v.len() != 2 {
                        eprintln!("Expected 1 argument, got {}", v.len() - 1);
                        continue;
                    }
                    let address: std::net::SocketAddr = match v[1].parse() {
                        Err(e) => {
                            eprintln!("Invalid address: {}", e);
                            continue;
                        }
                        Ok(a) => a,
                    };
                    send_message(&mut socket, PromptMessage::Connect(address));
                } else if trim_line == "Exit" || trim_line == "exit" {
                    println!("Bye !");
                    send_message(&mut socket, PromptMessage::Disconnect);
                    break;
                } else if trim_line == "Help" || trim_line == "help" {
                    println!("Commands :");
                    println!("\tHelp: prints this help");
                    println!("\tExit: closes the prompt");
                    println!("\tConnect address:port: Creates a connection to the specified node");
                    println!("\tTransaction {{json}}: Registers a Transaction (as json)");
                } else {
                    eprintln!("Invalid command: {}", line);
                }
            }
            Err(ReadlineError::Interrupted) => println!("CTRL-C"),
            Err(ReadlineError::Eof) => {
                send_message(&mut socket, PromptMessage::Disconnect);
                println!("Bye !");
                break;
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                break;
            }
        }
    }
}
