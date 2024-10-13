use clap::Parser;
use clap::ValueEnum;
use regex::Regex;
use std::fs;
use std::io;
use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use termion::raw::IntoRawMode;

#[derive(ValueEnum, Clone, Debug)]
enum Style {
    Compact,
    Full,
}

enum Direction {
    Up,
    Down,
    Left,
    Right,
}

enum Navigation {
    PageUp,
    PageDown,
    Home,
    End,
}

enum Event {
    Key(char),
    Direction(Direction),
    Navigation(Navigation),
    Backspace,
    Clear,
    Enter,
    Exit,
}

const BACKSPACE: u8 = 0x08;
const DEL: u8 = 0x7f;
const CTRL_U: u8 = 0x15;
const CTRL_D: u8 = 0x04;
const ENTER: u8 = b'\r';
const ESCAPE: u8 = 0x1b;

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

macro_rules! set_cursor_position {
    ($x:expr, $y:expr) => {
        print!("\x1B[{};{}H", $y, $x);
    };
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

fn render_directory_tree(
    node: &DirectoryNode,
    prefix: &str,
    is_last: bool,
    style: &Style,
) -> Vec<Line> {
    if !node.matched {
        return vec![];
    }

    let file_name = node.path.file_name().map_or_else(
        || "<unknown>".to_string(),
        |name| name.to_string_lossy().into_owned(),
    );

    let connector = match (style, is_last) {
        (Style::Compact, true) => "└",
        (Style::Compact, false) => "├",
        (Style::Full, true) => "└─",
        (Style::Full, false) => "├─",
    };

    let error = node
        .error
        .as_ref()
        .map_or_else(String::new, |e| format!(" {e}"));

    let mut lines = vec![Line {
        first_part: format!("{prefix}{connector}"),
        last_part: format!("{file_name}{error}"),
        color: node.color.clone(),
    }];

    let index_of_last_match = node
        .children
        .iter()
        .enumerate()
        .filter_map(|(i, child)| child.matched.then_some(i))
        .last()
        .unwrap_or(0);

    for (i, child) in node.children.iter().enumerate() {
        lines.extend(render_directory_tree(
            child,
            &if is_last {
                match style {
                    Style::Compact => format!("{prefix} "),
                    Style::Full => format!("{prefix}  "),
                }
            } else {
                match style {
                    Style::Compact => format!("{prefix}│"),
                    Style::Full => format!("{prefix}│ "),
                }
            },
            i == index_of_last_match,
            style,
        ));
    }

    lines
}

fn fixed_length_string(s: &str, n: usize) -> String {
    match s.len().cmp(&n) {
        std::cmp::Ordering::Less => format!("{}{}", s, " ".repeat(n - s.len())),
        std::cmp::Ordering::Greater => s[..n].to_string(),
        std::cmp::Ordering::Equal => s.to_string(),
    }
}

fn draw_tree(tree: &[Line], max_width: usize, max_height: usize, scroll: usize) -> String {
    let blank_line = &(" ".repeat(max_width) + "\r");

    tree.iter()
        .skip(scroll)
        .take(max_height)
        .fold(String::new(), |acc, line| {
            acc + blank_line + line.to_string(max_width).as_str() + "\r\n"
        })
        + ((tree.len() - scroll)..max_height)
            .fold(String::new(), |acc, _| acc + blank_line + "\r\n")
            .as_str()
}

fn render_input(pattern: &str, screen_size: (u16, u16)) -> String {
    let mut hex = String::new();
    for byte in pattern.as_bytes() {
        hex.push_str(&format!("0x{byte:02x} "));
    }

    format!(
        "{}\r\n{}\r\n{}",
        fixed_length_string(hex.as_str(), screen_size.0 as usize),
        fixed_length_string(
            format!("Pattern: '{pattern}'").as_str(),
            screen_size.0 as usize
        ),
        fixed_length_string("Ctrl+D to exit", screen_size.0 as usize)
    )
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

fn consume_7e(char_value: char, event: Navigation) -> Event {
    let mut buffer = [0; 1];
    match io::stdin().read_exact(&mut buffer) {
        Ok(()) => {
            let char_value = buffer[0] as char;
            match char_value as u8 {
                0x7e => Event::Navigation(event),
                _ => Event::Key(char_value),
            }
        }
        _ => Event::Key(char_value),
    }
}

fn handle_control_keys(char_value: char) -> Event {
    let mut buffer = [0; 1];
    match io::stdin().read_exact(&mut buffer) {
        Ok(()) => {
            let char_value = buffer[0] as char;
            match char_value as u8 {
                0x5b => {}
                _ => return Event::Key(char_value),
            }
        }
        Err(_) => return Event::Key(char_value),
    };

    let mut buffer = [0; 1];
    match io::stdin().read_exact(&mut buffer) {
        Ok(()) => {
            let char_value = buffer[0] as char;
            match char_value as u8 {
                0x41 => Event::Direction(Direction::Up),
                0x42 => Event::Direction(Direction::Down),
                0x43 => Event::Direction(Direction::Right),
                0x44 => Event::Direction(Direction::Left),
                0x35 => consume_7e(char_value, Navigation::PageUp),
                0x36 => consume_7e(char_value, Navigation::PageDown),
                0x31 => consume_7e(char_value, Navigation::Home),
                0x34 => consume_7e(char_value, Navigation::End),
                _ => Event::Key(char_value),
            }
        }
        Err(_) => Event::Key(char_value),
    }
}

fn get_input() -> Event {
    let mut buffer = [0; 1];
    match io::stdin().read_exact(&mut buffer) {
        Ok(()) => {
            let char_value = buffer[0] as char;
            match char_value as u8 {
                BACKSPACE | DEL => Event::Backspace,
                CTRL_U => Event::Clear,
                CTRL_D => Event::Exit,
                ENTER => Event::Enter,
                ESCAPE => handle_control_keys(char_value),
                _ => Event::Key(char_value),
            }
        }
        Err(_) => Event::Exit,
    }
}

fn main_loop(directory: &str, style: &Style, case_sensitive: bool) -> Option<String> {
    let term = termion::get_tty().expect("Failed to get terminal");
    let _raw_term = term.into_raw_mode().expect("Failed to enter raw mode");
    let mut directory_tree = build_directory_tree(directory);

    let mut pattern = String::new();
    let mut scroll = 0;
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
                eprintln!("{RED}Error: Failed to compile regex: {e}\r\n{NORMAL}");
                return None;
            }
        };

        mark_matched_nodes(&mut directory_tree, &re);

        set_cursor_position!(1, 1);
        let lines = render_directory_tree(&directory_tree, "", true, style);

        if scroll >= lines.len() {
            scroll = lines.len().saturating_sub(1);
        }

        print!(
            "{}\r\n",
            draw_tree(
                &lines,
                screen_size.0 as usize,
                screen_size.1 as usize - 3,
                scroll
            )
        );
        set_cursor_position!(1, screen_size.1.saturating_sub(2));
        print!("{}", render_input(pattern.as_str(), screen_size));
        std::io::stdout().flush().expect("Failed to flush stdout");

        match get_input() {
            Event::Key(c) => {
                pattern.push(c);
            }
            Event::Direction(d) => {
                match d {
                    Direction::Up => {}
                    Direction::Down => {}
                    Direction::Left => {}
                    Direction::Right => {}
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
                    Navigation::Home => {}
                    Navigation::End => {}
                };
            }
            Event::Backspace => {
                pattern.pop();
            }
            Event::Clear => {
                pattern.clear();
            }
            Event::Enter => {
                return Some(pattern);
            }
            Event::Exit => {
                break;
            }
        }
    }

    None
}

fn main() {
    let args = Args::parse();
    let style = match args.style.as_str() {
        "compact" => Style::Compact,
        _ => Style::Full,
    };

    print!("{ALTERNATE_SCREEN}");
    let result = main_loop(&args.directory, &style, args.case_sensitive);
    print!("{NORMAL_SCREEN}");

    if let Some(pattern) = result {
        print!("{pattern}");
    }
}
