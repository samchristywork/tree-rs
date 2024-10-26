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
use render::render;

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
    fn highlight(s: &str, re: &Regex) -> String {
        let mut highlighted = String::new();
        let mut last_end = 0;

        for mat in re.find_iter(s) {
            highlighted.push_str(&s[last_end..mat.start()]);
            highlighted.push_str(&format!(
                "{INVERT}{}{}",
                &s[mat.start()..mat.end()],
                UNINVERT
            ));
            last_end = mat.end();
        }

        highlighted.push_str(&s[last_end..]);
        highlighted
    }

    fn to_string(&self, re: &Regex, n: usize, selected: bool) -> String {
        if n < self.first_part.len() {
            return self.first_part[..n].to_string();
        }

        let remaining = n - self.first_part.len();
        let s = if remaining > self.last_part.len() {
            &self.last_part.clone()
        } else {
            &self.last_part[..remaining].to_string()
        };

        if selected {
            format!(
                ">{}{INVERT}{}{}{NORMAL}{UNINVERT}",
                self.first_part, self.color, s
            )
        } else {
            let last_part = Self::highlight(s, re);
            format!(
                " {}{}{last_part}{NORMAL}{UNINVERT}",
                self.first_part, self.color
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
    let mut last_working_pattern = String::new();
    let mut scroll = 0;
    let mut cursor_pos = 0;
    let mut selected = 0;
    loop {
        let p = if case_sensitive {
            format!("(?-s:{pattern})")
        } else {
            format!("(?i:{pattern})")
        };

        let mut pattern_is_valid = false;
        let re = match Regex::new(&p) {
            Ok(re) => {
                pattern_is_valid = true;
                last_working_pattern.clone_from(&p);
                re
            }
            Err(_) => Regex::new(&last_working_pattern).expect("Failed to create regex"),
        };

        mark_matched_nodes(&mut directory_tree, &re);

        render(
            &directory_tree,
            &pattern,
            style,
            &mut scroll,
            cursor_pos,
            &re,
            pattern_is_valid,
            &mut selected,
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
