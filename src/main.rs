use clap::{arg, command, ArgGroup, Command};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::time::Duration;
use std::{io, path::PathBuf};
use tokio::sync::mpsc;
use tokio::task;
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::Rect,
    text::{Span, Spans},
    widgets::{Block, Borders, Paragraph},
    Frame, Terminal,
};
use std::time::Instant;
use tokio::runtime::Runtime;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Copy, Clone, Eq, PartialEq)]
enum NodeType {
    File,
    Dir,
}

struct TreeNode {
    color: i32,
    val: String,
    children: Vec<TreeNode>,
    node_type: NodeType,
}

enum ColorOptions {
    Default,
    NoColor,
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

fn read_dir_incremental(
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
                        let left_off_from = read_dir_incremental(
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
                        let left_off_from = read_dir_incremental(
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
                    let left_off_from = read_dir_incremental(
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

fn read_dir_incremental_2(root: &mut TreeNode, dirname: PathBuf, limit: &mut i32) {
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

            read_dir_incremental_2(root.children.last_mut().unwrap(), path, limit);
        }
    } else {
        let last_val = root.children.last().unwrap().val.clone();
        for entry in entries {
            let path = entry.unwrap().path();

            let val = path.file_name().unwrap().to_str().unwrap().to_string();

            root.children.push(TreeNode {
                color: 33,
                val,
                children: Vec::new(),
                node_type: NodeType::Dir,
            });

            read_dir_incremental_2(root.children.last_mut().unwrap(), path, limit);
        }
    }
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

async fn render(root: &mut TreeNode, dirname: PathBuf) {
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
                    ret = read_dir_incremental(root, dirname.clone(), ret, &mut counter, 4, &mut 0, &mut 0);

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

fn render2(root: &mut TreeNode, dirname: PathBuf) {
    let mut search_term = String::new();

    let mut ret = None;
    let mut running = true;
    loop {
        if running{
            let mut counter = 0;
            ret = read_dir_incremental(root, dirname.clone(), ret, &mut counter, 400, &mut 0, &mut 0);

            if ret.is_none() {
                let out = print_tree(&root, &Vec::new(), &ColorOptions::NoColor);
                println!("{}", out);

                running = false;
                break;
            }
        }
    }

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

    render(&mut root, dirname).await;
}
