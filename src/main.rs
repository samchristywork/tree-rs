pub mod render;
pub mod util;

use crate::render::{print_tree, render};
use crate::util::{filter_tree, print_node_name, term_setup, term_teardown};
use clap::{arg, command, ArgGroup, Command};
use crossterm::event::{self, Event, KeyCode};
use std::time::{SystemTime, UNIX_EPOCH};
use std::{path::PathBuf, time::Duration};
use tokio::sync::mpsc;
use tokio::task;
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::Rect,
    text::{Span, Spans},
    widgets::{Block, Borders, Paragraph},
    Frame, Terminal,
};

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum NodeType {
    File,
    Dir,
}

pub struct TreeNode {
    color: i32,
    val: String,
    children: Vec<TreeNode>,
    node_type: NodeType,
}

enum ColorOptions {
    Default,
    NoColor,
}

fn read_dir_recursive_and_print(dirname: PathBuf, indent: &Vec<String>) {
    if indent.len() == 0 {
        print_node_name(&dirname);
    }

    let entries = match std::fs::read_dir(dirname) {
        Ok(entries) => entries,
        Err(_) => {
            return;
        }
    };

    let mut entries: Vec<_> = entries.collect();
    entries.sort_by(|a, b| a.as_ref().unwrap().path().cmp(&b.as_ref().unwrap().path()));

    entries.retain(|entry| {
        let entry = entry.as_ref().unwrap();
        let path = entry.path();

        if path.file_name().unwrap().to_str().unwrap().starts_with(".") {
            return false;
        }
        if path
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("target")
        {
            return false;
        }

        true
    });

    for (i, entry) in entries.iter().enumerate() {
        let entry = entry.as_ref().unwrap();
        let path = entry.path();

        if path.is_dir() {
            if i == entries.len() - 1 {
                print!("{}└── ", indent.join(""));
                print_node_name(&path);
                let mut indent = indent.clone();
                indent.push("    ".to_string());
                read_dir_recursive_and_print(path, &indent);
            } else {
                print!("{}├── ", indent.join(""));
                print_node_name(&path);
                let mut indent = indent.clone();
                indent.push("│   ".to_string());
                read_dir_recursive_and_print(path, &indent);
            }
        } else {
            if i == entries.len() - 1 {
                print!("{}└── ", indent.join(""));
                print_node_name(&path);
            } else {
                print!("{}├── ", indent.join(""));
                print_node_name(&path);
            }
        }
    }
}

fn read_dir_recursive(dirname: PathBuf) -> TreeNode {
    let mut root = TreeNode {
        color: 33,
        val: dirname.file_name().unwrap().to_str().unwrap().to_string(),
        children: Vec::new(),
        node_type: NodeType::Dir,
    };

    let entries = match std::fs::read_dir(dirname) {
        Ok(entries) => entries,
        Err(_) => {
            return root;
        }
    };

    let mut entries: Vec<_> = entries.collect();
    entries.sort_by(|a, b| a.as_ref().unwrap().path().cmp(&b.as_ref().unwrap().path()));

    for entry in entries {
        let entry = entry.unwrap();
        let path = entry.path();

        if path.file_name().unwrap().to_str().unwrap().starts_with(".") {
            continue;
        }

        if path
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("target")
        {
            continue;
        }

        if path.is_dir() {
            let child = read_dir_recursive(path);
            root.children.push(child);
        } else {
            let child = TreeNode {
                color: 34,
                val: path.file_name().unwrap().to_str().unwrap().to_string(),
                children: Vec::new(),
                node_type: NodeType::File,
            };
            root.children.push(child);
        }
    }

    root
}

fn read_dir_recursive_in_place(root: &mut TreeNode, dirname: PathBuf) {
    root.color = 33;
    root.val = dirname.file_name().unwrap().to_str().unwrap().to_string();
    root.children = Vec::new();

    let entries = match std::fs::read_dir(dirname) {
        Ok(entries) => entries,
        Err(_) => {
            return;
        }
    };

    let mut entries: Vec<_> = entries.collect();
    entries.sort_by(|a, b| a.as_ref().unwrap().path().cmp(&b.as_ref().unwrap().path()));

    for entry in entries {
        let entry = entry.unwrap();
        let path = entry.path();

        if path.file_name().unwrap().to_str().unwrap().starts_with(".") {
            continue;
        }

        if path
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("target")
        {
            continue;
        }

        if path.is_dir() {
            let mut child = TreeNode {
                color: 33,
                val: path.file_name().unwrap().to_str().unwrap().to_string(),
                children: Vec::new(),
                node_type: NodeType::Dir,
            };
            read_dir_recursive_in_place(&mut child, path);
            root.children.push(child);
        } else {
            let child = TreeNode {
                color: 34,
                val: path.file_name().unwrap().to_str().unwrap().to_string(),
                children: Vec::new(),
                node_type: NodeType::File,
            };
            root.children.push(child);
        }
    }
}

fn legacy_read_dir_incremental(
    root: &mut TreeNode,
    dirname: PathBuf,
    begin_from: Option<PathBuf>,
    counter: &mut i32,
    increment: i32,
    directories: &mut i32,
    files: &mut i32,
) -> Option<PathBuf> {
    root.color = 33;
    root.val = dirname.file_name().unwrap().to_str().unwrap().to_string();

    let entries = match std::fs::read_dir(&dirname) {
        Ok(entries) => entries,
        Err(_) => {
            return None;
        }
    };

    let mut entries: Vec<_> = entries.collect();
    entries.sort_by(|a, b| a.as_ref().unwrap().path().cmp(&b.as_ref().unwrap().path()));

    for entry in entries {
        let entry = entry.unwrap();
        let path = entry.path();

        if path.file_name().unwrap().to_str().unwrap().starts_with(".") {
            continue;
        }

        if path
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("target")
        {
            continue;
        }

        if let Some(bf) = &begin_from {
            let bf = bf.iter().zip(path.iter()).collect::<Vec<_>>();
            if bf.last().unwrap().0 > bf.last().unwrap().1 {
                continue;
            }
        }

        if *counter > increment {
            return Some(path);
        }

        if path.is_dir() {
            let val = path.file_name().unwrap().to_str().unwrap().to_string();
            let last = root.children.last_mut();
            match last {
                Some(last) => {
                    if last.val == val {
                        let left_off_from = legacy_read_dir_incremental(
                            last,
                            path,
                            begin_from.clone(),
                            counter,
                            increment,
                            directories,
                            files,
                        );
                        if let Some(left_off_from) = left_off_from {
                            return Some(left_off_from);
                        }
                    } else {
                        *directories += 1;
                        let mut child = TreeNode {
                            color: 33,
                            val,
                            children: Vec::new(),
                            node_type: NodeType::Dir,
                        };
                        let left_off_from = legacy_read_dir_incremental(
                            &mut child,
                            path,
                            begin_from.clone(),
                            counter,
                            increment,
                            directories,
                            files,
                        );
                        root.children.push(child);
                        if let Some(left_off_from) = left_off_from {
                            return Some(left_off_from);
                        }
                    }
                }
                None => {
                    *counter += 1;
                    *directories += 1;
                    let mut child = TreeNode {
                        color: 33,
                        val,
                        children: Vec::new(),
                        node_type: NodeType::Dir,
                    };
                    let left_off_from = legacy_read_dir_incremental(
                        &mut child,
                        path,
                        begin_from.clone(),
                        counter,
                        increment,
                        directories,
                        files,
                    );
                    root.children.push(child);
                    if let Some(left_off_from) = left_off_from {
                        return Some(left_off_from);
                    }
                }
            }
        } else {
            *counter += 1;
            *files += 1;
            let child = TreeNode {
                color: 34,
                val: path.file_name().unwrap().to_str().unwrap().to_string(),
                children: Vec::new(),
                node_type: NodeType::File,
            };
            root.children.push(child);
        }
    }

    None
}

fn read_dir_incremental(root: &mut TreeNode, dirname: PathBuf, limit: &mut i32) {
    root.color = 33;
    root.val = dirname.file_name().unwrap().to_str().unwrap().to_string();

    *limit -= 1;

    if dirname.is_file() {
        root.node_type = NodeType::File;
        return;
    }

    root.node_type = NodeType::Dir;
    let entries = match std::fs::read_dir(&dirname) {
        Ok(entries) => entries,
        Err(_) => {
            return;
        }
    };

    let mut entries: Vec<_> = entries.collect();
    entries.sort_by(|a, b| a.as_ref().unwrap().path().cmp(&b.as_ref().unwrap().path()));

    if root.children.len() == 0 {
        for entry in entries {
            let path = entry.unwrap().path();

            if limit == &0 {
                return;
            }

            let val = path.file_name().unwrap().to_str().unwrap().to_string();
            root.children.push(TreeNode {
                color: 33,
                val,
                children: Vec::new(),
                node_type: NodeType::Dir,
            });

            read_dir_incremental(root.children.last_mut().unwrap(), path, limit);
        }
    } else {
        let mut start = false;
        let last_val = root.children.last().unwrap().val.clone();
        for entry in entries {
            let path = entry.unwrap().path();

            let val = path.file_name().unwrap().to_str().unwrap().to_string();

            if val == last_val {
                start = true;
                *limit += 1;
                read_dir_incremental(root.children.last_mut().unwrap(), path, limit);
                continue;
            }

            if start {
                if limit == &0 {
                    return;
                }

                root.children.push(TreeNode {
                    color: 33,
                    val,
                    children: Vec::new(),
                    node_type: NodeType::Dir,
                });

                read_dir_incremental(root.children.last_mut().unwrap(), path, limit);
            }
        }
    }
}

fn cli() -> Command {
    command!()
        .group(ArgGroup::new("LISTING OPTIONS").multiple(true))
        .next_help_heading("LISTING OPTIONS")
        .args([arg!(-d --depth <level> "Descend only level directories deep").group("LISTING OPTIONS")])
        .args([arg!(-n --number <number> "Specify the number of items to return").group("LISTING OPTIONS")])
        .arg(arg!(<dirname> "Directory name").required(false))
}

fn ui(f: &mut Frame<impl Backend>, search_term: Option<String>, content: Option<String>) {
    let mut main_window_size = f.size();
    main_window_size.height -= 3;

    let search_window_size = Rect::new(
        main_window_size.x,
        main_window_size.y + main_window_size.height,
        main_window_size.width,
        3,
    );

    let tree_window = Block::default().title("Tree").borders(Borders::ALL);
    let search_window = Block::default().title("Search").borders(Borders::ALL);
    let mut text = Vec::new();

    match content {
        Some(c) => {
            c.split("\n").for_each(|line| {
                text.push(Spans::from(vec![Span::raw(format!("{}", line))]));
            });
        }
        None => {}
    }

    let tree_widget = Paragraph::new(text)
        .block(tree_window)
        .wrap(tui::widgets::Wrap { trim: false });

    let search_widget = Paragraph::new(search_term.unwrap_or("".to_string()))
        .block(search_window)
        .wrap(tui::widgets::Wrap { trim: false });

    f.render_widget(tree_widget, main_window_size);
    f.render_widget(search_widget, search_window_size);
}

fn refresh(
    root: &TreeNode,
    search_term: String,
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
) {
    let tree = filter_tree(&root, &search_term);
    let content = print_tree(&tree, &Vec::new(), &ColorOptions::NoColor);
    terminal
        .draw(|f| ui(f, Some(search_term.clone()), Some(content.clone())))
        .unwrap();
}

async fn legacy_render(root: &mut TreeNode, dirname: PathBuf) {
    let mut terminal = term_setup();

    let content = print_tree(&root, &Vec::new(), &ColorOptions::NoColor);
    terminal.draw(|f| ui(f, None, Some(content))).unwrap();

    let mut search_term = String::new();

    let (tick_tx, tick_rx) = mpsc::channel(1);
    let (tx, mut rx) = mpsc::channel(100);
    task::spawn(async move {
        loop {
            if let Ok(event) = event::read() {
                if tx.send(event).await.is_err() {
                    break;
                }
            }
        }
    });

    let mut interval = tokio::time::interval(Duration::from_millis(10));
    task::spawn(async move {
        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if tick_tx.send(()).await.is_err() {
                        break;
                    }
                }
                else => break,
            }
        }
    });

    let mut tick_rx = Some(tick_rx);

    let mut ret = None;
    let mut running = true;
    loop {
        tokio::select! {
            _ = tick_rx.as_mut().unwrap().recv() => {
                let start = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
                loop {
                    if running{
                    let mut counter = 0;
                    ret = legacy_read_dir_incremental(root, dirname.clone(), ret, &mut counter, 4, &mut 0, &mut 0);

                    let tree = filter_tree(&root, &search_term);
                    let content = print_tree(&tree, &Vec::new(), &ColorOptions::NoColor);
                    terminal
                        .draw(|f| ui(f, Some(search_term.clone()), Some(content.clone())))
                        .unwrap();

                    if ret.is_none() {
                        running = false;
                    }
                    }
                    let end = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
                    if end - start > 100 {
                        break;
                    }
                }
            }
            Some(event) = rx.recv() => {
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
                        _ => {
                        }
                    }
                }
            }
        }
    }

    term_teardown(&mut terminal);
}

#[tokio::main]
async fn main() {
    let args = cli().get_matches();

    let depth: Option<&String> = args.get_one("depth");
    let dirname: Option<&String> = args.get_one("dirname");

    let dirname = match dirname {
        Some(d) => d,
        None => ".",
    };

    let dirname = match PathBuf::from(dirname).canonicalize() {
        Ok(path) => path,
        Err(e) => {
            println!("Error: {}", e);
            return;
        }
    };

    let mut root = TreeNode {
        color: 33,
        val: dirname.to_str().unwrap().to_string(),
        children: Vec::new(),
        node_type: NodeType::Dir,
    };

    render(&mut root, dirname.clone());
}
