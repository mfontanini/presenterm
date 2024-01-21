use super::printer::{PrintImage, PrintImageError, PrintOptions, RegisterImageError, ResourceProperties};
use base64::{engine::general_purpose::STANDARD, Engine};
use image::{codecs::gif::GifDecoder, io::Reader, AnimationDecoder, Delay, DynamicImage, EncodableLayout, RgbaImage};
use rand::Rng;
use std::{
    fmt,
    fs::{self, File},
    io::{self, BufReader},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU32, Ordering},
};
use tempfile::{tempdir, TempDir};

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

struct GifFrame<T> {
    delay: Delay,
    buffer: T,
}

pub struct KittyPrinter {
    mode: KittyMode,
    base_directory: TempDir,
    next: AtomicU32,
}

impl KittyPrinter {
    pub(crate) fn new(mode: KittyMode) -> io::Result<Self> {
        let base_directory = tempdir()?;
        Ok(Self { mode, base_directory, next: Default::default() })
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
        let options = vec![
            ControlOption::Format(ImageFormat::Rgba),
            ControlOption::Action(Action::TransmitAndDisplay),
            ControlOption::Width(dimensions.0),
            ControlOption::Height(dimensions.1),
            ControlOption::Columns(print_options.columns),
            ControlOption::Rows(print_options.rows),
        ];

        match &buffer {
            KittyBuffer::Filesystem(path) => Self::print_local(options, path, writer),
            KittyBuffer::Memory(buffer) => Self::print_remote(options, buffer, writer, false),
        }
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
        let image_id = rand::thread_rng().gen();
        for (frame_id, frame) in frames.iter().enumerate() {
            let (num, denom) = frame.delay.numer_denom_ms();
            // default to 100ms in case somehow the denomiator is 0
            let delay = num.checked_div(denom).unwrap_or(100);
            let mut options = vec![
                ControlOption::Format(ImageFormat::Rgba),
                ControlOption::ImageId(image_id),
                ControlOption::Width(dimensions.0),
                ControlOption::Height(dimensions.1),
            ];
            if frame_id == 0 {
                options.extend([
                    ControlOption::Action(Action::TransmitAndDisplay),
                    ControlOption::Columns(print_options.columns),
                    ControlOption::Rows(print_options.rows),
                ]);
            } else {
                options.extend([ControlOption::Action(Action::TransmitFrame), ControlOption::Delay(delay)]);
            }

            let is_frame = frame_id > 0;
            match &frame.buffer {
                KittyBuffer::Filesystem(path) => Self::print_local(options, path, writer)?,
                KittyBuffer::Memory(buffer) => Self::print_remote(options, buffer, writer, is_frame)?,
            };

            if frame_id == 0 {
                let options = &[
                    ControlOption::Action(Action::Animate),
                    ControlOption::ImageId(image_id),
                    ControlOption::FrameId(1),
                    ControlOption::Loops(1),
                ];
                let command = ControlCommand(options, "");
                write!(writer, "{command}")?;
            } else if frame_id == 1 {
                let options = &[
                    ControlOption::Action(Action::Animate),
                    ControlOption::ImageId(image_id),
                    ControlOption::FrameId(1),
                    ControlOption::AnimationState(2),
                ];
                let command = ControlCommand(options, "");
                write!(writer, "{command}")?;
            }
        }
        let options = &[
            ControlOption::Action(Action::Animate),
            ControlOption::ImageId(image_id),
            ControlOption::FrameId(1),
            ControlOption::AnimationState(3),
            ControlOption::Loops(1),
        ];
        let command = ControlCommand(options, "");
        write!(writer, "{command}")?;
        Ok(())
    }

    fn print_local<W>(mut options: Vec<ControlOption>, path: &Path, writer: &mut W) -> Result<(), PrintImageError>
    where
        W: io::Write,
    {
        let Some(path) = path.to_str() else {
            return Err(PrintImageError::other("path is not valid utf8"));
        };
        let encoded_path = STANDARD.encode(path);
        options.push(ControlOption::Medium(TransmissionMedium::LocalFile));

        let command = ControlCommand(&options, &encoded_path);
        write!(writer, "{command}")?;
        Ok(())
    }

    fn print_remote<W>(
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
            let command = ControlCommand(&options, payload);
            write!(writer, "{command}")?;

            options.clear();
            if is_frame {
                options.push(ControlOption::Action(Action::TransmitFrame));
            }
        }
        Ok(())
    }

    fn load_raw_resource(path: &Path) -> Result<RawResource, RegisterImageError> {
        let file = File::open(path)?;
        if path.extension().unwrap_or_default() == "gif" {
            let decoder = GifDecoder::new(file)?;
            let mut frames = Vec::new();
            for frame in decoder.into_frames() {
                let frame = frame?;
                let frame = GifFrame { delay: frame.delay(), buffer: frame.into_buffer() };
                frames.push(frame);
            }
            Ok(RawResource::Gif(frames))
        } else {
            let reader = Reader::new(BufReader::new(file)).with_guessed_format()?;
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

struct ControlCommand<'a, D>(&'a [ControlOption], D);

impl<'a, D: fmt::Display> fmt::Display for ControlCommand<'a, D> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "\x1b_G")?;
        let options = self.0.iter().chain([&ControlOption::Quiet(2)]);
        for (index, option) in options.enumerate() {
            if index > 0 {
                write!(f, ",")?;
            }
            write!(f, "{option}")?;
        }
        write!(f, ";{}\x1b\\", &self.1)?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
enum ControlOption {
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
        }
    }
}

#[derive(Debug, Clone)]
enum ImageFormat {
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
enum TransmissionMedium {
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
enum Action {
    Animate,
    TransmitAndDisplay,
    TransmitFrame,
}

impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Action::*;
        let value = match self {
            Animate => 'a',
            TransmitAndDisplay => 'T',
            TransmitFrame => 'f',
        };
        write!(f, "{value}")
    }
}
