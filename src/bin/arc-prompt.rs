mod constants;
mod data;
mod error;
pub use error::Error;

extern crate ensicoin_serializer;
extern crate rustyline;
#[macro_use]
extern crate ensicoin_serializer_derive;
#[macro_use]
extern crate log;
#[macro_use]
extern crate lazy_static;

use std::borrow::Cow;

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
    static ref MESSAGES: Vec<String> = vec!["Connect".to_string(), "NewTransaction".to_string()];
}

impl Completer for MyHelper {
    type Candidate = String;

    fn complete(
        &self,
        line: &str,
        _pos: usize,
        _ctx: &Context<'_>,
    ) -> Result<(usize, Vec<String>), ReadlineError> {
        Ok(if line.starts_with('C') {
            (0, vec!["Connect".to_string()])
        } else if line.starts_with('N') {
            (0, vec!["NewTransaction".to_string()])
        } else {
            (0, MESSAGES.to_vec())
        })
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

fn main() {
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
                println!("Line: {}", line)
            }
            Err(ReadlineError::Interrupted) => println!("CTRL-C"),
            Err(ReadlineError::Eof) => {
                break;
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                break;
            }
        }
    }
}
