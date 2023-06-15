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

}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let dirname = args.iter().nth(1).unwrap();

    let dirname = PathBuf::from(dirname);
    let root = read_dir_recursive(dirname);
}
