use std::path::Path;
use regex::Regex;

enum Style {
    Compact,
    Full,
}

fn render_directory_tree(dir: &str, prefix: &str, pattern: Option<&str>, style: &Style) -> Result<(String, bool), std::io::Error> {
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

fn main() {
    let dir=".";
    let pattern="query-cache.bin";
    match render_directory_tree(dir, "", Some(pattern), &Style::Compact) {
        Ok((tree, matched)) => {
            println!("{dir}");
            print!("{}", tree);
            println!("Matched pattern: {}", matched);
        }
        Err(e) => eprintln!("Failed to render directory tree: {}", e),
    }
}
