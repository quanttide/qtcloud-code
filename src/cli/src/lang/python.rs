use std::path::Path;

use super::{LanguageParser, ParseResult};

pub struct PythonParser {
    parser: tree_sitter::Parser,
}

impl PythonParser {
    pub fn new() -> Result<Self, String> {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_python::LANGUAGE.into())
            .map_err(|e| format!("设置 Python 语言失败: {}", e))?;
        Ok(Self { parser })
    }
}

impl LanguageParser for PythonParser {
    fn language_name(&self) -> &'static str {
        "Python"
    }

    fn file_extensions(&self) -> &'static [&'static str] {
        &["py"]
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
        let parser = PythonParser::new().unwrap();
        assert_eq!(parser.language_name(), "Python");
    }

    #[test]
    fn test_file_extensions() {
        let parser = PythonParser::new().unwrap();
        assert_eq!(parser.file_extensions(), &["py"]);
    }

    #[test]
    fn test_parse_valid() {
        let mut parser = PythonParser::new().unwrap();
        let result = parser.parse(Path::new("f.py"), "x = 1").unwrap();
        assert_eq!(result.file_path, "f.py");
    }

    #[test]
    fn test_parse_empty() {
        let mut parser = PythonParser::new().unwrap();
        assert!(parser.parse(Path::new("f.py"), "").is_some());
    }
}
