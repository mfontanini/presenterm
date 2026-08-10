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
use std::{cell::RefCell, collections::BTreeMap, fs, path::Path, rc::Rc, sync::OnceLock};
use syntect::{
    LoadingError,
    easy::HighlightLines,
    highlighting::{Style, Theme, ThemeSet},
    parsing::{SyntaxReference, SyntaxSet},
};

static SYNTAX_SET: OnceLock<SyntaxSet> = OnceLock::new();

/// The syntaxes that are used to highlight code snippets.
///
/// Unless [register_syntaxes_from_directory] was called beforehand, this only contains the
/// syntaxes that are shipped with the binary.
fn syntax_set() -> &'static SyntaxSet {
    SYNTAX_SET.get_or_init(bundled_syntaxes)
}

fn bundled_syntaxes() -> SyntaxSet {
    let contents = include_bytes!("../../bat/syntaxes.bin");
    bincode::deserialize(contents).expect("syntaxes are broken")
}

/// Register all `.sublime-syntax` syntaxes in the given directory, in addition to the ones that are
/// shipped with the binary.
///
/// Syntaxes are looked up recursively and take precedence over the bundled ones, meaning a syntax
/// that claims an extension that's already claimed by a bundled one will be used instead of it.
/// This is a no-op if the directory doesn't exist or contains no syntaxes.
///
/// This must be invoked before any code is highlighted, as otherwise the syntaxes will be ignored.
pub fn register_syntaxes_from_directory<P: AsRef<Path>>(path: P) -> Result<(), LoadingError> {
    let Some(syntaxes) = load_syntaxes(path)? else {
        return Ok(());
    };
    // This can only fail if we already started highlighting code, in which case there's nothing we
    // can do about it.
    let _ = SYNTAX_SET.set(syntaxes);
    Ok(())
}

/// Build a syntax set that contains the bundled syntaxes plus the ones in the given directory.
///
/// This returns `None` if there's no syntax to be loaded from that directory, as building a syntax
/// set is not cheap and there's no point in doing it if it would be identical to the bundled one.
fn load_syntaxes<P: AsRef<Path>>(path: P) -> Result<Option<SyntaxSet>, LoadingError> {
    let path = path.as_ref();
    if !fs::metadata(path).map(|metadata| metadata.is_dir()).unwrap_or(false) {
        return Ok(None);
    }
    let mut builder = bundled_syntaxes().into_builder();
    let bundled_count = builder.syntaxes().len();
    // Snippet lines are fed to the highlighter including their trailing newline, which is also how
    // bat builds the syntaxes we bundle.
    builder.add_from_folder(path, /* lines_include_newline */ true)?;
    if builder.syntaxes().len() == bundled_count { Ok(None) } else { Ok(Some(builder.build())) }
}

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
        let syntax = Self::language_syntax(syntax_set(), language);
        let highlighter = HighlightLines::new(syntax, &self.theme);
        LanguageHighlighter::new(language.clone(), highlighter)
    }

    fn language_syntax<'a>(syntax_set: &'a SyntaxSet, language: &SnippetLanguage) -> &'a SyntaxReference {
        match language {
            // Languages we don't know about are looked up by name so that syntaxes we don't
            // support natively, including any locally registered ones, can still be used by simply
            // naming them in the code block.
            SnippetLanguage::Unknown(token) => {
                syntax_set.find_syntax_by_token(token).unwrap_or_else(|| syntax_set.find_syntax_plain_text())
            }
            _ => {
                let extension = Self::language_extension(language);
                syntax_set.find_syntax_by_extension(extension).unwrap_or_else(|| syntax_set.find_syntax_plain_text())
            }
        }
    }

    fn language_extension(language: &SnippetLanguage) -> &'static str {
        use SnippetLanguage::*;
        match language {
            Ada => "adb",
            Asp => "asa",
            Awk => "awk",
            Bash => "sh",
            BatchFile => "cmd",
            C => "c",
            CMake => "cmake",
            CSharp => "cs",
            Clojure => "clj",
            Cpp => "cpp",
            Crontab => "crontab",
            Css => "css",
            Dart => "dart",
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
            GdScript => "gd",
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
            PowerShell => "ps1",
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
            Wsl => "sh",
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
    language: SnippetLanguage,
    highlighter: HighlightLines<'a>,
    parse_started: bool,
}

impl<'a> LanguageHighlighter<'a> {
    fn new(language: SnippetLanguage, highlighter: HighlightLines<'a>) -> Self {
        Self { language, highlighter, parse_started: false }
    }

    pub(crate) fn style_line(&mut self, line: &str, block_style: &CodeBlockStyle) -> Line {
        if !self.parse_started {
            let line = line.trim();
            if !line.is_empty() {
                self.parse_started = true;
                // Parse a fake "<?php" line if PHP code doesn't start with one so highlighting
                // looks good.
                if matches!(self.language, SnippetLanguage::Php) && !line.starts_with("<?php") {
                    self.highlighter.highlight_line("<?php\n", syntax_set()).unwrap();
                }
            }
        }
        let texts: Vec<_> = self
            .highlighter
            .highlight_line(line, syntax_set())
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
            let syntax = syntax_set().find_syntax_by_extension(extension);
            assert!(syntax.is_some(), "extension {extension} for {language:?} not found");
        }
    }

    #[test]
    fn default_highlighter() {
        SnippetHighlighter::default();
    }

    fn write_syntax(directory: &Path, name: &str, extension: &str) {
        let syntax = format!(
            r#"%YAML 1.2
---
name: {name}
file_extensions: [{extension}]
scope: source.{extension}
contexts:
  main:
    - match: potato
      scope: keyword.other
"#
        );
        fs::write(directory.join(format!("{name}.sublime-syntax")), syntax).expect("writing syntax");
    }

    #[test]
    fn load_custom_syntaxes() {
        let directory = tempdir().expect("creating tempdir");
        // Use a nested directory to ensure we look up syntaxes recursively.
        let nested = directory.path().join("nested");
        fs::create_dir(&nested).expect("creating directory");
        write_syntax(&nested, "Potato", "potato");

        let syntaxes = load_syntaxes(directory.path()).expect("loading syntaxes").expect("no syntaxes loaded");
        assert!(syntaxes.find_syntax_by_name("Potato").is_some());
        // Bundled syntaxes must still be there.
        assert!(syntaxes.find_syntax_by_extension("rs").is_some());
    }

    #[test]
    fn custom_syntaxes_take_precedence() {
        let directory = tempdir().expect("creating tempdir");
        write_syntax(directory.path(), "Not rust", "rs");

        let syntaxes = load_syntaxes(directory.path()).expect("loading syntaxes").expect("no syntaxes loaded");
        let syntax = SnippetHighlighter::language_syntax(&syntaxes, &SnippetLanguage::Rust);
        assert_eq!(syntax.name, "Not rust");
    }

    #[test]
    fn load_syntaxes_from_empty_directory() {
        let directory = tempdir().expect("creating tempdir");
        let syntaxes = load_syntaxes(directory.path()).expect("loading syntaxes");
        assert!(syntaxes.is_none());
    }

    #[test]
    fn load_syntaxes_from_missing_directory() {
        let syntaxes =
            load_syntaxes("/tmp/presenterm/8ee2027983915ec78acc45027d874316").expect("loading syntaxes failed");
        assert!(syntaxes.is_none());
    }

    #[test]
    fn load_invalid_syntax() {
        let directory = tempdir().expect("creating tempdir");
        fs::write(directory.path().join("potato.sublime-syntax"), "this is not a syntax").expect("writing syntax");
        load_syntaxes(directory.path()).expect_err("loading syntaxes succeeded");
    }

    #[test]
    fn unknown_language_syntax_lookup() {
        let directory = tempdir().expect("creating tempdir");
        write_syntax(directory.path(), "Potato", "potato");
        let syntaxes = load_syntaxes(directory.path()).expect("loading syntaxes").expect("no syntaxes loaded");

        let lookup = |name: &str| {
            SnippetHighlighter::language_syntax(&syntaxes, &SnippetLanguage::Unknown(name.to_string())).name.clone()
        };
        // Custom syntaxes can be looked up by name and by extension.
        assert_eq!(lookup("Potato"), "Potato");
        assert_eq!(lookup("potato"), "Potato");
        // As can bundled ones that we don't support natively.
        assert_eq!(lookup("nim"), "Nim");
        // Anything else falls back to plain text.
        assert_eq!(lookup("something we don't know about"), "Plain Text");
        assert_eq!(lookup(""), "Plain Text");
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
