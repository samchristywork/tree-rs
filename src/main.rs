use std::path::Path;
use regex::Regex;

fn render_directory_tree(dir: &str, prefix: &str, pattern: Option<&str>) -> Result<(String, bool), std::io::Error> {
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

        let connector = if is_last { "└─" } else { "├─" };
        let line = format!("{}{}{}\n", prefix, connector, file_name_str);
        output.push_str(&line);

        if let Some(p) = pattern {
            let re = Regex::new(p).unwrap();
            if re.is_match(&file_name_str) {
                matched = true;
            }
        }

        let entry_path = entry.path();
        if entry_path.is_dir() {
            let new_prefix = if is_last {
                format!("{}  ", prefix)
            } else {
                format!("{}│ ", prefix)
            };
            let (subtree, sub_matched) = render_directory_tree(entry_path.to_str().unwrap(), &new_prefix, pattern)?;
            output.push_str(&subtree);
            matched = matched || sub_matched;
        }
    }

    Ok((output, matched))
}

fn main() {
    match render_directory_tree("my_dir", "", Some("c1")) {
        Ok((tree, matched)) => {
            print!("{}", tree);
            println!("Matched pattern: {}", matched);
        }
        Err(e) => eprintln!("Failed to render directory tree: {}", e),
    }
}
