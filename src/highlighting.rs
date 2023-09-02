use syntect::{
    easy::HighlightLines,
    highlighting::{Style, Theme, ThemeSet},
    parsing::SyntaxSet,
    util::{as_24_bit_terminal_escaped, LinesWithEndings},
};

use crate::elements::CodeLanguage;

pub struct CodeHighlighter {
    syntax_set: SyntaxSet,
    theme: Theme,
}

impl CodeHighlighter {
    pub fn new(theme: &str) -> Result<Self, InvalidTheme> {
        let syntax_set = SyntaxSet::load_defaults_newlines();
        let theme_set = ThemeSet::load_defaults();
        let theme = theme_set.themes.get(theme).ok_or(InvalidTheme)?.clone();
        Ok(Self { syntax_set, theme })
    }

    pub fn highlight(&self, code: &str, language: &CodeLanguage) -> Vec<String> {
        let extension = match language {
            CodeLanguage::Rust => "rs",
            // huh?
            CodeLanguage::Other => return vec![code.to_string()],
        };
        let syntax = self.syntax_set.find_syntax_by_extension(extension).unwrap();
        let mut highlight_lines = HighlightLines::new(syntax, &self.theme);
        let mut lines = Vec::new();
        for line in LinesWithEndings::from(code) {
            let ranges: Vec<(Style, &str)> = highlight_lines.highlight_line(line, &self.syntax_set).unwrap();
            let escaped = as_24_bit_terminal_escaped(&ranges[..], false);
            lines.push(escaped);
        }
        lines
    }
}

#[derive(Debug, thiserror::Error)]
#[error("invalid theme")]
pub struct InvalidTheme;
