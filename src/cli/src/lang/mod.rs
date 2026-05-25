use std::path::Path;

pub mod dart;
pub mod go;
pub mod rust;
pub mod python;
pub mod typescript;

#[derive(Debug, Clone)]
pub struct ParseResult {
    pub file_path: String,
    pub tree: tree_sitter::Tree,
    pub source: String,
}

pub trait LanguageParser {
    fn language_name(&self) -> &'static str;
    fn file_extensions(&self) -> &'static [&'static str];
    fn parse(&mut self, file_path: &Path, source: &str) -> Option<ParseResult>;
}
