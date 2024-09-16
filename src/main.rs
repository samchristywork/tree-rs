use clap::{Parser, ValueEnum};
use regex::Regex;
use std::fs;
use std::io;
use std::io::Read;
use std::io::Write;
use std::path::PathBuf;
use termion::raw::IntoRawMode;

#[derive(ValueEnum, Clone, Debug)]
enum Style {
    Compact,
    Full,
}

#[derive(Parser, Debug)]
#[clap(author = "Sam Christy", version = "1.0", about = "An interactive program for exploring directory trees.", long_about = None)]
struct Args {
    /// Directory to render
    #[clap(short, long, default_value = ".")]
    directory: String,

    /// Style to use for rendering (compact or full)
    #[clap(short, long, default_value = "full")]
    style: String,
}

fn cyan() -> String {
    "\x1B[36m".to_string()
}

fn magenta() -> String {
    "\x1B[35m".to_string()
}

fn yellow() -> String {
    "\x1B[33m".to_string()
}

fn red() -> String {
    "\x1B[31m".to_string()
}

fn normal() -> String {
    "\x1B[0m".to_string()
}

struct Line {
    first_part: String,
    last_part: String,
    color: String,
}

impl Line {
    fn length(&self) -> usize {
        self.first_part.len() + self.last_part.len()
    }

    fn to_limited_string(&self, n: usize) -> String {
        if self.length() <= n {
            format!("{}{}{}", self.first_part, self.color, self.last_part) + &normal()
        } else if self.first_part.len() < n {
            format!(
                "{}{}{}",
                self.first_part,
                self.color,
                &self.last_part[..n - self.first_part.len()]
            ) + &normal()
        } else {
            format!("{}{}{}", &self.first_part[..n], self.color, &self.last_part) + &normal()
        }
    }
}

struct DirectoryNode {
    path: PathBuf,
    children: Vec<DirectoryNode>,
    matched: bool,
    error: Option<io::Error>,
}

fn build_directory_tree_data(dir: &str) -> DirectoryNode {
    let path = PathBuf::from(dir);
    let mut node = DirectoryNode {
        path: path.clone(),
        children: Vec::new(),
        matched: false,
        error: None,
    };

    if !path.is_dir() {
        node.error = Some(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Error: '{dir}' is not a directory."),
        ));
        return node;
    }

    let entries = match fs::read_dir(&path) {
        Ok(entries) => entries,
        Err(e) => {
            node.error = Some(e);
            return node;
        }
    };

    let mut entries_vec: Vec<_> = entries
        .collect::<Result<_, _>>()
        .expect("Failed to collect directory entries");
    entries_vec.sort_by_key(std::fs::DirEntry::file_name);

    for entry in entries_vec {
        let entry_path = entry.path();
        let file_type = entry
            .file_type()
            .expect("Failed to get file type for entry");

        if file_type.is_dir() {
            let child_node = build_directory_tree_data(
                entry_path
                    .to_str()
                    .expect("Failed to convert path to string"),
            );
            node.children.push(child_node);
        } else {
            node.children.push(DirectoryNode {
                path: entry_path,
                children: Vec::new(),
                matched: false,
                error: None,
            });
        }
    }

    node
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

    let mut lines = Vec::new();

    let file_name = node
        .path
        .file_name()
        .expect("Failed to get file name")
        .to_string_lossy();

    let connector = match (style, is_last) {
        (Style::Compact, true) => "└",
        (Style::Compact, false) => "├",
        (Style::Full, true) => "└─",
        (Style::Full, false) => "├─",
    };

    let mut color = if fs::symlink_metadata(&node.path)
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false)
    {
        yellow()
    } else if node.path.is_dir() {
        cyan()
    } else {
        magenta()
    };

    if node.error.is_some() {
        color = red();
    }

    let error = if let Some(ref e) = node.error {
        format!(" {e}")
    } else {
        "".to_string()
    };

    let line = Line {
        first_part: format!("{prefix}{connector}"),
        last_part: format!("{file_name}{error}"),
        color,
    };
    lines.push(line);

    let num_children = node.children.len();
    for (i, child) in node.children.iter().enumerate() {
        let is_last_child = i == num_children - 1;
        let new_prefix = if is_last {
            match style {
                Style::Compact => format!("{prefix} "),
                Style::Full => format!("{prefix}  "),
            }
        } else {
            match style {
                Style::Compact => format!("{prefix}│"),
                Style::Full => format!("{prefix}│ "),
            }
        };

        let subtree_lines = render_directory_tree(child, &new_prefix, is_last_child, style);
        lines.extend(subtree_lines);
    }

    lines
}

fn flush() {
    std::io::stdout().flush().expect("Failed to flush stdout");
}

fn alternate_screen() {
    println!("\x1B[?1049h");
    flush();
}

fn normal_screen() {
    println!("\x1B[?1049l");
    flush();
}

fn set_cursor_position(x: u16, y: u16) {
    print!("\x1B[{y};{x}H");
}

fn fixed_length_string(s: &str, n: usize) -> String {
    match s.len().cmp(&n) {
        std::cmp::Ordering::Less => format!("{}{}", s, " ".repeat(n - s.len())),
        std::cmp::Ordering::Greater => s[..n].to_string(),
        std::cmp::Ordering::Equal => s.to_string(),
    }
}

fn draw_tree(tree: &[Line], screen_size: (u16, u16)) -> String {
    let max_width = screen_size.0 as usize;
    let max_height = screen_size.1 as usize - 5;

    let mut constrained_tree = String::new();

    for line in tree.iter().take(max_height) {
        constrained_tree += line.to_limited_string(max_width).as_str();

        let remaining_space = max_width.saturating_sub(line.length());
        if remaining_space > 0 {
            constrained_tree += &" ".repeat(remaining_space);
        }

        constrained_tree += "\r\n";
    }

    for _ in tree.len()..max_height {
        constrained_tree += &" ".repeat(max_width);
        constrained_tree += "\r\n";
    }

    constrained_tree
}

fn render_input(pattern: &str, screen_size: (u16, u16)) -> String {
    let mut hex = String::new();
    for byte in pattern.as_bytes() {
        hex.push_str(&format!("0x{byte:02x} "));
    }

    let hex = hex.as_str();
    format!(
        "{}\r\n{}\r\n{}",
        fixed_length_string(hex, screen_size.0 as usize),
        fixed_length_string(
            format!("Pattern: '{pattern}'").as_str(),
            screen_size.0 as usize
        ),
        fixed_length_string("Ctrl+D to exit", screen_size.0 as usize)
    )
}

fn render_data(
    directory_tree: &DirectoryNode,
    screen_size: (u16, u16),
    style: &Style,
) -> Result<String, std::io::Error> {
    let lines = render_directory_tree(directory_tree, "", true, style);

    Ok(format!(
        "{}{}{}\r\n{}\r\n",
        cyan(),
        fixed_length_string(
            directory_tree.path.to_str().unwrap_or(""),
            screen_size.0 as usize
        ),
        normal(),
        draw_tree(&lines, screen_size),
    ))
}

fn mark_matched_nodes(node: &mut DirectoryNode, re: &Regex) -> bool {
    let mut matched = re.is_match(
        node.path
            .file_name()
            .expect("Failed to get file name")
            .to_string_lossy()
            .as_ref(),
    );

    for child in node.children.iter_mut() {
        matched |= mark_matched_nodes(child, re);
    }

    node.matched = matched;
    matched
}

fn main_loop(directory: &str, style: &Style) -> String {
    let mut pattern = String::new();

    let term = termion::get_tty().expect("Failed to get terminal");
    let _raw_term = term.into_raw_mode().expect("Failed to enter raw mode");

    let mut directory_tree = build_directory_tree_data(directory);
    loop {
        let screen_size = termion::terminal_size().unwrap_or((80, 24));

        let re = match Regex::new(&pattern) {
            Ok(re) => re,
            Err(e) => {
                return format!(
                    "{}Error: Failed to compile regex: {}\r\n{}",
                    red(),
                    e,
                    normal()
                );
            }
        };

        mark_matched_nodes(&mut directory_tree, &re);

        set_cursor_position(1, 1);
        match render_data(&directory_tree, screen_size, style) {
            Ok(output) => print!("{output}"),
            Err(e) => {
                print!(
                    "{}Error: Failed to render directory: {}\r\n{}",
                    red(),
                    e,
                    normal()
                );
            }
        }
        set_cursor_position(1, screen_size.1.saturating_sub(2));
        print!("{}", render_input(pattern.as_str(), screen_size));
        flush();

        let mut buffer = [0; 1];
        match io::stdin().read_exact(&mut buffer) {
            Ok(()) => {
                let char_value = buffer[0] as char;
                match char_value as u8 {
                    0x7f | 0x08 => {
                        // Backspace
                        if !pattern.is_empty() {
                            pattern.pop();
                        }
                    }
                    0x15 => {
                        // Ctrl+U
                        pattern.clear();
                    }
                    0x04 => {
                        // Ctrl+D
                        pattern.clear();
                        break;
                    }
                    b'\r' => {
                        // Enter
                        break;
                    }
                    _ => {
                        pattern.push(char_value);
                    }
                }
            }
            Err(e) => {
                eprintln!("Error reading input: {e}");
                break;
            }
        }

        if pattern.len() >= 4 {
            match pattern
                .as_bytes()
                .iter()
                .rev()
                .take(4)
                .rev()
                .copied()
                .collect::<Vec<u8>>()[..]
            {
                [0x1b, 0x5b, 0x35, 0x7e] => {
                    // Page Up
                    pattern.pop();
                    pattern.pop();
                    pattern.pop();
                    pattern.pop();
                }
                [0x1b, 0x5b, 0x36, 0x7e] => {
                    // Page Down
                    pattern.pop();
                    pattern.pop();
                    pattern.pop();
                    pattern.pop();
                }
                _ => {}
            }
        }
    }

    pattern
}

fn main() {
    let args = Args::parse();

    let style = match args.style.as_str() {
        "compact" => Style::Compact,
        _ => Style::Full,
    };

    alternate_screen();

    let result = main_loop(&args.directory, &style);

    normal_screen();

    if result.is_empty() {
        println!("No output generated.");
    } else {
        println!("{result}");
    }
}
