use crate::{
    code::snippet::SnippetLanguage,
    markdown::{
        elements::{Line, Text},
        text_style::{Color, TextStyle},
    },
    theme::CodeBlockStyle,
};
use flate2::read::ZlibDecoder;
use once_cell::sync::Lazy;
use serde::Deserialize;
use std::{cell::RefCell, collections::BTreeMap, fs, path::Path, rc::Rc};
use syntect::{
    LoadingError,
    easy::HighlightLines,
    highlighting::{Style, Theme, ThemeSet},
    parsing::SyntaxSet,
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
    pub fn load_by_name(&self, name: &str) -> Option<SnippetHighlighter> {
        let mut themes = self.themes.borrow_mut();
        // Check if we already loaded this one.
        if let Some(theme) = themes.get(name).cloned() {
            Some(SnippetHighlighter { theme })
        }
        // Otherwise try to deserialize it from bat's themes
        else if let Some(theme) = self.deserialize_bat_theme(name) {
            themes.insert(name.into(), theme.clone());
            Some(SnippetHighlighter { theme })
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

/// A snippet highlighter.
#[derive(Clone)]
pub(crate) struct SnippetHighlighter {
    theme: Rc<Theme>,
}

impl SnippetHighlighter {
    /// Create a highlighter for a specific language.
    pub(crate) fn language_highlighter(&self, language: &SnippetLanguage) -> LanguageHighlighter<'_> {
        let extension = Self::language_extension(language);
        let syntax = SYNTAX_SET.find_syntax_by_extension(extension).unwrap();
        let highlighter = HighlightLines::new(syntax, &self.theme);
        LanguageHighlighter { highlighter }
    }

    fn language_extension(language: &SnippetLanguage) -> &'static str {
        use SnippetLanguage::*;
        match language {
            Ada => "adb",
            Asp => "asa",
            Awk => "awk",
            Bash => "sh",
            BatchFile => "bat",
            C => "c",
            CMake => "cmake",
            CSharp => "cs",
            Clojure => "clj",
            Cpp => "cpp",
            Crontab => "crontab",
            Css => "css",
            D2 => "txt",
            DLang => "d",
            Diff => "diff",
            Docker => "Dockerfile",
            Dotenv => "env",
            Elixir => "ex",
            Elm => "elm",
            Erlang => "erl",
            File => "txt",
            Fish => "fish",
            FSharp => "fsx",
            Go => "go",
            GraphQL => "graphql",
            Haskell => "hs",
            Html => "html",
            Java => "java",
            JavaScript => "js",
            Json => "json",
            Jsonnet => "jsonnet",
            Julia => "jl",
            Kotlin => "kt",
            Latex => "tex",
            Lua => "lua",
            Makefile => "make",
            Markdown => "md",
            Mermaid => "txt",
            Nix => "nix",
            Nushell => "txt",
            OCaml => "ml",
            Perl => "pl",
            Php => "php",
            Protobuf => "proto",
            Puppet => "pp",
            Python => "py",
            R => "r",
            Racket => "rkt",
            Ruby => "rb",
            Rust => "rs",
            RustScript => "rs",
            Scala => "scala",
            Shell => "sh",
            Sql => "sql",
            Swift => "swift",
            Svelte => "svelte",
            Tcl => "tcl",
            Terraform => "tf",
            Toml => "toml",
            TypeScript => "ts",
            TypeScriptReact => "tsx",
            Typst => "txt",
            // default to plain text so we get the same look&feel
            Unknown(_) => "txt",
            Verilog => "v",
            Vue => "vue",
            Xml => "xml",
            Yaml => "yaml",
            Zsh => "sh",
            Zig => "zig",
        }
    }
}

impl Default for SnippetHighlighter {
    fn default() -> Self {
        let themes = HighlightThemeSet::default();
        themes.load_by_name("base16-eighties.dark").expect("default theme not found")
    }
}

pub(crate) struct LanguageHighlighter<'a> {
    highlighter: HighlightLines<'a>,
}

impl LanguageHighlighter<'_> {
    pub(crate) fn highlight_line(&mut self, line: &str, block_style: &CodeBlockStyle) -> Line {
        self.style_line(line, block_style)
    }

    pub(crate) fn style_line(&mut self, line: &str, block_style: &CodeBlockStyle) -> Line {
        let texts: Vec<_> = self
            .highlighter
            .highlight_line(line, &SYNTAX_SET)
            .unwrap()
            .into_iter()
            .map(|(style, tokens)| StyledTokens::new(style, tokens, block_style).apply_style())
            .collect();
        Line(texts)
    }
}

pub(crate) struct StyledTokens<'a> {
    pub(crate) style: TextStyle,
    pub(crate) tokens: &'a str,
}

impl<'a> StyledTokens<'a> {
    pub(crate) fn new(style: Style, tokens: &'a str, block_style: &CodeBlockStyle) -> Self {
        let has_background = block_style.background;
        let background = has_background.then_some(parse_color(style.background)).flatten();
        let foreground = parse_color(style.foreground);
        let mut style = TextStyle::default();
        style.colors.background = background;
        style.colors.foreground = foreground;
        Self { style, tokens }
    }

    pub(crate) fn apply_style(&self) -> Text {
        let text: String = self.tokens.split('\n').collect();
        Text::new(text, self.style)
    }
}

// This code has been adapted from bat's: https://github.com/sharkdp/bat
fn parse_color(color: syntect::highlighting::Color) -> Option<Color> {
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
            0x08 => Color::DarkGrey,
            0x09 => Color::Red,
            0x0a => Color::Green,
            0x0b => Color::Yellow,
            0x0c => Color::Blue,
            0x0d => Color::Magenta,
            0x0e => Color::Cyan,
            0x0f => Color::White,
            n => Color::from_ansi(n)?,
        })
    } else if color.a == 1 {
        None
    } else {
        Some(Color::new(color.r, color.g, color.b))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use strum::IntoEnumIterator;
    use tempfile::tempdir;

    #[test]
    fn language_extensions_exist() {
        for language in SnippetLanguage::iter() {
            let extension = SnippetHighlighter::language_extension(&language);
            let syntax = SYNTAX_SET.find_syntax_by_extension(extension);
            assert!(syntax.is_some(), "extension {extension} for {language:?} not found");
        }
    }

    #[test]
    fn default_highlighter() {
        SnippetHighlighter::default();
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
        fs::write(directory.path().join("potato.tmTheme"), theme).expect("writing theme");

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
