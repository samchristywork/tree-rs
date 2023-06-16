use std::path::PathBuf;

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
    let entries = match std::fs::read_dir(dirname) {
        Ok(entries) => entries,
        Err(_) => {
            return;
        }
    };

    let mut entries: Vec<_> = entries.collect();

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

    for entry in entries {
        let entry = entry.unwrap();
        let path = entry.path();

        if path.file_name().unwrap().to_str().unwrap().starts_with(".") {
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

fn print_tree(root: &TreeNode, indent: &Vec<String>) {
    let mut indent = indent.clone();

    if indent.len() == 0 {
        print!("\x1b[{}m", root.color);
        println!("{}", root.val);
        print!("\x1b[0m");
    } else {
        print!("{}──", indent.join(""));
        print!("\x1b[{}m", root.color);
        println!(" {}", root.val);
        print!("\x1b[0m");
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
        print_tree(child, &indent);
    }
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

fn usage() {
    println!("Usage: tree <dirname>");
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
