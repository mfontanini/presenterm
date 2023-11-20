use crate::markdown::elements::CodeLanguage;
use once_cell::sync::Lazy;
use etcetera::BaseStrategy;
use syntect::{
    easy::HighlightLines,
    highlighting::{Style, Theme, ThemeSet},
    parsing::SyntaxSet,
    util::as_24_bit_terminal_escaped,
};

static SYNTAX_SET: Lazy<SyntaxSet> = Lazy::new(|| {
    let contents = include_bytes!("../../syntaxes/syntaxes.bin");
    bincode::deserialize(contents).expect("syntaxes are broken")
});
static THEMES: Lazy<ThemeSet> = Lazy::new(|| {
    let mut theme_set = ThemeSet::load_defaults();
    let basedirs = etcetera::choose_base_strategy().expect("could not choose base strategy");
    let bat_themes = basedirs.config_dir().join("bat").join("themes");

    theme_set.add_from_folder(&bat_themes).unwrap();
    theme_set
});

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

    /// Create a highlighter for a specific language.
    pub(crate) fn language_highlighter(&self, language: &CodeLanguage) -> LanguageHighlighter {
        let extension = Self::language_extension(language);
        let syntax = SYNTAX_SET.find_syntax_by_extension(extension).unwrap();
        let highlighter = HighlightLines::new(syntax, self.theme);
        LanguageHighlighter { highlighter }
    }

    fn language_extension(language: &CodeLanguage) -> &'static str {
        use CodeLanguage::*;
        match language {
            Ada => "adb",
            Asp => "asa",
            Awk => "awk",
            Bash => "bash",
            BatchFile => "bat",
            C => "c",
            CMake => "cmake",
            CSharp => "cs",
            Clojure => "clj",
            Cpp => "cpp",
            Crontab => "crontab",
            Css => "css",
            DLang => "d",
            Docker => "Dockerfile",
            Dotenv => "env",
            Elixir => "ex",
            Elm => "elm",
            Erlang => "erl",
            Go => "go",
            Haskell => "hs",
            Html => "html",
            Java => "java",
            JavaScript => "js",
            Json => "json",
            Kotlin => "kt",
            Latex => "tex",
            Lua => "lua",
            Makefile => "make",
            Markdown => "md",
            OCaml => "ml",
            Perl => "pl",
            Php => "php",
            Protobuf => "proto",
            Puppet => "pp",
            Python => "py",
            R => "r",
            Rust => "rs",
            Scala => "scala",
            Shell(_) => "sh",
            Sql => "sql",
            Swift => "swift",
            Svelte => "svelte",
            Terraform => "tf",
            TypeScript => "ts",
            // default to plain text so we get the same look&feel
            Unknown => "txt",
            Vue => "vue",
            Xml => "xml",
            Yaml => "yaml",
            Zig => "zig",
        }
    }
}
pub(crate) struct LanguageHighlighter {
    highlighter: HighlightLines<'static>,
}

impl LanguageHighlighter {
    pub(crate) fn highlight_line(&mut self, line: &str) -> String {
        let ranges = self.highlighter.highlight_line(line, &SYNTAX_SET).unwrap();
        as_24_bit_terminal_escaped(&ranges, true)
    }

    pub(crate) fn style_line<'a>(&mut self, line: &'a str) -> Vec<StyledTokens<'a>> {
        self.highlighter
            .highlight_line(line, &SYNTAX_SET)
            .unwrap()
            .into_iter()
            .map(|(style, tokens)| StyledTokens { style, tokens })
            .collect()
    }
}

pub(crate) struct StyledTokens<'a> {
    pub(crate) style: Style,
    pub(crate) tokens: &'a str,
}

impl<'a> StyledTokens<'a> {
    pub(crate) fn apply_style(&self) -> String {
        as_24_bit_terminal_escaped(&[(self.style, self.tokens)], true)
    }
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
        for language in CodeLanguage::iter() {
            let extension = CodeHighlighter::language_extension(&language);
            let syntax = SYNTAX_SET.find_syntax_by_extension(extension);
            assert!(syntax.is_some(), "extension {extension} for {language:?} not found");
        }
    }
}
