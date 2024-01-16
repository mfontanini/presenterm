//! Presenterm: a terminal slideshow presentation tool.
//!
//! This is not meant to be used as a crate!

pub(crate) mod custom;
pub(crate) mod diff;
pub(crate) mod execute;
pub(crate) mod export;
pub(crate) mod input;
pub(crate) mod markdown;
pub(crate) mod presentation;
pub(crate) mod presenter;
pub(crate) mod processing;
pub(crate) mod render;
pub(crate) mod request;
pub(crate) mod resource;
pub(crate) mod style;
pub(crate) mod theme;
pub(crate) mod tools;
pub(crate) mod typst;

pub use crate::{
    custom::Config,
    export::{ExportError, Exporter},
    input::source::CommandSource,
    markdown::parse::MarkdownParser,
    presenter::{PresentMode, Presenter, PresenterOptions},
    processing::builder::{PresentationBuilderOptions, Themes},
    render::{
        highlighting::{CodeHighlighter, HighlightThemeSet},
        media::{GraphicsMode, MediaRender},
    },
    request::run_demo,
    resource::Resources,
    theme::{LoadThemeError, PresentationTheme, PresentationThemeSet},
    typst::TypstRender,
};
