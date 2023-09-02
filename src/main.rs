use clap::Parser;
use comrak::{Arena, ComrakOptions};
use presenterm::{
    draw::Drawer,
    highlighting::CodeHighlighter,
    input::{Command, Input},
    parse::SlideParser,
    presentation::Presentation,
    resource::Resources,
    theme::{Alignment, ElementStyle, ElementType, SlideTheme},
};
use std::{fs, io, path::PathBuf};

#[derive(Parser)]
struct Cli {
    path: PathBuf,
}

struct SlideShow {
    resources: Resources,
    highlighter: CodeHighlighter,
    theme: SlideTheme,
}

impl SlideShow {
    fn present(mut self, mut presentation: Presentation) -> io::Result<()> {
        let mut drawer = Drawer::new(io::stdout())?;
        loop {
            let slide = presentation.current_slide();
            drawer.draw_slide(&mut self.resources, &self.highlighter, &self.theme, slide)?;

            loop {
                let Some(command) = Input::next_command()? else { continue; };
                match command {
                    Command::Redraw => (),
                    Command::NextSlide => presentation.move_next_slide(),
                    Command::PreviousSlide => presentation.move_previous_slide(),
                    Command::Exit => return Ok(()),
                };
                break;
            }
        }
    }
}

fn main() {
    let cli = Cli::parse();
    let arena = Arena::new();
    let options = ComrakOptions::default();
    let parser = SlideParser::new(&arena, options);

    let content = fs::read_to_string(cli.path).expect("reading failed");
    let slides = parser.parse(&content).expect("parse failed");
    let presentation = Presentation::new(slides);

    let resources = Resources::default();
    let highlighter = CodeHighlighter::new("Solarized (light)").expect("creating highlighter failed");
    let theme = SlideTheme {
        default_style: ElementStyle { alignment: Alignment::Left { margin: 5 } },
        element_style: [(ElementType::SlideTitle, ElementStyle { alignment: Alignment::Center { minimum_margin: 5 } })]
            .into(),
    };

    let slideshow = SlideShow { resources, highlighter, theme };
    if let Err(e) = slideshow.present(presentation) {
        eprintln!("Error running slideshow: {e}");
    };
}
