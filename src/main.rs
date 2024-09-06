use clap::{Parser, ValueEnum};
use regex::Regex;
use std::io;
use std::io::BufRead;
use std::io::Write;
use std::path::Path;
use termion;

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

    /// Pattern to match
    #[clap(short, long)]
    pattern: Option<String>,

    /// Style to use for rendering (compact or full)
    #[clap(short, long, default_value = "full")]
    style: String,

    /// Disable alternate screen buffer
    #[clap(long, action, default_value_t = false)]
    no_alternate_screen: bool,
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

fn render_directory_tree(
    dir: &str,
    prefix: &str,
    pattern: &str,
    style: &Style,
) -> Result<(Vec<Line>, bool), std::io::Error> {
    let path = Path::new(dir);
    let mut output: Vec<Line> = Vec::new();
    let mut matched = false;

    if !path.is_dir() {
        output.push(Line {
            first_part: String::new(),
            last_part: format!("Error: {} is not a directory.", dir),
            color: red(),
        });
        return Ok((output, false));
    }

    let entries = match std::fs::read_dir(path) {
        Ok(entries) => entries,
        Err(e) => {
            if e.kind() == io::ErrorKind::PermissionDenied {
                output.push(Line {
                    first_part: String::new(),
                    last_part: format!("Error: Permission denied for directory '{}'.", dir),
                    color: red(),
                });
                return Ok((output, false));
            } else {
                return Err(e);
            }
        }
    };

    let mut entries_vec: Vec<_> = entries.collect::<Result<_, _>>()?;

    entries_vec.sort_by(|a, b| a.file_name().cmp(&b.file_name()));

    let num_entries = entries_vec.len();

    for (i, entry) in entries_vec.iter().enumerate() {
        let file_name = entry.file_name();
        let file_name_str = file_name.to_string_lossy();
        let is_last = i == num_entries - 1;

        let mut current_matched = false;
        match Regex::new(pattern) {
            Ok(re) => {
                if re.is_match(&file_name_str) {
                    current_matched = true;
                    matched = true;
                }
            }
            Err(_) => {
                eprintln!("Invalid regex pattern: {}", pattern);
                return Ok((output, false));
            }
        }

        let entry_path = entry.path();
        let mut subtree = Vec::new();

        let mut subtree_matched = false;
        let file_type = entry.file_type()?;

        if file_type.is_dir() {
            let new_prefix = if is_last {
                match &style {
                    Style::Compact => format!("{} ", prefix),
                    Style::Full => format!("{}  ", prefix),
                }
            } else {
                match &style {
                    Style::Compact => format!("{}│", prefix),
                    Style::Full => format!("{}│ ", prefix),
                }
            };

            let (subtree_result, sub_matched_result) =
                render_directory_tree(entry_path.to_str().unwrap(), &new_prefix, pattern, &style)?;
            subtree = subtree_result;
            subtree_matched = sub_matched_result;
            if sub_matched_result {
                matched = true;
            }
        }

        if current_matched || subtree_matched {
            let connector = match (&style, is_last) {
                (Style::Compact, true) => "└",
                (Style::Compact, false) => "├",
                (Style::Full, true) => "└─",
                (Style::Full, false) => "├─",
            };

            let color = if file_type.is_symlink() {
                yellow()
            } else if file_type.is_dir() {
                cyan()
            } else {
                magenta()
            };

            let line = Line {
                first_part: format!("{}{}", prefix, connector),
                last_part: format!("{}", file_name_str),
                color: color,
            };
            output.push(line);
            output.extend(subtree);
        }
    }

    Ok((output, matched))
}

fn flush() {
    std::io::stdout().flush().unwrap();
}

fn alternate_screen() {
    println!("\x1B[?1049h");
    flush();
}

fn normal_screen() {
    println!("\x1B[?1049l");
    flush();
}

fn get_user_input() -> Option<String> {
    let stdin = io::stdin();
    let mut line = String::new();

    match stdin.lock().read_line(&mut line) {
        Ok(0) => {
            return None;
        }
        Ok(_) => {
            line.pop();
            return Some(line);
        }
        Err(_) => {
            return None;
        }
    }
}

fn cleanup(no_alternate_screen: bool) {
    if !no_alternate_screen {
        normal_screen();
    }
}

fn clear_screen() {
    print!("\x1B[2J\x1B[H");
    flush();
}

fn constrain_dimensions(tree: Vec<Line>, screen_size: (u16, u16)) -> String {
    let max_width = screen_size.0 as usize;
    let max_height = screen_size.1 as usize - 3;

    let mut constrained_tree = String::new();

    for line in tree.iter().take(max_height) {
        constrained_tree += line.to_limited_string(max_width).as_str();
        constrained_tree += "\n";
    }

    constrained_tree
}

fn main() {
    let args = Args::parse();

    if !args.no_alternate_screen {
        alternate_screen();
    }

    let mut pattern = args.pattern.clone().unwrap_or_else(|| String::from(""));

    let style = match args.style.as_str() {
        "compact" => Style::Compact,
        "full" => Style::Full,
        _ => Style::Full,
    };

    loop {
        let screen_size = termion::terminal_size().unwrap_or((80, 24));

        match render_directory_tree(&args.directory, "", &pattern, &style) {
            Ok((tree, _matched)) => {
                clear_screen();
                println!("{}{}{}", cyan(), args.directory, normal());
                print!("{}", constrain_dimensions(tree, screen_size));
                flush();
                println!("");
                println!("Ctrl+D to exit");
                print!("Pattern (current: '{}'): ", pattern);
                flush();
            }
            Err(e) => eprintln!("Failed to render directory tree: {}", e),
        }

        match get_user_input() {
            Some(input) => {
                pattern = input;
            }
            None => break,
        }
    }

    cleanup(args.no_alternate_screen);
}
