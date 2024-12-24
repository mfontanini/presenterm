use super::printer::{PrintImage, PrintImageError, PrintOptions, RegisterImageError, ResourceProperties};
use crate::style::Color;
use base64::{Engine, engine::general_purpose::STANDARD};
use crossterm::{QueueableCommand, cursor::MoveToColumn, style::SetForegroundColor};
use image::{AnimationDecoder, Delay, DynamicImage, EncodableLayout, ImageReader, RgbaImage, codecs::gif::GifDecoder};
use rand::Rng;
use std::{
    fmt,
    fs::{self, File},
    io::{self, BufReader, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU32, Ordering},
};
use tempfile::{TempDir, tempdir};

const IMAGE_PLACEHOLDER: &str = "\u{10EEEE}";
const DIACRITICS: &[u32] = &[
    0x305, 0x30d, 0x30e, 0x310, 0x312, 0x33d, 0x33e, 0x33f, 0x346, 0x34a, 0x34b, 0x34c, 0x350, 0x351, 0x352, 0x357,
    0x35b, 0x363, 0x364, 0x365, 0x366, 0x367, 0x368, 0x369, 0x36a, 0x36b, 0x36c, 0x36d, 0x36e, 0x36f, 0x483, 0x484,
    0x485, 0x486, 0x487, 0x592, 0x593, 0x594, 0x595, 0x597, 0x598, 0x599, 0x59c, 0x59d, 0x59e, 0x59f, 0x5a0, 0x5a1,
    0x5a8, 0x5a9, 0x5ab, 0x5ac, 0x5af, 0x5c4, 0x610, 0x611, 0x612, 0x613, 0x614, 0x615, 0x616, 0x617, 0x657, 0x658,
    0x659, 0x65a, 0x65b, 0x65d, 0x65e, 0x6d6, 0x6d7, 0x6d8, 0x6d9, 0x6da, 0x6db, 0x6dc, 0x6df, 0x6e0, 0x6e1, 0x6e2,
    0x6e4, 0x6e7, 0x6e8, 0x6eb, 0x6ec, 0x730, 0x732, 0x733, 0x735, 0x736, 0x73a, 0x73d, 0x73f, 0x740, 0x741, 0x743,
    0x745, 0x747, 0x749, 0x74a, 0x7eb, 0x7ec, 0x7ed, 0x7ee, 0x7ef, 0x7f0, 0x7f1, 0x7f3, 0x816, 0x817, 0x818, 0x819,
    0x81b, 0x81c, 0x81d, 0x81e, 0x81f, 0x820, 0x821, 0x822, 0x823, 0x825, 0x826, 0x827, 0x829, 0x82a, 0x82b, 0x82c,
    0x82d, 0x951, 0x953, 0x954, 0xf82, 0xf83, 0xf86, 0xf87, 0x135d, 0x135e, 0x135f, 0x17dd, 0x193a, 0x1a17, 0x1a75,
    0x1a76, 0x1a77, 0x1a78, 0x1a79, 0x1a7a, 0x1a7b, 0x1a7c, 0x1b6b, 0x1b6d, 0x1b6e, 0x1b6f, 0x1b70, 0x1b71, 0x1b72,
    0x1b73, 0x1cd0, 0x1cd1, 0x1cd2, 0x1cda, 0x1cdb, 0x1ce0, 0x1dc0, 0x1dc1, 0x1dc3, 0x1dc4, 0x1dc5, 0x1dc6, 0x1dc7,
    0x1dc8, 0x1dc9, 0x1dcb, 0x1dcc, 0x1dd1, 0x1dd2, 0x1dd3, 0x1dd4, 0x1dd5, 0x1dd6, 0x1dd7, 0x1dd8, 0x1dd9, 0x1dda,
    0x1ddb, 0x1ddc, 0x1ddd, 0x1dde, 0x1ddf, 0x1de0, 0x1de1, 0x1de2, 0x1de3, 0x1de4, 0x1de5, 0x1de6, 0x1dfe, 0x20d0,
    0x20d1, 0x20d4, 0x20d5, 0x20d6, 0x20d7, 0x20db, 0x20dc, 0x20e1, 0x20e7, 0x20e9, 0x20f0, 0x2cef, 0x2cf0, 0x2cf1,
    0x2de0, 0x2de1, 0x2de2, 0x2de3, 0x2de4, 0x2de5, 0x2de6, 0x2de7, 0x2de8, 0x2de9, 0x2dea, 0x2deb, 0x2dec, 0x2ded,
    0x2dee, 0x2def, 0x2df0, 0x2df1, 0x2df2, 0x2df3, 0x2df4, 0x2df5, 0x2df6, 0x2df7, 0x2df8, 0x2df9, 0x2dfa, 0x2dfb,
    0x2dfc, 0x2dfd, 0x2dfe, 0x2dff, 0xa66f, 0xa67c, 0xa67d, 0xa6f0, 0xa6f1, 0xa8e0, 0xa8e1, 0xa8e2, 0xa8e3, 0xa8e4,
    0xa8e5, 0xa8e6, 0xa8e7, 0xa8e8, 0xa8e9, 0xa8ea, 0xa8eb, 0xa8ec, 0xa8ed, 0xa8ee, 0xa8ef, 0xa8f0, 0xa8f1, 0xaab0,
    0xaab2, 0xaab3, 0xaab7, 0xaab8, 0xaabe, 0xaabf, 0xaac1, 0xfe20, 0xfe21, 0xfe22, 0xfe23, 0xfe24, 0xfe25, 0xfe26,
    0x10a0f, 0x10a38, 0x1d185, 0x1d186, 0x1d187, 0x1d188, 0x1d189, 0x1d1aa, 0x1d1ab, 0x1d1ac, 0x1d1ad, 0x1d242,
    0x1d243, 0x1d244,
];

enum GenericResource<B> {
    Image(B),
    Gif(Vec<GifFrame<B>>),
}

type RawResource = GenericResource<RgbaImage>;

impl RawResource {
    fn into_memory_resource(self) -> KittyResource {
        match self {
            Self::Image(image) => KittyResource {
                dimensions: image.dimensions(),
                resource: GenericResource::Image(KittyBuffer::Memory(image.into_raw())),
            },
            Self::Gif(frames) => {
                let dimensions = frames[0].buffer.dimensions();
                let frames = frames
                    .into_iter()
                    .map(|frame| GifFrame { delay: frame.delay, buffer: KittyBuffer::Memory(frame.buffer.into_raw()) })
                    .collect();
                let resource = GenericResource::Gif(frames);
                KittyResource { dimensions, resource }
            }
        }
    }
}

pub(crate) struct KittyResource {
    dimensions: (u32, u32),
    resource: GenericResource<KittyBuffer>,
}

impl ResourceProperties for KittyResource {
    fn dimensions(&self) -> (u32, u32) {
        self.dimensions
    }
}

enum KittyBuffer {
    Filesystem(PathBuf),
    Memory(Vec<u8>),
}

impl Drop for KittyBuffer {
    fn drop(&mut self) {
        if let Self::Filesystem(path) = self {
            let _ = fs::remove_file(path);
        }
    }
}

struct GifFrame<T> {
    delay: Delay,
    buffer: T,
}

pub struct KittyPrinter {
    mode: KittyMode,
    tmux: bool,
    base_directory: TempDir,
    next: AtomicU32,
}

impl KittyPrinter {
    pub(crate) fn new(mode: KittyMode, tmux: bool) -> io::Result<Self> {
        let base_directory = tempdir()?;
        Ok(Self { mode, tmux, base_directory, next: Default::default() })
    }

    fn allocate_tempfile(&self) -> PathBuf {
        let file_number = self.next.fetch_add(1, Ordering::AcqRel);
        self.base_directory.path().join(file_number.to_string())
    }

    fn persist_image(&self, image: RgbaImage) -> io::Result<KittyResource> {
        let path = self.allocate_tempfile();
        fs::write(&path, image.as_bytes())?;

        let buffer = KittyBuffer::Filesystem(path);
        let resource = KittyResource { dimensions: image.dimensions(), resource: GenericResource::Image(buffer) };
        Ok(resource)
    }

    fn persist_gif(&self, frames: Vec<GifFrame<RgbaImage>>) -> io::Result<KittyResource> {
        let mut persisted_frames = Vec::new();
        let mut dimensions = (0, 0);
        for frame in frames {
            let path = self.allocate_tempfile();
            fs::write(&path, frame.buffer.as_bytes())?;
            dimensions = frame.buffer.dimensions();

            let frame = GifFrame { delay: frame.delay, buffer: KittyBuffer::Filesystem(path) };
            persisted_frames.push(frame);
        }
        Ok(KittyResource { dimensions, resource: GenericResource::Gif(persisted_frames) })
    }

    fn persist_resource(&self, resource: RawResource) -> io::Result<KittyResource> {
        match resource {
            RawResource::Image(image) => self.persist_image(image),
            RawResource::Gif(frames) => self.persist_gif(frames),
        }
    }

    fn generate_image_id() -> u32 {
        rand::thread_rng().gen_range(1..u32::MAX)
    }

    fn print_image<W>(
        &self,
        dimensions: (u32, u32),
        buffer: &KittyBuffer,
        writer: &mut W,
        print_options: &PrintOptions,
    ) -> Result<(), PrintImageError>
    where
        W: io::Write,
    {
        let mut options = vec![
            ControlOption::Format(ImageFormat::Rgba),
            ControlOption::Action(Action::TransmitAndDisplay),
            ControlOption::Width(dimensions.0),
            ControlOption::Height(dimensions.1),
            ControlOption::Columns(print_options.columns),
            ControlOption::Rows(print_options.rows),
            ControlOption::ZIndex(print_options.z_index),
            ControlOption::Quiet(2),
        ];
        let mut image_id = 0;
        if self.tmux {
            image_id = Self::generate_image_id();
            options.extend([ControlOption::UnicodePlaceholder, ControlOption::ImageId(image_id)]);
        }

        match &buffer {
            KittyBuffer::Filesystem(path) => self.print_local(options, path, writer)?,
            KittyBuffer::Memory(buffer) => self.print_remote(options, buffer, writer, false)?,
        };
        if self.tmux {
            self.print_unicode_placeholders(writer, print_options, image_id)?;
        }

        Ok(())
    }

    fn print_gif<W>(
        &self,
        dimensions: (u32, u32),
        frames: &[GifFrame<KittyBuffer>],
        writer: &mut W,
        print_options: &PrintOptions,
    ) -> Result<(), PrintImageError>
    where
        W: io::Write,
    {
        let image_id = Self::generate_image_id();
        for (frame_id, frame) in frames.iter().enumerate() {
            let (num, denom) = frame.delay.numer_denom_ms();
            // default to 100ms in case somehow the denominator is 0
            let delay = num.checked_div(denom).unwrap_or(100);
            let mut options = vec![
                ControlOption::Format(ImageFormat::Rgba),
                ControlOption::ImageId(image_id),
                ControlOption::Width(dimensions.0),
                ControlOption::Height(dimensions.1),
                ControlOption::ZIndex(print_options.z_index),
                ControlOption::Quiet(2),
            ];
            if frame_id == 0 {
                options.extend([
                    ControlOption::Action(Action::TransmitAndDisplay),
                    ControlOption::Columns(print_options.columns),
                    ControlOption::Rows(print_options.rows),
                ]);
                if self.tmux {
                    options.push(ControlOption::UnicodePlaceholder);
                }
            } else {
                options.extend([ControlOption::Action(Action::TransmitFrame), ControlOption::Delay(delay)]);
            }

            let is_frame = frame_id > 0;
            match &frame.buffer {
                KittyBuffer::Filesystem(path) => self.print_local(options, path, writer)?,
                KittyBuffer::Memory(buffer) => self.print_remote(options, buffer, writer, is_frame)?,
            };

            if frame_id == 0 {
                let options = &[
                    ControlOption::Action(Action::Animate),
                    ControlOption::ImageId(image_id),
                    ControlOption::FrameId(1),
                    ControlOption::Loops(1),
                ];
                let command = self.make_command(options, "");
                write!(writer, "{command}")?;
            } else if frame_id == 1 {
                let options = &[
                    ControlOption::Action(Action::Animate),
                    ControlOption::ImageId(image_id),
                    ControlOption::FrameId(1),
                    ControlOption::AnimationState(2),
                ];
                let command = self.make_command(options, "");
                write!(writer, "{command}")?;
            }
        }
        if self.tmux {
            self.print_unicode_placeholders(writer, print_options, image_id)?;
        }
        let options = &[
            ControlOption::Action(Action::Animate),
            ControlOption::ImageId(image_id),
            ControlOption::FrameId(1),
            ControlOption::AnimationState(3),
            ControlOption::Loops(1),
            ControlOption::Quiet(2),
        ];
        let command = self.make_command(options, "");
        write!(writer, "{command}")?;
        Ok(())
    }

    fn make_command<'a, P>(&self, options: &'a [ControlOption], payload: P) -> ControlCommand<'a, P> {
        ControlCommand { options, payload, tmux: self.tmux }
    }

    fn print_local<W>(
        &self,
        mut options: Vec<ControlOption>,
        path: &Path,
        writer: &mut W,
    ) -> Result<(), PrintImageError>
    where
        W: io::Write,
    {
        let Some(path) = path.to_str() else {
            return Err(PrintImageError::other("path is not valid utf8"));
        };
        let encoded_path = STANDARD.encode(path);
        options.push(ControlOption::Medium(TransmissionMedium::LocalFile));

        let command = self.make_command(&options, &encoded_path);
        write!(writer, "{command}")?;
        Ok(())
    }

    fn print_remote<W>(
        &self,
        mut options: Vec<ControlOption>,
        frame: &[u8],
        writer: &mut W,
        is_frame: bool,
    ) -> Result<(), PrintImageError>
    where
        W: io::Write,
    {
        options.push(ControlOption::Medium(TransmissionMedium::Direct));

        let payload = STANDARD.encode(frame);
        let chunk_size = 4096;
        let mut index = 0;
        while index < payload.len() {
            let start = index;
            let end = payload.len().min(start + chunk_size);
            index = end;

            let more = end != payload.len();
            options.push(ControlOption::MoreData(more));

            let payload = &payload[start..end];
            let command = self.make_command(&options, payload);
            write!(writer, "{command}")?;

            options.clear();
            if is_frame {
                options.push(ControlOption::Action(Action::TransmitFrame));
            }
        }
        Ok(())
    }

    fn print_unicode_placeholders<W: Write>(
        &self,
        writer: &mut W,
        options: &PrintOptions,
        image_id: u32,
    ) -> Result<(), PrintImageError> {
        let color = Color::new((image_id >> 16) as u8, (image_id >> 8) as u8, image_id as u8);
        writer.queue(SetForegroundColor(color.into()))?;
        if options.rows.max(options.columns) >= DIACRITICS.len() as u16 {
            return Err(PrintImageError::other("image is too large to fit in tmux"));
        }

        let last_byte = char::from_u32(DIACRITICS[(image_id >> 24) as usize]).unwrap();
        for row in 0..options.rows {
            let row_diacritic = char::from_u32(DIACRITICS[row as usize]).unwrap();
            for column in 0..options.columns {
                let column_diacritic = char::from_u32(DIACRITICS[column as usize]).unwrap();
                write!(writer, "{IMAGE_PLACEHOLDER}{row_diacritic}{column_diacritic}{last_byte}")?;
            }
            if row != options.rows - 1 {
                writeln!(writer)?;
            }
            writer.queue(MoveToColumn(options.cursor_position.column))?;
        }
        Ok(())
    }

    fn load_raw_resource(path: &Path) -> Result<RawResource, RegisterImageError> {
        let file = File::open(path)?;
        if path.extension().unwrap_or_default() == "gif" {
            let decoder = GifDecoder::new(BufReader::new(file))?;
            let mut frames = Vec::new();
            for frame in decoder.into_frames() {
                let frame = frame?;
                let frame = GifFrame { delay: frame.delay(), buffer: frame.into_buffer() };
                frames.push(frame);
            }
            Ok(RawResource::Gif(frames))
        } else {
            let reader = ImageReader::new(BufReader::new(file)).with_guessed_format()?;
            let image = reader.decode()?;
            Ok(RawResource::Image(image.into_rgba8()))
        }
    }
}

impl PrintImage for KittyPrinter {
    type Resource = KittyResource;

    fn register_image(&self, image: DynamicImage) -> Result<Self::Resource, RegisterImageError> {
        let resource = RawResource::Image(image.into_rgba8());
        let resource = match &self.mode {
            KittyMode::Local => self.persist_resource(resource)?,
            KittyMode::Remote => resource.into_memory_resource(),
        };
        Ok(resource)
    }

    fn register_resource<P: AsRef<Path>>(&self, path: P) -> Result<Self::Resource, RegisterImageError> {
        let resource = Self::load_raw_resource(path.as_ref())?;
        let resource = match &self.mode {
            KittyMode::Local => self.persist_resource(resource)?,
            KittyMode::Remote => resource.into_memory_resource(),
        };
        Ok(resource)
    }

    fn print<W: std::io::Write>(
        &self,
        image: &Self::Resource,
        options: &PrintOptions,
        writer: &mut W,
    ) -> Result<(), PrintImageError> {
        match &image.resource {
            GenericResource::Image(resource) => self.print_image(image.dimensions, resource, writer, options)?,
            GenericResource::Gif(frames) => self.print_gif(image.dimensions, frames, writer, options)?,
        };
        writeln!(writer)?;
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub enum KittyMode {
    Local,
    Remote,
}

pub(crate) struct ControlCommand<'a, D> {
    pub(crate) options: &'a [ControlOption],
    pub(crate) payload: D,
    pub(crate) tmux: bool,
}

impl<D: fmt::Display> fmt::Display for ControlCommand<'_, D> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.tmux {
            write!(f, "\x1bPtmux;\x1b")?;
        }
        write!(f, "\x1b_G")?;
        for (index, option) in self.options.iter().enumerate() {
            if index > 0 {
                write!(f, ",")?;
            }
            write!(f, "{option}")?;
        }
        write!(f, ";{}", &self.payload)?;
        if self.tmux {
            write!(f, "\x1b\x1b\\\x1b\\")?;
        } else {
            write!(f, "\x1b\\")?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub(crate) enum ControlOption {
    Action(Action),
    Format(ImageFormat),
    Medium(TransmissionMedium),
    Width(u32),
    Height(u32),
    Columns(u16),
    Rows(u16),
    MoreData(bool),
    ImageId(u32),
    FrameId(u32),
    Delay(u32),
    AnimationState(u32),
    Loops(u32),
    Quiet(u32),
    ZIndex(i32),
    UnicodePlaceholder,
}

impl fmt::Display for ControlOption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use ControlOption::*;
        match self {
            Action(action) => write!(f, "a={action}"),
            Format(format) => write!(f, "f={format}"),
            Medium(medium) => write!(f, "t={medium}"),
            Width(width) => write!(f, "s={width}"),
            Height(height) => write!(f, "v={height}"),
            Columns(columns) => write!(f, "c={columns}"),
            Rows(rows) => write!(f, "r={rows}"),
            MoreData(true) => write!(f, "m=1"),
            MoreData(false) => write!(f, "m=0"),
            ImageId(id) => write!(f, "i={id}"),
            FrameId(id) => write!(f, "r={id}"),
            Delay(delay) => write!(f, "z={delay}"),
            AnimationState(state) => write!(f, "s={state}"),
            Loops(count) => write!(f, "v={count}"),
            Quiet(option) => write!(f, "q={option}"),
            ZIndex(index) => write!(f, "z={index}"),
            UnicodePlaceholder => write!(f, "U=1"),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) enum ImageFormat {
    Rgba,
}

impl fmt::Display for ImageFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use ImageFormat::*;
        let value = match self {
            Rgba => 32,
        };
        write!(f, "{value}")
    }
}

#[derive(Debug, Clone)]
pub(crate) enum TransmissionMedium {
    Direct,
    LocalFile,
}

impl fmt::Display for TransmissionMedium {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use TransmissionMedium::*;
        let value = match self {
            Direct => 'd',
            LocalFile => 'f',
        };
        write!(f, "{value}")
    }
}

#[derive(Debug, Clone)]
pub(crate) enum Action {
    Animate,
    TransmitAndDisplay,
    TransmitFrame,
    Query,
}

impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Action::*;
        let value = match self {
            Animate => 'a',
            TransmitAndDisplay => 'T',
            TransmitFrame => 'f',
            Query => 'q',
        };
        write!(f, "{value}")
    }
}
