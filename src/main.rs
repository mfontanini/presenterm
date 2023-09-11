use clap::{error::ErrorKind, CommandFactory, Parser};
use comrak::Arena;
use presenterm::{
    input::{Command, Input},
    parse::SlideParser,
    presentation::Presentation,
    render::{
        draw::{DrawResult, Drawer},
        highlighting::CodeHighlighter,
    },
    resource::Resources,
    theme::SlideTheme,
};
use std::{fs, io, path::PathBuf};

#[derive(Parser)]
struct Cli {
    path: PathBuf,

    #[clap(default_value = "dark")]
    theme: String,
}

struct SlideShow {
    resources: Resources,
    highlighter: CodeHighlighter,
    theme: SlideTheme,
    input: Input,
}

impl SlideShow {
    fn present(mut self, mut presentation: Presentation) -> DrawResult {
        let mut drawer = Drawer::new(io::stdout())?;
        loop {
            let slide = presentation.current_slide();
            drawer.draw_slide(&mut self.resources, &self.highlighter, &self.theme, slide, &presentation)?;

            loop {
                let Some(command) = self.input.next_command()? else {
                    continue;
                };
                let needs_redraw = match command {
                    Command::Redraw => true,
                    Command::JumpNextSlide => presentation.jump_next_slide(),
                    Command::JumpPreviousSlide => presentation.jump_previous_slide(),
                    Command::JumpFirstSlide => presentation.jump_first_slide(),
                    Command::JumpLastSlide => presentation.jump_last_slide(),
                    Command::JumpSlide(number) => presentation.jump_slide(number.saturating_sub(1) as usize),
                    Command::Exit => return Ok(()),
                };
                if needs_redraw {
                    break;
                }
            }
        }
    }
}

fn main() {
    let cli = Cli::parse();
    let Some(theme) = SlideTheme::from_name(&cli.theme) else {
        let mut cmd = Cli::command();
        cmd.error(ErrorKind::InvalidValue, "invalid theme name").exit();
    };

    let arena = Arena::new();
    let parser = SlideParser::new(&arena);

    let content = fs::read_to_string(cli.path).expect("reading failed");
    let slides = parser.parse(&content).expect("parse failed");
    let presentation = Presentation::new(slides);

    let resources = Resources::default();
    let highlighter = CodeHighlighter::new("base16-ocean.dark").expect("creating highlighter failed");
    let input = Input::default();

    let slideshow = SlideShow { resources, highlighter, theme, input };
    if let Err(e) = slideshow.present(presentation) {
        eprintln!("Error running slideshow: {e}");
    };
}
