use super::{
    AuthorPositioning, FooterTemplate, Margin,
    raw::{self, RawColor},
};
use crate::{
    markdown::text_style::{Color, Colors, TextStyle, UndefinedPaletteColorError},
    resource::Resources,
    terminal::image::{Image, printer::RegisterImageError},
};
use std::collections::BTreeMap;

const DEFAULT_CODE_HIGHLIGHT_THEME: &str = "base16-eighties.dark";
const DEFAULT_BLOCK_QUOTE_PREFIX: &str = "▍ ";
const DEFAULT_PROGRESS_BAR_CHAR: char = '█';
const DEFAULT_FOOTER_HEIGHT: u16 = 3;
const DEFAULT_TYPST_HORIZONTAL_MARGIN: u16 = 5;
const DEFAULT_TYPST_VERTICAL_MARGIN: u16 = 7;
const DEFAULT_MERMAID_THEME: &str = "default";
const DEFAULT_MERMAID_BACKGROUND: &str = "transparent";

#[derive(Clone, Debug, Default)]
pub(crate) struct ThemeOptions {
    pub(crate) font_size_supported: bool,
}

impl ThemeOptions {
    fn adjust_font_size(&self, font_size: Option<u8>) -> u8 {
        if !self.font_size_supported { 1 } else { font_size.unwrap_or(1).clamp(1, 7) }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct PresentationTheme {
    pub(crate) slide_title: SlideTitleStyle,
    pub(crate) code: CodeBlockStyle,
    pub(crate) execution_output: ExecutionOutputBlockStyle,
    pub(crate) inline_code: InlineCodeStyle,
    pub(crate) table: Alignment,
    pub(crate) block_quote: BlockQuoteStyle,
    pub(crate) alert: AlertStyle,
    pub(crate) default_style: DefaultStyle,
    pub(crate) headings: HeadingStyles,
    pub(crate) intro_slide: IntroSlideStyle,
    pub(crate) footer: FooterStyle,
    pub(crate) typst: TypstStyle,
    pub(crate) mermaid: MermaidStyle,
    pub(crate) modals: ModalStyle,
    pub(crate) palette: ColorPalette,
}

impl PresentationTheme {
    pub(crate) fn new(
        raw: &raw::PresentationTheme,
        resources: &Resources,
        options: &ThemeOptions,
    ) -> Result<Self, ProcessingThemeError> {
        let raw::PresentationTheme {
            slide_title,
            code,
            execution_output,
            inline_code,
            table,
            block_quote,
            alert,
            default_style,
            headings,
            intro_slide,
            footer,
            typst,
            mermaid,
            modals,
            palette,
            extends: _,
        } = raw;

        let palette = ColorPalette::try_from(palette)?;
        let default_style = DefaultStyle::new(default_style, &palette)?;
        Ok(Self {
            slide_title: SlideTitleStyle::new(slide_title, &palette, options)?,
            code: CodeBlockStyle::new(code),
            execution_output: ExecutionOutputBlockStyle::new(execution_output, &palette)?,
            inline_code: InlineCodeStyle::new(inline_code, &palette)?,
            table: table.clone().unwrap_or_default().into(),
            block_quote: BlockQuoteStyle::new(block_quote, &palette)?,
            alert: AlertStyle::new(alert, &palette)?,
            default_style: default_style.clone(),
            headings: HeadingStyles::new(headings, &palette, options)?,
            intro_slide: IntroSlideStyle::new(intro_slide, &palette, options)?,
            footer: FooterStyle::new(&footer.clone().unwrap_or_default(), &palette, resources)?,
            typst: TypstStyle::new(typst, &palette)?,
            mermaid: MermaidStyle::new(mermaid),
            modals: ModalStyle::new(modals, &default_style, &palette)?,
            palette,
        })
    }

    pub(crate) fn alignment(&self, element: &ElementType) -> Alignment {
        use ElementType::*;

        match element {
            SlideTitle => self.slide_title.alignment,
            Heading1 => self.headings.h1.alignment,
            Heading2 => self.headings.h2.alignment,
            Heading3 => self.headings.h3.alignment,
            Heading4 => self.headings.h4.alignment,
            Heading5 => self.headings.h5.alignment,
            Heading6 => self.headings.h6.alignment,
            Paragraph | List => Default::default(),
            PresentationTitle => self.intro_slide.title.alignment,
            PresentationSubTitle => self.intro_slide.subtitle.alignment,
            PresentationEvent => self.intro_slide.event.alignment,
            PresentationLocation => self.intro_slide.location.alignment,
            PresentationDate => self.intro_slide.date.alignment,
            PresentationAuthor => self.intro_slide.author.alignment,
            Table => self.table,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum ProcessingThemeError {
    #[error(transparent)]
    Palette(#[from] UndefinedPaletteColorError),

    #[error("palette cannot contain other palette colors")]
    PaletteColorInPalette,

    #[error("invalid footer image: {0}")]
    FooterImage(RegisterImageError),
}

#[derive(Clone, Debug)]
pub(crate) struct SlideTitleStyle {
    pub(crate) alignment: Alignment,
    pub(crate) separator: bool,
    pub(crate) padding_top: u8,
    pub(crate) padding_bottom: u8,
    pub(crate) style: TextStyle,
}

impl SlideTitleStyle {
    fn new(
        raw: &raw::SlideTitleStyle,
        palette: &ColorPalette,
        options: &ThemeOptions,
    ) -> Result<Self, ProcessingThemeError> {
        let raw::SlideTitleStyle {
            alignment,
            separator,
            padding_top,
            padding_bottom,
            colors,
            bold,
            italics,
            underlined,
            font_size,
        } = raw;
        let colors = colors.resolve(palette)?;
        let mut style = TextStyle::colored(colors).size(options.adjust_font_size(*font_size));
        if bold.unwrap_or_default() {
            style = style.bold();
        }
        if italics.unwrap_or_default() {
            style = style.italics();
        }
        if underlined.unwrap_or_default() {
            style = style.underlined();
        }
        Ok(Self {
            alignment: alignment.clone().unwrap_or_default().into(),
            separator: *separator,
            padding_top: padding_top.unwrap_or_default(),
            padding_bottom: padding_bottom.unwrap_or_default(),
            style,
        })
    }
}

#[derive(Clone, Debug)]
pub(crate) struct HeadingStyles {
    pub(crate) h1: HeadingStyle,
    pub(crate) h2: HeadingStyle,
    pub(crate) h3: HeadingStyle,
    pub(crate) h4: HeadingStyle,
    pub(crate) h5: HeadingStyle,
    pub(crate) h6: HeadingStyle,
}

impl HeadingStyles {
    fn new(
        raw: &raw::HeadingStyles,
        palette: &ColorPalette,
        options: &ThemeOptions,
    ) -> Result<Self, ProcessingThemeError> {
        let raw::HeadingStyles { h1, h2, h3, h4, h5, h6 } = raw;
        Ok(Self {
            h1: HeadingStyle::new(h1, palette, options)?,
            h2: HeadingStyle::new(h2, palette, options)?,
            h3: HeadingStyle::new(h3, palette, options)?,
            h4: HeadingStyle::new(h4, palette, options)?,
            h5: HeadingStyle::new(h5, palette, options)?,
            h6: HeadingStyle::new(h6, palette, options)?,
        })
    }
}

#[derive(Clone, Debug)]
pub(crate) struct HeadingStyle {
    pub(crate) alignment: Alignment,
    pub(crate) prefix: Option<String>,
    pub(crate) style: TextStyle,
}

impl HeadingStyle {
    fn new(
        raw: &raw::HeadingStyle,
        palette: &ColorPalette,
        options: &ThemeOptions,
    ) -> Result<Self, ProcessingThemeError> {
        let raw::HeadingStyle { alignment, prefix, colors, font_size } = raw;
        let alignment = alignment.clone().unwrap_or_default().into();
        let style = TextStyle::colored(colors.resolve(palette)?).size(options.adjust_font_size(*font_size));
        Ok(Self { alignment, prefix: prefix.clone(), style })
    }
}

#[derive(Clone, Debug)]
pub(crate) struct BlockQuoteStyle {
    pub(crate) alignment: Alignment,
    pub(crate) prefix: String,
    pub(crate) base_style: TextStyle,
    pub(crate) prefix_style: TextStyle,
}

impl BlockQuoteStyle {
    fn new(raw: &raw::BlockQuoteStyle, palette: &ColorPalette) -> Result<Self, ProcessingThemeError> {
        let raw::BlockQuoteStyle { alignment, prefix, colors } = raw;
        let alignment = alignment.clone().unwrap_or_default().into();
        let prefix = prefix.as_deref().unwrap_or(DEFAULT_BLOCK_QUOTE_PREFIX).to_string();
        let base_style = TextStyle::colored(colors.base.resolve(palette)?);
        let mut prefix_style = TextStyle::colored(colors.base.resolve(palette)?);
        if let Some(color) = &colors.prefix {
            prefix_style.colors.foreground = color.resolve(palette)?;
        }
        Ok(Self { alignment, prefix, base_style, prefix_style })
    }
}

#[derive(Clone, Debug)]
pub(crate) struct AlertStyle {
    pub(crate) alignment: Alignment,
    pub(crate) base_style: TextStyle,
    pub(crate) prefix: String,
    pub(crate) styles: AlertTypeStyles,
}

impl AlertStyle {
    fn new(raw: &raw::AlertStyle, palette: &ColorPalette) -> Result<Self, ProcessingThemeError> {
        let raw::AlertStyle { alignment, base_colors, prefix, styles } = raw;
        let alignment = alignment.clone().unwrap_or_default().into();
        let base_style = TextStyle::colored(base_colors.resolve(palette)?);
        let prefix = prefix.as_deref().unwrap_or(DEFAULT_BLOCK_QUOTE_PREFIX).to_string();
        let styles = AlertTypeStyles::new(styles, base_style, palette)?;
        Ok(Self { alignment, base_style, prefix, styles })
    }
}

#[derive(Clone, Debug)]
pub(crate) struct AlertTypeStyles {
    pub(crate) note: AlertTypeStyle,
    pub(crate) tip: AlertTypeStyle,
    pub(crate) important: AlertTypeStyle,
    pub(crate) warning: AlertTypeStyle,
    pub(crate) caution: AlertTypeStyle,
}

impl AlertTypeStyles {
    fn new(
        raw: &raw::AlertTypeStyles,
        base_style: TextStyle,
        palette: &ColorPalette,
    ) -> Result<Self, ProcessingThemeError> {
        let raw::AlertTypeStyles { note, tip, important, warning, caution } = raw;
        Ok(Self {
            note: AlertTypeStyle::new(
                note,
                &AlertTypeDefaults { title: "Note", icon: "󰋽", color: Color::Blue },
                base_style,
                palette,
            )?,
            tip: AlertTypeStyle::new(
                tip,
                &AlertTypeDefaults { title: "Tip", icon: "", color: Color::Green },
                base_style,
                palette,
            )?,
            important: AlertTypeStyle::new(
                important,
                &AlertTypeDefaults { title: "Important", icon: "", color: Color::Cyan },
                base_style,
                palette,
            )?,
            warning: AlertTypeStyle::new(
                warning,
                &AlertTypeDefaults { title: "Warning", icon: "", color: Color::Yellow },
                base_style,
                palette,
            )?,
            caution: AlertTypeStyle::new(
                caution,
                &AlertTypeDefaults { title: "Caution", icon: "󰳦", color: Color::Red },
                base_style,
                palette,
            )?,
        })
    }
}

#[derive(Clone, Debug)]
pub(crate) struct AlertTypeStyle {
    pub(crate) style: TextStyle,
    pub(crate) title: String,
    pub(crate) icon: String,
}

impl AlertTypeStyle {
    fn new(
        raw: &raw::AlertTypeStyle,
        defaults: &AlertTypeDefaults,
        base_style: TextStyle,
        palette: &ColorPalette,
    ) -> Result<Self, ProcessingThemeError> {
        let raw::AlertTypeStyle { color, title, icon, .. } = raw;
        let color = color.as_ref().map(|c| c.resolve(palette)).transpose()?.flatten().unwrap_or(defaults.color);
        let style = base_style.fg_color(color);
        let title = title.as_deref().unwrap_or(defaults.title).to_string();
        let icon = icon.as_deref().unwrap_or(defaults.icon).to_string();
        Ok(Self { style, title, icon })
    }
}

struct AlertTypeDefaults {
    title: &'static str,
    icon: &'static str,
    color: Color,
}

#[derive(Clone, Debug)]
pub(crate) struct IntroSlideStyle {
    pub(crate) title: IntroSlideTitleStyle,
    pub(crate) subtitle: IntroSlideLabelStyle,
    pub(crate) event: IntroSlideLabelStyle,
    pub(crate) location: IntroSlideLabelStyle,
    pub(crate) date: IntroSlideLabelStyle,
    pub(crate) author: AuthorStyle,
    pub(crate) footer: bool,
}

impl IntroSlideStyle {
    fn new(
        raw: &raw::IntroSlideStyle,
        palette: &ColorPalette,
        options: &ThemeOptions,
    ) -> Result<Self, ProcessingThemeError> {
        let raw::IntroSlideStyle { title, subtitle, event, location, date, author, footer } = raw;
        Ok(Self {
            title: IntroSlideTitleStyle::new(title, palette, options)?,
            subtitle: IntroSlideLabelStyle::new(subtitle, palette)?,
            event: IntroSlideLabelStyle::new(event, palette)?,
            location: IntroSlideLabelStyle::new(location, palette)?,
            date: IntroSlideLabelStyle::new(date, palette)?,
            author: AuthorStyle::new(author, palette)?,
            footer: footer.unwrap_or(false),
        })
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct IntroSlideLabelStyle {
    pub(crate) alignment: Alignment,
    pub(crate) style: TextStyle,
}

impl IntroSlideLabelStyle {
    fn new(raw: &raw::BasicStyle, palette: &ColorPalette) -> Result<Self, ProcessingThemeError> {
        let raw::BasicStyle { alignment, colors } = raw;
        let style = TextStyle::colored(colors.resolve(palette)?);
        Ok(Self { alignment: alignment.clone().unwrap_or_default().into(), style })
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct IntroSlideTitleStyle {
    pub(crate) alignment: Alignment,
    pub(crate) style: TextStyle,
}

impl IntroSlideTitleStyle {
    fn new(
        raw: &raw::IntroSlideTitleStyle,
        palette: &ColorPalette,
        options: &ThemeOptions,
    ) -> Result<Self, ProcessingThemeError> {
        let raw::IntroSlideTitleStyle { alignment, colors, font_size } = raw;
        let style = TextStyle::colored(colors.resolve(palette)?).size(options.adjust_font_size(*font_size));
        Ok(Self { alignment: alignment.clone().unwrap_or_default().into(), style })
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct AuthorStyle {
    pub(crate) alignment: Alignment,
    pub(crate) style: TextStyle,
    pub(crate) positioning: AuthorPositioning,
}

impl AuthorStyle {
    fn new(raw: &raw::AuthorStyle, palette: &ColorPalette) -> Result<Self, ProcessingThemeError> {
        let raw::AuthorStyle { alignment, colors, positioning } = raw;
        let style = TextStyle::colored(colors.resolve(palette)?);
        Ok(Self { alignment: alignment.clone().unwrap_or_default().into(), style, positioning: positioning.clone() })
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct DefaultStyle {
    pub(crate) margin: Margin,
    pub(crate) style: TextStyle,
}

impl DefaultStyle {
    fn new(raw: &raw::DefaultStyle, palette: &ColorPalette) -> Result<Self, ProcessingThemeError> {
        let raw::DefaultStyle { margin, colors } = raw;
        let margin = margin.unwrap_or_default();
        let style = TextStyle::colored(colors.resolve(palette)?);
        Ok(Self { margin, style })
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) enum Alignment {
    Left { margin: Margin },
    Right { margin: Margin },
    Center { minimum_margin: Margin, minimum_size: u16 },
}

impl Alignment {
    pub(crate) fn adjust_size(&self, size: u16) -> u16 {
        match self {
            Self::Left { .. } | Self::Right { .. } => size,
            Self::Center { minimum_size, .. } => size.max(*minimum_size),
        }
    }
}

impl From<raw::Alignment> for Alignment {
    fn from(alignment: raw::Alignment) -> Self {
        match alignment {
            raw::Alignment::Left { margin } => Self::Left { margin },
            raw::Alignment::Right { margin } => Self::Right { margin },
            raw::Alignment::Center { minimum_margin, minimum_size } => Self::Center { minimum_margin, minimum_size },
        }
    }
}

impl Default for Alignment {
    fn default() -> Self {
        Self::Left { margin: Margin::Fixed(0) }
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) enum FooterStyle {
    Template {
        left: Option<FooterContent>,
        center: Option<FooterContent>,
        right: Option<FooterTemplate>,
        style: TextStyle,
        height: u16,
    },
    ProgressBar {
        character: char,
        style: TextStyle,
    },
    #[default]
    Empty,
}

impl FooterStyle {
    fn new(
        raw: &raw::FooterStyle,
        palette: &ColorPalette,
        resources: &Resources,
    ) -> Result<Self, ProcessingThemeError> {
        match raw {
            raw::FooterStyle::Template { left, center, right, colors, height } => {
                let left = left.as_ref().map(|t| FooterContent::new(t, resources)).transpose()?;
                let center = center.as_ref().map(|t| FooterContent::new(t, resources)).transpose()?;
                let right = right.clone();
                let style = TextStyle::colored(colors.resolve(palette)?);
                let height = height.unwrap_or(DEFAULT_FOOTER_HEIGHT);
                Ok(Self::Template { left, center, right, style, height })
            }
            raw::FooterStyle::ProgressBar { character, colors } => {
                let character = character.unwrap_or(DEFAULT_PROGRESS_BAR_CHAR);
                let style = TextStyle::colored(colors.resolve(palette)?);
                Ok(Self::ProgressBar { character, style })
            }
            raw::FooterStyle::Empty => Ok(Self::Empty),
        }
    }

    pub(crate) fn height(&self) -> u16 {
        match self {
            Self::Template { height, .. } => *height,
            _ => DEFAULT_FOOTER_HEIGHT,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) enum FooterContent {
    Template(FooterTemplate),
    Image(Image),
}

impl FooterContent {
    fn new(raw: &raw::FooterContent, resources: &Resources) -> Result<Self, ProcessingThemeError> {
        match raw {
            raw::FooterContent::Template(template) => Ok(Self::Template(template.clone())),
            raw::FooterContent::Image { path } => {
                let image = resources.theme_image(path).map_err(ProcessingThemeError::FooterImage)?;
                Ok(Self::Image(image))
            }
        }
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct CodeBlockStyle {
    pub(crate) alignment: Alignment,
    pub(crate) padding: PaddingRect,
    pub(crate) theme_name: String,
    pub(crate) background: bool,
}

impl CodeBlockStyle {
    fn new(raw: &raw::CodeBlockStyle) -> Self {
        let raw::CodeBlockStyle { alignment, padding, theme_name, background } = raw;
        let padding = PaddingRect {
            horizontal: padding.horizontal.unwrap_or_default(),
            vertical: padding.vertical.unwrap_or_default(),
        };
        Self {
            alignment: alignment.clone().unwrap_or_default().into(),
            padding,
            theme_name: theme_name.as_deref().unwrap_or(DEFAULT_CODE_HIGHLIGHT_THEME).to_string(),
            background: background.unwrap_or(true),
        }
    }
}

/// Vertical/horizontal padding.
#[derive(Clone, Debug, Default)]
pub(crate) struct PaddingRect {
    /// The number of columns to use as horizontal padding.
    pub(crate) horizontal: u8,

    /// The number of rows to use as vertical padding.
    pub(crate) vertical: u8,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct ExecutionOutputBlockStyle {
    pub(crate) style: TextStyle,
    pub(crate) status: ExecutionStatusBlockStyle,
}

impl ExecutionOutputBlockStyle {
    fn new(raw: &raw::ExecutionOutputBlockStyle, palette: &ColorPalette) -> Result<Self, ProcessingThemeError> {
        let raw::ExecutionOutputBlockStyle { colors, status } = raw;
        let colors = colors.resolve(palette)?;
        let style = TextStyle::colored(colors);
        Ok(Self { style, status: ExecutionStatusBlockStyle::new(status, palette)? })
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct ExecutionStatusBlockStyle {
    pub(crate) running_style: TextStyle,
    pub(crate) success_style: TextStyle,
    pub(crate) failure_style: TextStyle,
    pub(crate) not_started_style: TextStyle,
}

impl ExecutionStatusBlockStyle {
    fn new(raw: &raw::ExecutionStatusBlockStyle, palette: &ColorPalette) -> Result<Self, ProcessingThemeError> {
        let raw::ExecutionStatusBlockStyle { running, success, failure, not_started } = raw;
        let running_style = TextStyle::colored(running.resolve(palette)?);
        let success_style = TextStyle::colored(success.resolve(palette)?);
        let failure_style = TextStyle::colored(failure.resolve(palette)?);
        let not_started_style = TextStyle::colored(not_started.resolve(palette)?);
        Ok(Self { running_style, success_style, failure_style, not_started_style })
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct InlineCodeStyle {
    pub(crate) style: TextStyle,
}

impl InlineCodeStyle {
    fn new(raw: &raw::InlineCodeStyle, palette: &ColorPalette) -> Result<Self, ProcessingThemeError> {
        let raw::InlineCodeStyle { colors } = raw;
        let style = TextStyle::colored(colors.resolve(palette)?);
        Ok(Self { style })
    }
}

#[derive(Clone, Debug)]
pub(crate) enum ElementType {
    SlideTitle,
    Heading1,
    Heading2,
    Heading3,
    Heading4,
    Heading5,
    Heading6,
    Paragraph,
    List,
    PresentationTitle,
    PresentationSubTitle,
    PresentationEvent,
    PresentationLocation,
    PresentationDate,
    PresentationAuthor,
    Table,
}

#[derive(Clone, Debug)]
pub(crate) struct TypstStyle {
    pub(crate) horizontal_margin: u16,
    pub(crate) vertical_margin: u16,
    pub(crate) style: TextStyle,
}

impl TypstStyle {
    fn new(raw: &raw::TypstStyle, palette: &ColorPalette) -> Result<Self, ProcessingThemeError> {
        let raw::TypstStyle { horizontal_margin, vertical_margin, colors } = raw;
        let horizontal_margin = horizontal_margin.unwrap_or(DEFAULT_TYPST_HORIZONTAL_MARGIN);
        let vertical_margin = vertical_margin.unwrap_or(DEFAULT_TYPST_VERTICAL_MARGIN);
        let style = TextStyle::colored(colors.resolve(palette)?);
        Ok(Self { horizontal_margin, vertical_margin, style })
    }
}

#[derive(Clone, Debug)]
pub(crate) struct MermaidStyle {
    pub(crate) theme: String,
    pub(crate) background: String,
}

impl MermaidStyle {
    fn new(raw: &raw::MermaidStyle) -> Self {
        let raw::MermaidStyle { theme, background } = raw;
        let theme = theme.as_deref().unwrap_or(DEFAULT_MERMAID_THEME).to_string();
        let background = background.as_deref().unwrap_or(DEFAULT_MERMAID_BACKGROUND).to_string();
        Self { theme, background }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ModalStyle {
    pub(crate) style: TextStyle,
    pub(crate) selection_style: TextStyle,
}

impl ModalStyle {
    fn new(
        raw: &raw::ModalStyle,
        default_style: &DefaultStyle,
        palette: &ColorPalette,
    ) -> Result<Self, ProcessingThemeError> {
        let raw::ModalStyle { colors, selection_colors } = raw;
        let mut style = default_style.style;
        style.merge(&TextStyle::colored(colors.resolve(palette)?));

        let mut selection_style = style.bold();
        selection_style.merge(&TextStyle::colored(selection_colors.resolve(palette)?));
        Ok(Self { style, selection_style })
    }
}

/// The color palette.
#[derive(Clone, Debug, Default)]
pub(crate) struct ColorPalette {
    pub(crate) colors: BTreeMap<String, Color>,
    pub(crate) classes: BTreeMap<String, Colors>,
}

impl TryFrom<&raw::ColorPalette> for ColorPalette {
    type Error = ProcessingThemeError;

    fn try_from(palette: &raw::ColorPalette) -> Result<Self, Self::Error> {
        let mut colors = BTreeMap::new();
        let mut classes = BTreeMap::new();

        for (name, color) in &palette.colors {
            let raw::RawColor::Color(color) = color else {
                return Err(ProcessingThemeError::PaletteColorInPalette);
            };
            colors.insert(name.clone(), *color);
        }

        let resolve_local = |color: &RawColor| match color {
            raw::RawColor::Color(c) => Ok(*c),
            raw::RawColor::Palette(name) => colors
                .get(name)
                .copied()
                .ok_or_else(|| ProcessingThemeError::Palette(UndefinedPaletteColorError(name.clone()))),
            _ => Err(ProcessingThemeError::PaletteColorInPalette),
        };
        for (name, colors) in &palette.classes {
            let foreground = colors.foreground.as_ref().map(resolve_local).transpose()?;
            let background = colors.background.as_ref().map(resolve_local).transpose()?;
            classes.insert(name.clone(), Colors { foreground, background });
        }
        Ok(Self { colors, classes })
    }
}
