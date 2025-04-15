use super::{
    RenderError,
    engine::{RenderEngine, RenderEngineOptions},
};
use crate::{
    ImageRegistry, WindowSize,
    presentation::Presentation,
    terminal::{
        image::{Image, ImageSource},
        printer::{TerminalCommand, TerminalError, TerminalIo},
    },
};
use std::thread;
use unicode_width::UnicodeWidthStr;

pub(crate) struct AsciiScaler {
    options: RenderEngineOptions,
    registry: ImageRegistry,
}

impl AsciiScaler {
    pub(crate) fn new(options: RenderEngineOptions, registry: ImageRegistry) -> Self {
        Self { options, registry }
    }

    pub(crate) fn process(self, presentation: &Presentation, dimensions: &WindowSize) -> Result<(), RenderError> {
        let mut collector = ImageCollector::default();
        for slide in presentation.iter_slides() {
            let engine = RenderEngine::new(&mut collector, dimensions.clone(), self.options.clone());
            engine.render(slide.iter_operations())?;
        }
        thread::spawn(move || Self::scale(collector.images, self.registry));
        Ok(())
    }

    fn scale(images: Vec<ScalableImage>, registry: ImageRegistry) {
        for image in images {
            let ascii_image = registry.as_ascii(&image.image);
            ascii_image.cache_scaling(image.columns, image.rows);
        }
    }
}

struct ScalableImage {
    image: Image,
    rows: u16,
    columns: u16,
}

struct ImageCollector {
    current_column: u16,
    current_row: u16,
    current_row_height: u16,
    images: Vec<ScalableImage>,
}

impl Default for ImageCollector {
    fn default() -> Self {
        Self { current_row: 0, current_column: 0, current_row_height: 1, images: Default::default() }
    }
}

impl TerminalIo for ImageCollector {
    fn execute(&mut self, command: &TerminalCommand<'_>) -> Result<(), TerminalError> {
        use TerminalCommand::*;
        match command {
            MoveTo { column, row } => {
                self.current_column = *column;
                self.current_row = *row;
            }
            MoveToRow(row) => self.current_row = *row,
            MoveToColumn(column) => self.current_column = *column,
            MoveDown(amount) => self.current_row = self.current_row.saturating_add(*amount),
            MoveRight(amount) => self.current_column = self.current_column.saturating_add(*amount),
            MoveLeft(amount) => self.current_column = self.current_column.saturating_sub(*amount),
            MoveToNextLine => {
                self.current_row = self.current_row.saturating_add(1);
                self.current_column = 0;
                self.current_row_height = 1;
            }
            PrintText { content, style } => {
                self.current_column = self.current_column.saturating_add(content.width() as u16);
                self.current_row_height = self.current_row_height.max(style.size as u16);
            }
            PrintImage { image, options } => {
                // we can only really cache filesystem images for now
                if matches!(image.source, ImageSource::Filesystem(_)) {
                    let image =
                        ScalableImage { image: image.clone(), rows: options.rows * 2, columns: options.columns };
                    self.images.push(image);
                }
            }
            ClearScreen => {
                self.current_column = 0;
                self.current_row = 0;
                self.current_row_height = 1;
            }
            BeginUpdate | EndUpdate | Flush | SetColors(_) | SetBackgroundColor(_) => (),
        };
        Ok(())
    }

    fn cursor_row(&self) -> u16 {
        self.current_row
    }
}
