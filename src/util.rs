use crate::{NodeType, TreeNode};
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{io, path::PathBuf};
use tui::{backend::CrosstermBackend, Terminal};

fn print_node_name(dirname: &PathBuf) {
    match get_filetype(&dirname) {
        0 => {
            print!("\x1b[{}m", 31);
            println!("{}", dirname.file_name().unwrap().to_str().unwrap());
            print!("\x1b[0m");
        }
        1 => {
            print!("\x1b[{}m", 33);
            println!(
                "{}",
                dirname
                    .file_name()
                    .unwrap_or(std::ffi::OsStr::new("/"),)
                    .to_str()
                    .unwrap()
            );
            print!("\x1b[0m");
        }
        2 => {
            print!("\x1b[{}m", 34);
            println!("{}", dirname.file_name().unwrap().to_str().unwrap());
            print!("\x1b[0m");
        }
        _ => {}
    }
}

fn get_filetype(path: &PathBuf) -> i32 {
    let metadata = match std::fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(_) => {
            return 0;
        }
    };

    if metadata.is_dir() {
        return 1;
    }

    if metadata.is_file() {
        return 2;
    }

    0
}

fn sort_tree(root: &mut TreeNode) {
    root.children.sort_by(|a, b| a.val.cmp(&b.val));

    for child in &mut root.children {
        sort_tree(child);
    }
}

fn filter_tree(root: &TreeNode, filter: &str) -> TreeNode {
    let mut new_root = TreeNode {
        color: root.color,
        val: root.val.clone(),
        children: Vec::new(),
        node_type: root.node_type,
    };

    for child in &root.children {
        let node = filter_tree(child, filter);
        if node.children.len() != 0 || node.val.contains(filter) {
            new_root.children.push(node);
        }
    }

    new_root
}

fn term_setup() -> Terminal<CrosstermBackend<std::io::Stdout>> {
    enable_raw_mode().unwrap();
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture).unwrap();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal.clear().unwrap();

    terminal
}

fn term_teardown(terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>) {
    disable_raw_mode().unwrap();
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )
    .unwrap();
    terminal.show_cursor().unwrap();
}

fn get_tree_count(root: &TreeNode, node_type: NodeType) -> usize {
    let mut count = 0;
    for child in &root.children {
        if child.node_type == node_type {
            count += 1;
        }
        count += get_tree_count(child, node_type);
    }
    count
}
