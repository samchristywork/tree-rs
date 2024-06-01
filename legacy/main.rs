pub mod render;
pub mod util;

use crate::render::{print_tree, render};
use crate::util::filter_tree;
use clap::{arg, command, ArgGroup, Command};
use std::path::PathBuf;
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
    val: String,
    children: Vec<TreeNode>,
    node_type: NodeType,
}

fn read_dir_incremental(root: &mut TreeNode, dirname: PathBuf, limit: &mut i32) {
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
    entries.sort_by_key(|entry| entry.as_ref().unwrap().path());

    if root.children.is_empty() {
        for entry in entries {
            let path = entry.unwrap().path();

            if limit == &0 {
                return;
            }

            let val = path.file_name().unwrap().to_str().unwrap().to_string();
            root.children.push(TreeNode {
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

    if let Some(c) = content {
        c.split("\n").for_each(|line| {
            text.push(Spans::from(vec![Span::raw(line.to_string())]));
        });
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
    let tree = filter_tree(root, &search_term);
    let content = print_tree(&tree, &Vec::new());
    terminal
        .draw(|f| ui(f, Some(search_term.clone()), Some(content.clone())))
        .unwrap();
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
        val: dirname.to_str().unwrap().to_string(),
        children: Vec::new(),
        node_type: NodeType::Dir,
    };

    render(&mut root, dirname.clone());
}
