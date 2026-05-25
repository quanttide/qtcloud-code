use std::path::Path;

use super::{LanguageParser, ParseResult};

pub struct TypeScriptParser {
    parser: tree_sitter::Parser,
}

impl TypeScriptParser {
    pub fn new() -> Result<Self, String> {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into())
            .map_err(|e| format!("设置 TypeScript 语言失败: {}", e))?;
        Ok(Self { parser })
    }
}

impl LanguageParser for TypeScriptParser {
    fn language_name(&self) -> &'static str {
        "TypeScript"
    }

    fn file_extensions(&self) -> &'static [&'static str] {
        &["ts"]
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
    fn test_ts_language_name() {
        let parser = TypeScriptParser::new().unwrap();
        assert_eq!(parser.language_name(), "TypeScript");
    }

    #[test]
    fn test_ts_file_extensions() {
        let parser = TypeScriptParser::new().unwrap();
        assert_eq!(parser.file_extensions(), &["ts"]);
    }

    #[test]
    fn test_ts_parse_valid() {
        let mut parser = TypeScriptParser::new().unwrap();
        let result = parser.parse(Path::new("f.ts"), "let x = 1").unwrap();
        assert_eq!(result.file_path, "f.ts");
    }

    #[test]
    fn test_ts_parse_empty() {
        let mut parser = TypeScriptParser::new().unwrap();
        assert!(parser.parse(Path::new("f.ts"), "").is_some());
    }
}

pub struct TsxParser {
    parser: tree_sitter::Parser,
}

impl TsxParser {
    pub fn new() -> Result<Self, String> {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_typescript::LANGUAGE_TSX.into())
            .map_err(|e| format!("设置 TSX 语言失败: {}", e))?;
        Ok(Self { parser })
    }
}

impl LanguageParser for TsxParser {
    fn language_name(&self) -> &'static str {
        "TSX"
    }

    fn file_extensions(&self) -> &'static [&'static str] {
        &["tsx"]
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
mod tests_tsx {
    use super::*;

    #[test]
    fn test_tsx_language_name() {
        let parser = TsxParser::new().unwrap();
        assert_eq!(parser.language_name(), "TSX");
    }

    #[test]
    fn test_tsx_file_extensions() {
        let parser = TsxParser::new().unwrap();
        assert_eq!(parser.file_extensions(), &["tsx"]);
    }

    #[test]
    fn test_tsx_parse_valid() {
        let mut parser = TsxParser::new().unwrap();
        let result = parser.parse(Path::new("f.tsx"), "const x: number = 1").unwrap();
        assert_eq!(result.file_path, "f.tsx");
    }
}
