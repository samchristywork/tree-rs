use std::path::PathBuf;

struct TreeNode {
    val: String,
    children: Vec<TreeNode>,
}

fn read_dir_recursive(dirname: PathBuf) -> TreeNode {
    let mut root = TreeNode {
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
        println!("{}", root.val);
    } else {
        println!("{}── {}", indent.join(""), root.val);
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

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let dirname = args.iter().nth(1).unwrap();

    let dirname = PathBuf::from(dirname).canonicalize().unwrap();
    let mut root = read_dir_recursive(dirname);
    sort_tree(&mut root);
    print_tree(&root, &Vec::new());
}
