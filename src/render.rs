use crate::{
    read_dir_incremental, refresh, ui,
    util::{term_setup, term_teardown},
    ColorOptions, TreeNode,
};
use crossterm::event::{self, Event, KeyCode};
use std::{path::PathBuf, time::Duration};

pub fn print_tree(root: &TreeNode, indent: &Vec<String>, color_options: &ColorOptions) -> String {
    let mut return_string = String::new();
    let mut indent = indent.clone();

    if indent.len() == 0 {
        match color_options {
            ColorOptions::Default => {
                return_string.push_str(&format!("\x1b[{}m", root.color));
                return_string.push_str(&format!("{}", root.val));
                return_string.push_str(&format!("\x1b[0m\n"));
            }
            ColorOptions::NoColor => {
                return_string.push_str(&format!("{}", root.val));
                return_string.push_str(&format!("\n"));
            }
        }
    } else {
        match color_options {
            ColorOptions::Default => {
                return_string.push_str(&format!("{}──", indent.join("")));
                return_string.push_str(&format!("\x1b[{}m", root.color));
                return_string.push_str(&format!(" {}", root.val));
                return_string.push_str(&format!("\x1b[0m\n"));
            }
            ColorOptions::NoColor => {
                return_string.push_str(&format!("{}──", indent.join("")));
                return_string.push_str(&format!(" {}", root.val));
                return_string.push_str(&format!("\n"));
            }
        }
    }

    if root.children.len() != 0 {
        if indent.len() > 0 && indent.last().unwrap() == "├" {
            indent.pop();
            indent.push("│   ".to_string());
        }
        if indent.len() > 0 && indent.last().unwrap() == "└" {
            indent.pop();
            indent.push("    ".to_string());
        }
        indent.push("├".to_string());
    }

    for (i, child) in root.children.iter().enumerate() {
        if i == root.children.len() - 1 {
            indent.pop();
            indent.push("└".to_string());
        }
        return_string.push_str(&print_tree(child, &indent, color_options));
    }

    return_string
}

pub fn render(root: &mut TreeNode, dirname: PathBuf) {
    let mut terminal = term_setup();

    let content = print_tree(&root, &Vec::new(), &ColorOptions::NoColor);
    terminal.draw(|f| ui(f, None, Some(content))).unwrap();

    let mut search_term = String::new();

    let mut running = true;
    let mut duration = 0;
    loop {
        if running {
            let mut allocated = 100;
            read_dir_incremental(root, dirname.clone(), &mut allocated);

            if allocated > 0 {
                running = false;
                duration = 10;
            }
            refresh(&root, search_term.clone(), &mut terminal);
        }

        if let Ok(event) = event::poll(Duration::from_millis(duration)) {
            if event {
                if let Ok(event) = event::read() {
                    if let Event::Key(key) = event {
                        match key.code {
                            KeyCode::Char(c) => {
                                search_term.push(c);
                                refresh(&root, search_term.clone(), &mut terminal);
                            }
                            KeyCode::Esc => {
                                break;
                            }
                            KeyCode::Backspace => {
                                search_term.pop();
                                refresh(&root, search_term.clone(), &mut terminal);
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    term_teardown(&mut terminal);
}
