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

    /// Style of the tree
    #[clap(value_enum, default_value_t = Style::Compact)]
    style: Style,

    /// Disable alternate screen buffer
    #[clap(long, action)]
    no_alternate_screen: bool,
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
        if let Some(p) = pattern {
            let re = Regex::new(p).unwrap();
            if re.is_match(&file_name_str) {
                current_matched = true;
                matched = true;
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
            let line = format!("{}{}{}\n", prefix, connector, file_name_str);
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

fn get_user_input() -> String {
    let mut input = String::new();
    print!("Enter pattern to match (or leave empty to show all): ");
    flush();
    std::io::stdin().read_line(&mut input).unwrap();
    input.trim().to_string()
}

fn main() {
    let args = Args::parse();

    if !args.no_alternate_screen {
        alternate_screen();
    }

    let mut pattern = args.pattern.clone().unwrap_or_else(|| String::from(""));
    let style = args.style.clone();

    loop {
        match render_directory_tree(&args.directory, "", &pattern, &style) {
            Ok((tree, matched)) => {
                println!("{}", args.directory);
                print!("{}", tree);
                flush();
                println!("Matched pattern: {}", matched);
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

    if !args.no_alternate_screen {
        normal_screen();
    }
}
