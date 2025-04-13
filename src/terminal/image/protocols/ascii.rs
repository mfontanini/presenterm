use crate::{
    markdown::text_style::{Color, Colors, TextStyle},
    terminal::{
        image::printer::{ImageProperties, ImageSpec, PrintImage, PrintImageError, PrintOptions, RegisterImageError},
        printer::{TerminalCommand, TerminalIo},
    },
};
use image::{DynamicImage, GenericImageView, Pixel, Rgba, imageops::FilterType};
use itertools::Itertools;
use std::{fs, ops::Deref};

const TOP_CHAR: &str = "▀";
const BOTTOM_CHAR: &str = "▄";

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

    fn register(&self, spec: ImageSpec) -> Result<Self::Image, RegisterImageError> {
        let image = match spec {
            ImageSpec::Generated(image) => image,
            ImageSpec::Filesystem(path) => {
                let contents = fs::read(path)?;
                image::load_from_memory(&contents)?
            }
        };
        Ok(AsciiImage(image))
    }

    fn print<T>(&self, image: &Self::Image, options: &PrintOptions, terminal: &mut T) -> Result<(), PrintImageError>
    where
        T: TerminalIo,
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
            let top_row = rows.next().unwrap();
            let mut bottom_row = rows.next();
            for top_pixel in top_row {
                let bottom_pixel = bottom_row.as_mut().and_then(|pixels| pixels.next());

                // Get pixel colors for both of these. At this point the special case for the odd
                // number of rows disappears as we treat a transparent pixel and a non-existent
                // one the same: they're simply transparent.
                let background = default_background;
                let top = Self::pixel_color(top_pixel, background);
                let bottom = bottom_pixel.and_then(|c| Self::pixel_color(c, background));
                let command = match (top, bottom) {
                    (Some(top), Some(bottom)) => TerminalCommand::PrintText {
                        content: TOP_CHAR,
                        style: TextStyle::default().fg_color(top).bg_color(bottom),
                    },
                    (Some(top), None) => TerminalCommand::PrintText {
                        content: TOP_CHAR,
                        style: TextStyle::colored(Colors { foreground: Some(top), background: default_background }),
                    },
                    (None, Some(bottom)) => TerminalCommand::PrintText {
                        content: BOTTOM_CHAR,
                        style: TextStyle::colored(Colors { foreground: Some(bottom), background: default_background }),
                    },
                    (None, None) => TerminalCommand::MoveRight(1),
                };
                terminal.execute(&command)?;
            }
            terminal.execute(&TerminalCommand::MoveDown(1))?;
            terminal.execute(&TerminalCommand::MoveLeft(options.columns))?;
        }
        Ok(())
    }
}
