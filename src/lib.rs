//! Presenterm: a terminal slideshow presentation tool.
//!
//! This is not meant to be used as a crate!

pub(crate) mod custom;
pub(crate) mod demo;
pub(crate) mod diff;
pub(crate) mod execute;
pub(crate) mod export;
pub(crate) mod input;
pub(crate) mod markdown;
pub(crate) mod media;
pub(crate) mod presentation;
pub(crate) mod presenter;
pub(crate) mod processing;
pub(crate) mod render;
pub(crate) mod resource;
pub(crate) mod style;
pub(crate) mod theme;
pub(crate) mod tools;
pub(crate) mod typst;

pub use crate::{
    custom::{Config, ImageProtocol, ValidateOverflows},
    demo::ThemesDemo,
    execute::CodeExecuter,
    export::{ExportError, Exporter},
    input::source::CommandSource,
    markdown::parse::MarkdownParser,
    media::{graphics::GraphicsMode, printer::ImagePrinter, register::ImageRegistry},
    presenter::{PresentMode, Presenter, PresenterOptions},
    processing::builder::{PresentationBuilderOptions, Themes},
    render::highlighting::{CodeHighlighter, HighlightThemeSet},
    resource::Resources,
    theme::{LoadThemeError, PresentationTheme, PresentationThemeSet},
    typst::TypstRender,
};
