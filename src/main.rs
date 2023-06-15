use std::path::PathBuf;

struct TreeNode {
    val: String,
    children: Vec<TreeNode>,
}

fn read_dir_recursive(dirname: PathBuf) -> TreeNode {
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let dirname = args.iter().nth(1).unwrap();

    let dirname = PathBuf::from(dirname);
    let root = read_dir_recursive(dirname);
}
