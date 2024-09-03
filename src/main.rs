use std::path::Path;
use regex::Regex;
use std::io::Write;
use clap::{Parser, ValueEnum};
use std::io;
use std::io::BufRead;

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

fn normal() -> String {
    "\x1B[0m".to_string()
}

fn render_directory_tree(dir: &str, prefix: &str, pattern: &str, style: &Style) -> Result<(String, bool), std::io::Error> {
    let path = Path::new(dir);
    let mut output = String::new();
    let mut matched = false;

    if !path.is_dir() {
        output.push_str(&format!("Error: {} is not a directory.\n", dir));
        return Ok((output, false));
    }

    let entries = std::fs::read_dir(path)?;
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
        let mut subtree = String::new();

        if entry_path.is_dir() {
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
            let (subtree_result, sub_matched_result) = render_directory_tree(entry_path.to_str().unwrap(), &new_prefix, pattern, &style)?;
            subtree = subtree_result;
            let sub_matched = sub_matched_result;
            matched = matched || sub_matched;
            current_matched = current_matched || sub_matched;
        }

        if current_matched || matched {
            let connector = match (&style, is_last) {
                (Style::Compact, true) => "└",
                (Style::Compact, false) => "├",
                (Style::Full, true) => "└─",
                (Style::Full, false) => "├─",
            };

            let color = match entry_path.is_dir() {
                true => cyan(),
                false => magenta(),
            };

            let line = format!("{}{}{}{}{}\n", prefix, connector, color, file_name_str, normal());
            output.push_str(&line);
            output.push_str(&subtree);
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

fn main() {
    let args = Args::parse();

    if !args.no_alternate_screen {
        alternate_screen();
    }

    let mut pattern = args.pattern.clone().unwrap_or_else(|| String::from(""));

    let style=match args.style.as_str() {
        "compact" => Style::Compact,
        "full" => Style::Full,
        _ => Style::Full,
    };

    loop {
        match render_directory_tree(&args.directory, "", &pattern, &style) {
            Ok((tree, _matched)) => {
                clear_screen();
                println!("{}{}{}", cyan(), args.directory, normal());
                print!("{}", tree);
                flush();
                println!("");
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
