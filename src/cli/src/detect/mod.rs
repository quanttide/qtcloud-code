use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Finding {
    pub file_path: PathBuf,
    pub line: usize,
    pub column: usize,
    pub severity: Severity,
    pub rule_id: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Severity {
    Must,
    Should,
    May,
}

pub trait Detector {
    fn rule_id(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn detect(&self, source: &str, tree: &tree_sitter::Tree, file_path: &PathBuf) -> Vec<Finding>;
}

pub fn walk_tree<F: FnMut(tree_sitter::Node)>(tree: &tree_sitter::Tree, mut f: F) {
    let mut cursor = tree.walk();
    loop {
        f(cursor.node());
        if cursor.goto_first_child() {
            continue;
        }
        loop {
            if cursor.goto_next_sibling() {
                break;
            }
            if !cursor.goto_parent() {
                return;
            }
        }
    }
}

pub mod long_function;
pub mod long_parameter_list;
pub mod missing_tests;
pub mod unsafe_block;
pub mod unused_variable;
