use regex::Regex;
use std::io::Write;

use crate::DirectoryNode;
use crate::Line;
use crate::Style;

macro_rules! set_cursor_position {
    ($x:expr, $y:expr) => {
        print!("\x1B[{};{}H", $y, $x);
    };
}

fn fixed_length_string(s: &str, n: usize) -> String {
    match s.len().cmp(&n) {
        std::cmp::Ordering::Less => format!("{}{}", s, " ".repeat(n - s.len())),
        std::cmp::Ordering::Greater => s[..n].to_string(),
        std::cmp::Ordering::Equal => s.to_string(),
    }
}

fn flatten_tree(node: &DirectoryNode, prefix: &str, is_last: bool, style: &Style) -> Vec<Line> {
    if !node.matched {
        return vec![];
    }

    let file_name = node.path.file_name().map_or_else(
        || ".".to_string(),
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
        lines.extend(flatten_tree(
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

fn render_tree(
    tree: &[Line],
    max_width: usize,
    max_height: usize,
    scroll: usize,
    re: &Regex,
) -> String {
    let blank_line = &(" ".repeat(max_width) + "\r");

    tree.iter()
        .skip(scroll)
        .take(max_height)
        .fold(String::new(), |acc, line| {
            acc + blank_line + line.to_string(re, max_width).as_str() + "\r\n"
        })
        + ((tree.len() - scroll)..max_height)
            .fold(String::new(), |acc, _| acc + blank_line + "\r\n")
            .as_str()
}

fn render_input(pattern: &str, pattern_is_valid: bool, screen_size: (u16, u16)) -> String {
    let mut hex = String::new();

    if !pattern_is_valid {
        hex.push_str("Invalid Pattern ");
    }

    for byte in pattern.as_bytes() {
        hex.push_str(&format!("0x{byte:02x} "));
    }

    format!(
        "{}\r\n{}\r\n{}",
        fixed_length_string(hex.as_str(), screen_size.0 as usize),
        fixed_length_string(
            format!("Pattern: {pattern}").as_str(),
            screen_size.0 as usize
        ),
        fixed_length_string("Ctrl+D to exit", screen_size.0 as usize)
    )
}

pub fn render(
    directory_tree: &DirectoryNode,
    pattern: &str,
    style: &Style,
    scroll: &mut usize,
    cursor_pos: usize,
    re: &Regex,
    pattern_is_valid: bool,
) {
    let screen_size = termion::terminal_size().unwrap_or((80, 24));

    set_cursor_position!(1, 1);
    let lines = flatten_tree(directory_tree, "", true, style);

    if *scroll >= lines.len() {
        *scroll = lines.len().saturating_sub(1);
    }

    print!(
        "{}\r\n",
        render_tree(
            &lines,
            screen_size.0 as usize,
            screen_size.1 as usize - 3,
            *scroll,
            re,
        )
    );
    set_cursor_position!(1, screen_size.1.saturating_sub(2));
    print!("{}", render_input(pattern, pattern_is_valid, screen_size));

    set_cursor_position!(
        u16::try_from(cursor_pos).expect("Cursor position is too large") + 10,
        screen_size.1.saturating_sub(1)
    );

    std::io::stdout().flush().expect("Failed to flush stdout");
}
