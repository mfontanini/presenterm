use super::printer::{PrintImage, PrintImageError, PrintOptions, RegisterImageError, ResourceProperties};
use crossterm::{
    cursor::{MoveRight, MoveToColumn},
    style::{Color, Stylize},
    QueueableCommand,
};
use image::{imageops::FilterType, DynamicImage, GenericImageView, Rgba};
use itertools::Itertools;
use std::{fs, ops::Deref};

const TOP_CHAR: char = '▀';
const BOTTOM_CHAR: char = '▄';

pub(crate) struct AsciiResource(DynamicImage);

impl ResourceProperties for AsciiResource {
    fn dimensions(&self) -> (u32, u32) {
        self.0.dimensions()
    }
}

impl From<DynamicImage> for AsciiResource {
    fn from(image: DynamicImage) -> Self {
        let image = image.into_rgba8();
        Self(image.into())
    }
}

impl Deref for AsciiResource {
    type Target = DynamicImage;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Default)]
pub struct AsciiPrinter;

impl AsciiPrinter {
    fn pixel_color(pixel: &Rgba<u8>) -> Option<Color> {
        let [r, g, b, alpha] = pixel.0;
        if alpha == 0 { None } else { Some(Color::Rgb { r, g, b }) }
    }
}

impl PrintImage for AsciiPrinter {
    type Resource = AsciiResource;

    fn register_image(&self, image: image::DynamicImage) -> Result<Self::Resource, RegisterImageError> {
        Ok(AsciiResource(image))
    }

    fn register_resource<P: AsRef<std::path::Path>>(&self, path: P) -> Result<Self::Resource, RegisterImageError> {
        let contents = fs::read(path)?;
        let image = image::load_from_memory(&contents)?;
        Ok(AsciiResource(image))
    }

    fn print<W>(&self, image: &Self::Resource, options: &PrintOptions, writer: &mut W) -> Result<(), PrintImageError>
    where
        W: std::io::Write,
    {
        // The strategy here is taken from viuer: use half vertical ascii blocks in combination
        // with foreground/background colors to fit 2 vertical pixels per cell. That is, cell (x, y)
        // will contain the pixels at (x, y) and (x, y + 1) combined.
        let image = image.0.resize_exact(options.columns as u32, 2 * options.rows as u32, FilterType::Triangle);
        let image = image.into_rgba8();
        let default_background = options.background_color.map(Color::from);

        // Iterate pixel rows in pairs to be able to merge both pixels in a single iteration.
        // Note that may not have a second row if there's an odd number of them.
        for mut rows in &image.rows().chunks(2) {
            writer.queue(MoveToColumn(options.cursor_position.column))?;

            let top_row = rows.next().unwrap();
            let mut bottom_row = rows.next();
            for top_pixel in top_row {
                let bottom_pixel = bottom_row.as_mut().and_then(|pixels| pixels.next());

                // Get pixel colors for both of these. At this point the special case for the odd
                // number of rows disappears as we treat a transparent pixel and a non-existent
                // one the same: they're simply transparent.
                let top = Self::pixel_color(top_pixel);
                let bottom = bottom_pixel.and_then(Self::pixel_color);
                match (top, bottom) {
                    (Some(top), Some(bottom)) => {
                        write!(writer, "{}", TOP_CHAR.with(top).on(bottom))?;
                    }
                    (Some(top), None) => {
                        write!(writer, "{}", TOP_CHAR.with(top).maybe_on(default_background))?;
                    }
                    (None, Some(bottom)) => {
                        write!(writer, "{}", BOTTOM_CHAR.with(bottom).maybe_on(default_background))?;
                    }
                    (None, None) => {
                        writer.queue(MoveRight(1))?;
                    }
                };
            }
            writeln!(writer)?;
        }
        Ok(())
    }
}

trait StylizeExt: Stylize {
    fn maybe_on(self, color: Option<Color>) -> Self::Styled;
}

impl<T: Stylize> StylizeExt for T {
    fn maybe_on(self, color: Option<Color>) -> Self::Styled {
        match color {
            Some(background) => self.on(background),
            None => self.stylize(),
        }
    }
}
