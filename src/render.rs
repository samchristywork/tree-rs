use crate::{
    read_dir_incremental, refresh, ui,
    util::{term_setup, term_teardown},
    TreeNode,
};
use crossterm::event::{self, Event, KeyCode};
use std::{path::PathBuf, time::Duration};

pub fn print_tree(root: &TreeNode, indent: &Vec<String>) -> String {
    let mut return_string = String::new();
    let mut indent = indent.clone();

    if indent.len() == 0 {
        return_string.push_str(&format!("{}", root.val));
        return_string.push_str(&format!("\n"));
    } else {
        return_string.push_str(&format!("{}──", indent.join("")));
        return_string.push_str(&format!(" {}", root.val));
        return_string.push_str(&format!("\n"));
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
        return_string.push_str(&print_tree(child, &indent));
    }

    return_string
}

pub fn render(root: &mut TreeNode, dirname: PathBuf) {
    let mut terminal = term_setup();

    let content = print_tree(&root, &Vec::new());
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
