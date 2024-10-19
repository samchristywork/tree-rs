use clap::Parser;
use clap::ValueEnum;
use regex::Regex;
use std::fs;
use std::io;
use std::path::Path;
use std::path::PathBuf;
use termion::raw::IntoRawMode;

mod input;
mod render;

use input::get_input;
use render::draw;

#[derive(ValueEnum, Clone, Debug)]
enum Style {
    Compact,
    Full,
}

pub enum Direction {
    Up,
    Down,
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

const CYAN: &str = "\x1B[36m";
const MAGENTA: &str = "\x1B[35m";
const YELLOW: &str = "\x1B[33m";
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
            if c=='a' {
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
            format!("{}{}{}{NORMAL}", self.first_part, self.color, self.highlight(&self.last_part))
        } else if self.first_part.len() < n {
            format!(
                "{}{}{}{NORMAL}",
                self.first_part,
                self.color,
                self.highlight(&self.last_part[..n - self.first_part.len()])
            )
        } else {
            format!("{}{}{}{NORMAL}", &self.first_part[..n], self.color, self.highlight(&self.last_part))
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

fn determine_color(path: &Path) -> String {
    if path.is_symlink() {
        YELLOW // Symlinks
    } else if path.is_dir() {
        CYAN // Directories
    } else {
        MAGENTA // Regular files
    }
    .to_string()
}

fn build_directory_tree(dir: &str) -> DirectoryNode {
    let path = PathBuf::from(dir);

    if !path.is_dir() {
        return DirectoryNode {
            path: path.clone(),
            children: Vec::new(),
            matched: false,
            color: determine_color(&path),
            error: Some(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Error: '{dir}' is not a directory."),
            )),
        };
    }

    let children = match fs::read_dir(&path) {
        Ok(entries) => entries.filter_map(Result::ok),
        Err(e) => {
            return DirectoryNode {
                path,
                children: Vec::new(),
                matched: false,
                color: RED.to_string(),
                error: Some(e),
            };
        }
    }
    .map(|entry| {
        if entry
            .file_type()
            .expect("Failed to get file type for entry")
            .is_dir()
        {
            build_directory_tree(
                entry
                    .path()
                    .to_str()
                    .expect("Failed to convert path to string"),
            )
        } else {
            DirectoryNode {
                color: determine_color(&entry.path()),
                path: entry.path(),
                children: Vec::new(),
                matched: false,
                error: None,
            }
        }
    })
    .collect();

    DirectoryNode {
        path: path.clone(),
        children,
        matched: false,
        color: determine_color(&path),
        error: None,
    }
}

fn mark_matched_nodes(node: &mut DirectoryNode, re: &Regex) -> bool {
    node.matched = node
        .path
        .file_name()
        .is_some_and(|f| re.is_match(f.to_string_lossy().as_ref()))
        | node
            .children
            .iter_mut()
            .fold(false, |acc, child| acc | mark_matched_nodes(child, re));

    node.matched
}

fn main_loop(directory: &str, style: &Style, case_sensitive: bool) -> Result<Option<String>, String> {
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
            Err(e) => {
                return Err(format!(
                    "Error: Invalid regex pattern '{pattern}': {e}"
                ))
            }
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

        match get_input() {
            Event::Key(c) => {
                if cursor_pos < pattern.len() {
                    pattern.insert(cursor_pos, c);
                } else {
                    pattern.push(c);
                }
                cursor_pos += 1;
            }
            Event::Direction(d) => {
                match d {
                    Direction::Up => {}
                    Direction::Down => {}
                    Direction::Left => {
                        cursor_pos = cursor_pos.saturating_sub(1);
                    }
                    Direction::Right => {
                        cursor_pos += 1;
                        if cursor_pos > pattern.len() {
                            cursor_pos = pattern.len();
                        }
                    }
                };
            }
            Event::Navigation(n) => {
                match n {
                    Navigation::PageUp => {
                        scroll += 1;
                    }
                    Navigation::PageDown => {
                        scroll = scroll.saturating_sub(1);
                    }
                    Navigation::Home => {
                        cursor_pos = 0;
                    }
                    Navigation::End => {
                        cursor_pos = pattern.len();
                    }
                };
            }
            Event::Backspace => {
                let one_before = cursor_pos.saturating_sub(1);
                if one_before < pattern.len() {
                    pattern.remove(one_before);
                }
                cursor_pos = cursor_pos.saturating_sub(1);
            }
            Event::Clear => {
                pattern.clear();
                cursor_pos = 0;
            }
            Event::Enter => {
                return Ok(Some(pattern));
            }
            Event::Exit => {
                break;
            }
        }
    }

    Ok(None)
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
        print!("{pattern}");
    }
}
