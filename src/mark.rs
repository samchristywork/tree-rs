use regex::Regex;

use crate::DirectoryNode;

pub fn mark_matched_nodes(node: &mut DirectoryNode, re: &Regex) -> bool {
    node.matched = node
        .path
        .file_name()
        .is_some_and(|f| re.is_match(f.to_string_lossy().as_ref()))
        | node
            .children
            .iter_mut()
            .fold(false, |acc, child| acc | mark_matched_nodes(child, re));

    node.matched
}
