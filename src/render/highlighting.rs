use crate::{markdown::elements::CodeLanguage, theme::CodeBlockStyle};
use crossterm::{
    style::{SetBackgroundColor, SetForegroundColor},
    QueueableCommand,
};
use flate2::read::ZlibDecoder;
use once_cell::sync::Lazy;
use serde::Deserialize;
use std::{
    cell::RefCell,
    collections::BTreeMap,
    fs,
    io::{self, Write},
    path::Path,
    rc::Rc,
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

static BAT_THEMES: Lazy<LazyThemeSet> = Lazy::new(|| {
    let contents = include_bytes!("../../bat/themes.bin");
    let theme_set: LazyThemeSet = bincode::deserialize(contents).expect("syntaxes are broken");
    theme_set
});

// This structure mimic's `bat`'s serialized theme set's.
#[derive(Debug, Deserialize)]
struct LazyThemeSet {
    serialized_themes: BTreeMap<String, Vec<u8>>,
}

pub struct HighlightThemeSet {
    themes: RefCell<BTreeMap<String, Rc<Theme>>>,
}

impl HighlightThemeSet {
    /// Construct a new highlighter using the given [syntect] theme name.
    pub fn load_by_name(&self, name: &str) -> Option<CodeHighlighter> {
        let mut themes = self.themes.borrow_mut();
        // Check if we already loaded this one.
        if let Some(theme) = themes.get(name).cloned() {
            Some(CodeHighlighter { theme })
        }
        // Otherwise try to deserialize it from bat's themes
        else if let Some(theme) = self.deserialize_bat_theme(name) {
            themes.insert(name.into(), theme.clone());
            Some(CodeHighlighter { theme })
        } else {
            None
        }
    }

    /// Register all highlighting themes in the given directory.
    pub fn register_from_directory<P: AsRef<Path>>(&mut self, path: P) -> Result<(), LoadingError> {
        let Ok(metadata) = fs::metadata(&path) else {
            return Ok(());
        };
        if !metadata.is_dir() {
            return Ok(());
        }
        let themes = ThemeSet::load_from_folder(path)?;
        let themes = themes.themes.into_iter().map(|(name, theme)| (name, Rc::new(theme)));
        self.themes.borrow_mut().extend(themes);
        Ok(())
    }

    fn deserialize_bat_theme(&self, name: &str) -> Option<Rc<Theme>> {
        let serialized = BAT_THEMES.serialized_themes.get(name)?;
        let decoded: Theme = bincode::deserialize_from(ZlibDecoder::new(serialized.as_slice())).ok()?;
        let decoded = Rc::new(decoded);
        Some(decoded)
    }
}

impl Default for HighlightThemeSet {
    fn default() -> Self {
        let themes = ThemeSet::load_defaults();
        let themes = themes.themes.into_iter().map(|(name, theme)| (name, Rc::new(theme))).collect();
        Self { themes: RefCell::new(themes) }
    }
}

/// A code highlighter.
#[derive(Clone)]
pub struct CodeHighlighter {
    theme: Rc<Theme>,
}

impl CodeHighlighter {
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
            Diff => "diff",
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
            Nix => "nix",
            OCaml => "ml",
            Perl => "pl",
            Php => "php",
            Protobuf => "proto",
            Puppet => "pp",
            Python => "py",
            R => "r",
            Ruby => "rb",
            Rust => "rs",
            Scala => "scala",
            Shell(_) => "sh",
            Sql => "sql",
            Swift => "swift",
            Svelte => "svelte",
            Terraform => "tf",
            TypeScript => "ts",
            Typst => "txt",
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
        let themes = HighlightThemeSet::default();
        themes.load_by_name("base16-eighties.dark").expect("default theme not found")
    }
}

pub(crate) struct LanguageHighlighter<'a> {
    highlighter: HighlightLines<'a>,
}

impl<'a> LanguageHighlighter<'a> {
    pub(crate) fn highlight_line(&mut self, line: &str, block_style: &CodeBlockStyle) -> String {
        self.style_line(line).map(|s| s.apply_style(block_style)).collect()
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
    pub(crate) fn apply_style(&self, block_style: &CodeBlockStyle) -> String {
        let has_background = block_style.background.unwrap_or(true);
        let background = has_background.then_some(to_ansi_color(self.style.background)).flatten();
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
    use tempfile::tempdir;

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

    #[test]
    fn load_custom() {
        let directory = tempdir().expect("creating tempdir");
        // A minimalistic .tmTheme theme.
        let theme = r#"
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple Computer//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>potato</key>
    <string>Example Color Scheme</string>
    <key>settings</key>
    <array>
        <dict>
            <key>settings</key>
            <dict></dict>
        </dict>
    </array>
</dict>"#;
        fs::write(directory.path().join("potato.tmTheme"), &theme).expect("writing theme");

        let mut themes = HighlightThemeSet::default();
        themes.register_from_directory(directory.path()).expect("loading themes");
        assert!(themes.load_by_name("potato").is_some());
    }

    #[test]
    fn register_from_missing_directory() {
        let mut themes = HighlightThemeSet::default();
        let result = themes.register_from_directory("/tmp/presenterm/8ee2027983915ec78acc45027d874316");
        result.expect("loading failed");
    }

    #[test]
    fn default_themes() {
        let themes = HighlightThemeSet::default();
        // This is a bat theme
        assert!(themes.load_by_name("GitHub").is_some());
        // This is a default syntect theme
        assert!(themes.load_by_name("InspiredGitHub").is_some());
    }
}
