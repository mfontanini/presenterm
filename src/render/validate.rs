use super::{properties::WindowSize, terminal::TerminalWrite};
use crate::{
    ImagePrinter,
    presentation::Presentation,
    render::{
        draw::RenderError,
        engine::{RenderEngine, RenderEngineOptions},
        terminal::Terminal,
    },
};
use std::{io, sync::Arc};

pub(crate) struct OverflowValidator;

impl OverflowValidator {
    pub(crate) fn validate(presentation: &Presentation, dimensions: WindowSize) -> Result<(), OverflowError> {
        let printer = Arc::new(ImagePrinter::Null);
        for (index, slide) in presentation.iter_slides().enumerate() {
            let index = index + 1;
            let mut terminal = Terminal::new(io::Empty::default(), printer.clone()).map_err(RenderError::from)?;
            let options = RenderEngineOptions { validate_overflows: true };
            let engine = RenderEngine::new(&mut terminal, dimensions.clone(), options);
            match engine.render(slide.iter_visible_operations()) {
                Ok(()) => (),
                Err(RenderError::HorizontalOverflow) => return Err(OverflowError::Horizontal(index)),
                Err(RenderError::VerticalOverflow) => return Err(OverflowError::Vertical(index)),
                Err(e) => return Err(OverflowError::Render(e)),
            };
        }
        Ok(())
    }
}

impl TerminalWrite for io::Empty {
    fn init(&mut self) -> io::Result<()> {
        Ok(())
    }

    fn deinit(&mut self) {}
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum OverflowError {
    #[error("presentation overflows horizontally on slide {0}")]
    Horizontal(usize),

    #[error("presentation overflows vertically on slide {0}")]
    Vertical(usize),

    #[error(transparent)]
    Render(#[from] RenderError),
}
