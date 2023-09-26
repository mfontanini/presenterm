use crate::markdown::elements::CodeLanguage;
use syntect::{
    easy::HighlightLines,
    highlighting::{Style, Theme, ThemeSet},
    parsing::SyntaxSet,
    util::{as_24_bit_terminal_escaped, LinesWithEndings},
};

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

    pub fn highlight<'a>(&self, code: &'a str, language: &CodeLanguage) -> Vec<CodeLine<'a>> {
        let extension = Self::language_extension(language);
        let syntax = self.syntax_set.find_syntax_by_extension(extension).unwrap();
        let mut highlight_lines = HighlightLines::new(syntax, &self.theme);
        let mut lines = Vec::new();
        for line in LinesWithEndings::from(code) {
            let ranges: Vec<(Style, &str)> = highlight_lines.highlight_line(line, &self.syntax_set).unwrap();
            let escaped = as_24_bit_terminal_escaped(&ranges, true);
            let code_line = CodeLine { original: line, formatted: escaped };
            lines.push(code_line);
        }
        lines
    }

    fn language_extension(language: &CodeLanguage) -> &'static str {
        use CodeLanguage::*;
        match language {
            Asp => "asa",
            Bash => "bash",
            BatchFile => "bat",
            C => "c",
            CSharp => "cs",
            Clojure => "clj",
            Cpp => "cpp",
            Css => "css",
            DLang => "d",
            Erlang => "erl",
            Go => "go",
            Haskell => "hs",
            Html => "html",
            Java => "java",
            JavaScript => "js",
            Json => "json",
            Latex => "tex",
            Lua => "lua",
            Makefile => "make",
            Markdown => "md",
            OCaml => "ml",
            Perl => "pl",
            Php => "php",
            Python => "py",
            R => "r",
            Rust => "rs",
            Scala => "scala",
            Shell => "shell",
            Sql => "sql",
            TypeScript => "js",
            Xml => "xml",
            Yaml => "yaml",
            // default to plain text so we get the same look&feel
            Unknown => "txt",
        }
    }
}

pub struct CodeLine<'a> {
    pub original: &'a str,
    pub formatted: String,
}

#[derive(Debug, thiserror::Error)]
#[error("invalid theme")]
pub struct InvalidTheme;
