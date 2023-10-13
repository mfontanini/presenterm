use crate::markdown::elements::ProgrammingLanguage;
use once_cell::sync::Lazy;
use syntect::{
    easy::HighlightLines,
    highlighting::{Style, Theme, ThemeSet},
    parsing::SyntaxSet,
    util::{as_24_bit_terminal_escaped, LinesWithEndings},
};

static SYNTAX_SET: Lazy<SyntaxSet> = Lazy::new(SyntaxSet::load_defaults_newlines);
static THEMES: Lazy<ThemeSet> = Lazy::new(ThemeSet::load_defaults);

/// A code highlighter.
#[derive(Clone)]
pub struct CodeHighlighter {
    theme: &'static Theme,
}

impl CodeHighlighter {
    /// Construct a new highlighted using the given [syntect] theme name.
    pub fn new(theme: &str) -> Result<Self, ThemeNotFound> {
        let theme = THEMES.themes.get(theme).ok_or(ThemeNotFound)?;
        Ok(Self { theme })
    }

    /// Highlight a piece of code.
    ///
    /// This splits the given piece of code into lines, highlights them individually, and returns them.
    pub fn highlight<'a>(&self, code: &'a str, language: &ProgrammingLanguage) -> Vec<CodeLine<'a>> {
        let extension = Self::language_extension(language);
        let syntax = SYNTAX_SET.find_syntax_by_extension(extension).unwrap();
        let mut highlight_lines = HighlightLines::new(syntax, self.theme);
        let mut lines = Vec::new();
        for line in LinesWithEndings::from(code) {
            let ranges: Vec<(Style, &str)> = highlight_lines.highlight_line(line, &SYNTAX_SET).unwrap();
            let escaped = as_24_bit_terminal_escaped(&ranges, true);
            let code_line = CodeLine { original: line, formatted: escaped };
            lines.push(code_line);
        }
        lines
    }

    fn language_extension(language: &ProgrammingLanguage) -> &'static str {
        use ProgrammingLanguage::*;
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
            Shell => "sh",
            Sql => "sql",
            TypeScript => "js",
            Xml => "xml",
            Yaml => "yaml",
            // default to plain text so we get the same look&feel
            Unknown => "txt",
        }
    }
}

/// A line of highlighted code.
pub struct CodeLine<'a> {
    /// The original line of code.
    pub original: &'a str,

    /// The formatted line of code.
    ///
    /// This uses terminal escape codes internally and is ready to be printed.
    pub formatted: String,
}

/// A theme could not be found.
#[derive(Debug, thiserror::Error)]
#[error("theme not found")]
pub struct ThemeNotFound;

#[cfg(test)]
mod test {
    use super::*;
    use strum::IntoEnumIterator;

    #[test]
    fn language_extensions_exist() {
        for language in ProgrammingLanguage::iter() {
            let extension = CodeHighlighter::language_extension(&language);
            let syntax = SYNTAX_SET.find_syntax_by_extension(extension);
            assert!(syntax.is_some(), "extension {extension} for {language:?} not found");
        }
    }
}
