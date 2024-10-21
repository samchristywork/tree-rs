use clap::Parser;
use clap::ValueEnum;
use regex::Regex;
use std::io;
use std::path::PathBuf;
use termion::raw::IntoRawMode;

mod generate;
mod input;
mod mark;
mod render;

use generate::build_directory_tree;
use input::handle_input;
use mark::mark_matched_nodes;
use render::draw;

#[derive(ValueEnum, Clone, Debug)]
enum Style {
    Compact,
    Full,
}

pub enum Direction {
    Left,
    Right,
}

pub enum Navigation {
    PageUp,
    PageDown,
    Home,
    End,
}

pub enum Event {
    Key(char),
    Direction(Direction),
    Navigation(Navigation),
    Backspace,
    Clear,
    Enter,
    Exit,
}

const RED: &str = "\x1B[31m";
const NORMAL: &str = "\x1B[0m";
const INVERT: &str = "\x1B[7m";
const UNINVERT: &str = "\x1B[27m";

const ALTERNATE_SCREEN: &str = "\x1B[?1049h";
const NORMAL_SCREEN: &str = "\x1B[?1049l";

#[derive(Parser, Debug)]
#[clap(author = "Sam Christy", version = "1.0", about = "An interactive program for exploring directory trees.", long_about = None)]
struct Args {
    /// Directory to render
    #[clap(short, long, default_value = ".")]
    directory: String,

    /// Use case-sensitive regex matching (default is case-insensitive)
    #[clap(short, long)]
    case_sensitive: bool,

    /// Style to use for rendering (compact or full)
    #[clap(short, long, default_value = "full")]
    style: String,
}

struct Line {
    first_part: String,
    last_part: String,
    color: String,
}

impl Line {
    fn highlight(&self, s: &str) -> String {
        let mut highlighted = String::new();
        for c in s.chars() {
            if c == 'a' {
                highlighted.push_str(&format!("{INVERT}{c}{UNINVERT}"));
            } else {
                highlighted.push(c);
            }
        }

        highlighted
    }

    fn length(&self) -> usize {
        self.first_part.len() + self.last_part.len()
    }

    fn to_string(&self, n: usize) -> String {
        if self.length() <= n {
            format!(
                "{}{}{}{NORMAL}",
                self.first_part,
                self.color,
                self.highlight(&self.last_part)
            )
        } else if self.first_part.len() < n {
            format!(
                "{}{}{}{NORMAL}",
                self.first_part,
                self.color,
                self.highlight(&self.last_part[..n - self.first_part.len()])
            )
        } else {
            format!(
                "{}{}{}{NORMAL}",
                &self.first_part[..n],
                self.color,
                self.highlight(&self.last_part)
            )
        }
    }
}

struct DirectoryNode {
    path: PathBuf,
    children: Vec<DirectoryNode>,
    matched: bool,
    color: String,
    error: Option<io::Error>,
}

fn main_loop(
    directory: &str,
    style: &Style,
    case_sensitive: bool,
) -> Result<Option<String>, String> {
    let term = termion::get_tty().expect("Failed to get terminal");
    let _raw_term = term.into_raw_mode().expect("Failed to enter raw mode");
    let mut directory_tree = build_directory_tree(directory);

    let mut pattern = String::new();
    let mut scroll = 0;
    let mut cursor_pos = 0;
    loop {
        let screen_size = termion::terminal_size().unwrap_or((80, 24));

        let p = if case_sensitive {
            format!("(?-s:{pattern})")
        } else {
            format!("(?i:{pattern})")
        };

        let re = match Regex::new(&p) {
            Ok(re) => re,
            Err(e) => return Err(format!("Error: Invalid regex pattern '{pattern}': {e}")),
        };

        mark_matched_nodes(&mut directory_tree, &re);

        draw(
            &directory_tree,
            &pattern,
            style,
            &mut scroll,
            screen_size,
            cursor_pos,
        );

        match handle_input(&mut pattern, &mut cursor_pos, &mut scroll) {
            Some(p) if p.is_empty() => {
                return Ok(None);
            }
            Some(p) => {
                return Ok(Some(p));
            }
            None => {}
        };
    }
}

fn main() {
    let args = Args::parse();
    let style = match args.style.as_str() {
        "compact" => Style::Compact,
        _ => Style::Full,
    };

    print!("{ALTERNATE_SCREEN}");
    let result = match main_loop(&args.directory, &style, args.case_sensitive) {
        Ok(result) => {
            print!("{NORMAL_SCREEN}");
            result
        }
        Err(e) => {
            print!("{NORMAL_SCREEN}");
            print!("{RED}{e}{NORMAL}");
            None
        }
    };

    if let Some(pattern) = result {
        println!("{pattern}");
    }
}
