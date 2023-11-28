use crate::markdown::elements::CodeLanguage;
use crossterm::{
    style::{SetBackgroundColor, SetForegroundColor},
    QueueableCommand,
};
use flate2::read::ZlibDecoder;
use once_cell::sync::Lazy;
use serde::Deserialize;
use std::{
    collections::BTreeMap,
    io::{self, Write},
    path::Path,
    sync::{Arc, Mutex},
};
use syntect::{
    easy::HighlightLines,
    highlighting::{Style, Theme, ThemeSet},
    parsing::SyntaxSet,
    LoadingError,
};

static SYNTAX_SET: Lazy<SyntaxSet> = Lazy::new(|| {
    let contents = include_bytes!("../../bat/syntaxes.bin");
    bincode::deserialize(contents).expect("syntaxes are broken")
});

static THEMES: Lazy<LazyThemeSet> = Lazy::new(|| {
    let contents = include_bytes!("../../bat/themes.bin");
    let theme_set: LazyThemeSet = bincode::deserialize(contents).expect("syntaxes are broken");
    let default_themes = ThemeSet::load_defaults();
    theme_set.merge(default_themes);
    theme_set
});

// This structure mimic's `bat`'s serialized theme set's.
#[derive(Debug, Deserialize)]
struct LazyThemeSet {
    serialized_themes: BTreeMap<String, Vec<u8>>,
    #[serde(skip)]
    themes: Mutex<BTreeMap<String, Arc<Theme>>>,
}

impl LazyThemeSet {
    fn merge(&self, themes: ThemeSet) {
        let mut all_themes = self.themes.lock().unwrap();
        for (name, theme) in themes.themes {
            if !self.serialized_themes.contains_key(&name) {
                all_themes.insert(name, theme.into());
            }
        }
    }

    fn get(&self, theme_name: &str) -> Option<Arc<Theme>> {
        let mut themes = self.themes.lock().unwrap();
        if let Some(theme) = themes.get(theme_name) {
            return Some(theme.clone());
        }
        let serialized = self.serialized_themes.get(theme_name)?;
        let decoded: Theme = bincode::deserialize_from(ZlibDecoder::new(serialized.as_slice())).ok()?;
        let decoded = Arc::new(decoded);
        themes.insert(theme_name.to_string(), decoded);
        themes.get(theme_name).cloned()
    }
}

/// A code highlighter.
#[derive(Clone)]
pub struct CodeHighlighter {
    theme: Arc<Theme>,
}

impl CodeHighlighter {
    /// Construct a new highlighted using the given [syntect] theme name.
    pub fn new(theme: &str) -> Result<Self, ThemeNotFound> {
        let theme = THEMES.get(theme).ok_or(ThemeNotFound)?;
        Ok(Self { theme })
    }

    /// Load .tmTheme themes from the provided path.
    pub fn load_themes_from_path(path: &Path) -> Result<(), LoadingError> {
        let themes = ThemeSet::load_from_folder(path)?;
        THEMES.merge(themes);
        Ok(())
    }

    /// Create a highlighter for a specific language.
    pub(crate) fn language_highlighter(&self, language: &CodeLanguage) -> LanguageHighlighter {
        let extension = Self::language_extension(language);
        let syntax = SYNTAX_SET.find_syntax_by_extension(extension).unwrap();
        let highlighter = HighlightLines::new(syntax, &self.theme);
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

impl Default for CodeHighlighter {
    fn default() -> Self {
        Self::new("base16-eighties.dark").expect("default theme not found")
    }
}

pub(crate) struct LanguageHighlighter<'a> {
    highlighter: HighlightLines<'a>,
}

impl<'a> LanguageHighlighter<'a> {
    pub(crate) fn highlight_line(&mut self, line: &str) -> String {
        self.style_line(line).map(|s| s.apply_style()).collect()
    }

    pub(crate) fn style_line<'b>(&mut self, line: &'b str) -> impl Iterator<Item = StyledTokens<'b>> {
        self.highlighter
            .highlight_line(line, &SYNTAX_SET)
            .unwrap()
            .into_iter()
            .map(|(style, tokens)| StyledTokens { style, tokens })
    }
}

pub(crate) struct StyledTokens<'a> {
    pub(crate) style: Style,
    pub(crate) tokens: &'a str,
}

impl<'a> StyledTokens<'a> {
    pub(crate) fn apply_style(&self) -> String {
        let background = to_ansi_color(self.style.background);
        let foreground = to_ansi_color(self.style.foreground);

        // We do this conversion manually as crossterm will reset the color after styling, and we
        // want to "keep it open" so that padding also uses this background color.
        //
        // Note: these unwraps shouldn't happen as this is an in-memory writer so there's no
        // fallible IO here.
        let mut cursor = io::BufWriter::new(Vec::new());
        if let Some(color) = background {
            cursor.queue(SetBackgroundColor(color)).unwrap();
        }
        if let Some(color) = foreground {
            cursor.queue(SetForegroundColor(color)).unwrap();
        }
        // syntect likes its input to contain \n but we don't want them as we pad text with extra
        // " " at the end so we get rid of them here.
        for chunk in self.tokens.split('\n') {
            cursor.write_all(chunk.as_bytes()).unwrap();
        }

        cursor.flush().unwrap();
        String::from_utf8(cursor.into_inner().unwrap()).unwrap()
    }
}

/// A theme could not be found.
#[derive(Debug, thiserror::Error)]
#[error("theme not found")]
pub struct ThemeNotFound;

// This code has been adapted from bat's: https://github.com/sharkdp/bat
fn to_ansi_color(color: syntect::highlighting::Color) -> Option<crossterm::style::Color> {
    use crossterm::style::Color;
    if color.a == 0 {
        Some(match color.r {
            0x00 => Color::Black,
            0x01 => Color::DarkRed,
            0x02 => Color::DarkGreen,
            0x03 => Color::DarkYellow,
            0x04 => Color::DarkBlue,
            0x05 => Color::DarkMagenta,
            0x06 => Color::DarkCyan,
            0x07 => Color::Grey,
            n => Color::AnsiValue(n),
        })
    } else if color.a == 1 {
        None
    } else {
        Some(Color::Rgb { r: color.r, g: color.g, b: color.b })
    }
}

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

    #[test]
    fn default_highlighter() {
        CodeHighlighter::default();
    }
}
