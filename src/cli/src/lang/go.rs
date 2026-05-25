use std::path::Path;

use super::{LanguageParser, ParseResult};

pub struct GoParser {
    parser: tree_sitter::Parser,
}

impl GoParser {
    pub fn new() -> Result<Self, String> {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_go::LANGUAGE.into())
            .map_err(|e| format!("设置 Go 语言失败: {}", e))?;
        Ok(Self { parser })
    }
}

impl LanguageParser for GoParser {
    fn language_name(&self) -> &'static str {
        "Go"
    }

    fn file_extensions(&self) -> &'static [&'static str] {
        &["go"]
    }

    fn parse(&mut self, file_path: &Path, source: &str) -> Option<ParseResult> {
        let tree = self.parser.parse(source, None)?;
        Some(ParseResult {
            file_path: file_path.to_string_lossy().to_string(),
            tree,
            source: source.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_name() {
        let parser = GoParser::new().unwrap();
        assert_eq!(parser.language_name(), "Go");
    }

    #[test]
    fn test_file_extensions() {
        let parser = GoParser::new().unwrap();
        assert_eq!(parser.file_extensions(), &["go"]);
    }

    #[test]
    fn test_parse_valid() {
        let mut parser = GoParser::new().unwrap();
        let result = parser.parse(Path::new("f.go"), "package main").unwrap();
        assert_eq!(result.file_path, "f.go");
    }

    #[test]
    fn test_parse_empty() {
        let mut parser = GoParser::new().unwrap();
        assert!(parser.parse(Path::new("f.go"), "").is_some());
    }
}
