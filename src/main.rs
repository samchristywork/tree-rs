use clap::{arg, command, ArgGroup, ArgMatches, Command};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{io, path::PathBuf};
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::Rect,
    text::{Span, Spans},
    widgets::{Block, Borders, Paragraph},
    Frame, Terminal,
};

struct TreeNode {
    color: i32,
    val: String,
    children: Vec<TreeNode>,
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
            };
            root.children.push(child);
        }
    }

    root
}

enum ColorOptions {
    Default,
    NoColor,
}

fn print_tree(root: &TreeNode, indent: &Vec<String>, color_options: &ColorOptions) -> String {
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
    };

    for child in &root.children {
        let node = filter_tree(child, filter);
        if node.children.len() != 0 || node.val.contains(filter) {
            new_root.children.push(node);
        }
    }

    new_root
}

fn cli() -> Command {
    command!()
        .group(ArgGroup::new("foo").multiple(true))
        .next_help_heading("FOO")
        .args([arg!(-d --depth <LEVEL> "Descend only level directories deep").group("foo")])
        .group(ArgGroup::new("bar").multiple(true))
        .next_help_heading("BAR")
        .args([
            arg!(-o - -or "expr2 is not evaluate if exp1 is true").group("bar"),
            arg!(-a - -and "Same as `expr1 expr1`").group("bar"),
        ])
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

fn render(root: &TreeNode) {
    enable_raw_mode().unwrap();
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture).unwrap();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal.clear().unwrap();
    let content = print_tree(&root, &Vec::new(), &ColorOptions::NoColor);
    terminal.draw(|f| ui(f, None, Some(content))).unwrap();

    let mut search_term = String::new();
    loop {
        if let Event::Key(key) = event::read().unwrap() {
            match key.code {
                KeyCode::Char(c) => {
                    search_term.push(c);
                    let tree = filter_tree(&root, &search_term);
                    let content = print_tree(&tree, &Vec::new(), &ColorOptions::NoColor);
                    terminal
                        .draw(|f| ui(f, Some(search_term.clone()), Some(content.clone())))
                        .unwrap();
                }
                KeyCode::Esc => {
                    break;
                }
                e => {
                }
            }
        }
    }

    disable_raw_mode().unwrap();
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )
    .unwrap();
    terminal.show_cursor().unwrap();
}

fn main() {
    let mut args: Vec<String> = std::env::args().collect();

    if args.len() == 1 {
        args.push(".".to_string());
    }

    if args.len() > 2 {
        usage();
        return;
    }

    let dirname = args.iter().nth(1).unwrap();

    let dirname = match PathBuf::from(dirname).canonicalize() {
        Ok(path) => path,
        Err(e) => {
            println!("Error: {}", e);
            usage();
            return;
        }
    };

    let mut root = read_dir_recursive(dirname);
    sort_tree(&mut root);
    print_tree(&root, &Vec::new());
}
