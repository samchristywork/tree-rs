use std::path::PathBuf;

struct TreeNode {
    color: i32,
    val: String,
    children: Vec<TreeNode>,
}

fn read_dir_recursive(dirname: PathBuf) -> TreeNode {
    let mut root = TreeNode {
        color: 33,
        val: dirname.file_name().unwrap().to_str().unwrap().to_string(),
        children: Vec::new(),
    };

    let entries = std::fs::read_dir(dirname).unwrap();

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

    let dirname = PathBuf::from(dirname).canonicalize().unwrap();
    let mut root = read_dir_recursive(dirname);
    sort_tree(&mut root);
    print_tree(&root, &Vec::new());
}
