use crate::{
    markdown::{
        elements::{Line, Text},
        text_style::{Colors, TextStyle},
    },
    presentation::builder::{BuildResult, PresentationBuilder},
    render::operation::{BlockLine, RenderOperation},
    theme::{Alignment, raw::RawColor},
};
use comrak::nodes::AlertType;
use unicode_width::UnicodeWidthStr;

impl PresentationBuilder<'_, '_> {
    pub(crate) fn push_block_quote(&mut self, lines: Vec<Line<RawColor>>) -> BuildResult {
        let prefix = self.theme.block_quote.prefix.clone();
        let prefix_style = self.theme.block_quote.prefix_style;
        self.push_quoted_text(
            lines,
            prefix,
            self.theme.block_quote.base_style.colors,
            prefix_style,
            self.theme.block_quote.alignment,
        )
    }

    pub(crate) fn push_alert(
        &mut self,
        alert_type: AlertType,
        title: Option<String>,
        mut lines: Vec<Line<RawColor>>,
    ) -> BuildResult {
        let style = match alert_type {
            AlertType::Note => &self.theme.alert.styles.note,
            AlertType::Tip => &self.theme.alert.styles.tip,
            AlertType::Important => &self.theme.alert.styles.important,
            AlertType::Warning => &self.theme.alert.styles.warning,
            AlertType::Caution => &self.theme.alert.styles.caution,
        };

        let title = format!("{} {}", style.icon, title.as_deref().unwrap_or(style.title.as_ref()));
        lines.insert(0, Line::from(Text::from("")));
        lines.insert(0, Line::from(Text::new(title, style.style.into_raw())));

        let prefix = self.theme.alert.prefix.clone();
        self.push_quoted_text(
            lines,
            prefix,
            self.theme.alert.base_style.colors,
            style.style,
            self.theme.alert.alignment,
        )
    }

    fn push_quoted_text(
        &mut self,
        lines: Vec<Line<RawColor>>,
        prefix: String,
        base_colors: Colors,
        prefix_style: TextStyle,
        alignment: Alignment,
    ) -> BuildResult {
        let block_length = lines.iter().map(|line| line.width() + prefix.width()).max().unwrap_or(0) as u16;
        let font_size = self.slide_font_size();
        let prefix = Text::new(prefix, prefix_style.size(font_size));

        for line in lines {
            let mut line = line.resolve(&self.theme.palette)?;
            // Apply our colors to each chunk in this line.
            for text in &mut line.0 {
                if text.style.colors.background.is_none() && text.style.colors.foreground.is_none() {
                    text.style.colors = base_colors;
                    if text.style.is_code() {
                        text.style.colors = self.theme.inline_code.style.colors;
                    }
                }
                text.style = text.style.size(font_size);
            }
            self.chunk_operations.push(RenderOperation::RenderBlockLine(BlockLine {
                prefix: prefix.clone().into(),
                right_padding_length: 0,
                repeat_prefix_on_wrap: true,
                text: line.into(),
                block_length,
                alignment,
                block_color: base_colors.background,
            }));
            self.push_line_break();
        }
        self.set_colors(self.theme.default_style.style.colors);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{markdown::text_style::Color, presentation::builder::utils::Test, theme::raw};
    use rstest::rstest;

    #[rstest]
    #[case::left_no_margin(raw::Alignment::Left{ margin: raw::Margin::Fixed(0) },"▍ hi   ", "XXXXXXX", )]
    #[case::left_one_margin(raw::Alignment::Left{ margin: raw::Margin::Fixed(1) }, " ▍ hi  ", " XXXXX ")]
    #[case::center(raw::Alignment::Center{ minimum_margin: raw::Margin::Fixed(0), minimum_size: 0 }, " ▍ hi  ", " XXXX  ")]
    #[test]
    fn quote(#[case] alignment: raw::Alignment, #[case] line: &str, #[case] style: &str) {
        let input = "
> hi
> hi
";
        let color = Color::new(1, 1, 1);
        let theme = raw::PresentationTheme {
            block_quote: raw::BlockQuoteStyle {
                colors: raw::BlockQuoteColors {
                    base: raw::RawColors { foreground: None, background: Some(raw::RawColor::Color(color)) },
                    prefix: None,
                },
                alignment: Some(alignment),
                ..Default::default()
            },
            ..Default::default()
        };
        let (lines, styles) =
            Test::new(input).theme(theme).render().map_background(color, 'X').rows(4).columns(7).into_parts();
        let expected_lines = &["       ", line, line, "       "];
        let expected_styles = &["       ", style, style, "       "];
        assert_eq!(lines, expected_lines);
        assert_eq!(styles, expected_styles);
    }

    #[test]
    fn alert() {
        let input = "
> [!note]
> hi
";
        let theme = raw::PresentationTheme {
            alert: raw::AlertStyle {
                styles: raw::AlertTypeStyles {
                    note: raw::AlertTypeStyle { icon: Some("!".to_string()), ..Default::default() },
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        };
        let lines = Test::new(input).theme(theme).render().rows(5).columns(9).into_lines();
        let expected = &["         ", "▍ ! Note ", "▍        ", "▍ hi     ", "         "];
        assert_eq!(lines, expected);
    }
}
