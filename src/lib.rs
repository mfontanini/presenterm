//! Presenterm: a terminal slideshow presentation tool.
//!
//! This is not meant to be used as a crate!

pub(crate) mod builder;
pub(crate) mod custom;
pub(crate) mod diff;
pub(crate) mod execute;
pub(crate) mod export;
pub(crate) mod input;
pub(crate) mod markdown;
pub(crate) mod presentation;
pub(crate) mod presenter;
pub(crate) mod render;
pub(crate) mod resource;
pub(crate) mod style;
pub(crate) mod theme;

pub use crate::{
    builder::Themes,
    custom::Config,
    export::{ExportError, Exporter},
    input::source::CommandSource,
    markdown::parse::MarkdownParser,
    presenter::{PresentMode, Presenter},
    render::highlighting::{CodeHighlighter, HighlightThemeSet},
    resource::Resources,
    theme::{LoadThemeError, PresentationTheme, PresentationThemeSet},
};
