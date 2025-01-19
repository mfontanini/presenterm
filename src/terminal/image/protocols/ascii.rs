use crate::terminal::image::printer::{ImageProperties, PrintImage, PrintImageError, PrintOptions, RegisterImageError};
use crossterm::{
    QueueableCommand,
    cursor::{MoveRight, MoveToColumn},
    style::{Color, Stylize},
};
use image::{DynamicImage, GenericImageView, Pixel, Rgba, imageops::FilterType};
use itertools::Itertools;
use std::{fs, ops::Deref};

const TOP_CHAR: char = '▀';
const BOTTOM_CHAR: char = '▄';

pub(crate) struct AsciiImage(DynamicImage);

impl ImageProperties for AsciiImage {
    fn dimensions(&self) -> (u32, u32) {
        self.0.dimensions()
    }
}

impl From<DynamicImage> for AsciiImage {
    fn from(image: DynamicImage) -> Self {
        let image = image.into_rgba8();
        Self(image.into())
    }
}

impl Deref for AsciiImage {
    type Target = DynamicImage;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Default)]
pub struct AsciiPrinter;

impl AsciiPrinter {
    fn pixel_color(pixel: &Rgba<u8>, background: Option<Color>) -> Option<Color> {
        let [r, g, b, alpha] = pixel.0;
        if alpha == 0 {
            None
        } else if alpha < 255 {
            // For alpha > 0 && < 255, we blend it with the background color (if any). This helps
            // smooth the image's borders.
            let mut pixel = *pixel;
            match background {
                Some(Color::Rgb { r, g, b }) => {
                    pixel.blend(&Rgba([r, g, b, 255 - alpha]));
                    Some(Color::Rgb { r: pixel[0], g: pixel[1], b: pixel[2] })
                }
                // For transparent backgrounds, we can't really know whether we should blend it
                // towards light or dark.
                None | Some(_) => Some(Color::Rgb { r, g, b }),
            }
        } else {
            Some(Color::Rgb { r, g, b })
        }
    }
}

impl PrintImage for AsciiPrinter {
    type Image = AsciiImage;

    fn register(&self, image: image::DynamicImage) -> Result<Self::Image, RegisterImageError> {
        Ok(AsciiImage(image))
    }

    fn register_from_path<P: AsRef<std::path::Path>>(&self, path: P) -> Result<Self::Image, RegisterImageError> {
        let contents = fs::read(path)?;
        let image = image::load_from_memory(&contents)?;
        Ok(AsciiImage(image))
    }

    fn print<W>(&self, image: &Self::Image, options: &PrintOptions, writer: &mut W) -> Result<(), PrintImageError>
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
                let background = options.background_color.map(Color::from);
                let top = Self::pixel_color(top_pixel, background);
                let bottom = bottom_pixel.and_then(|c| Self::pixel_color(c, background));
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
