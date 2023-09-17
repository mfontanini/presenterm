use clap::{error::ErrorKind, CommandFactory, Parser};
use comrak::Arena;
use presenterm::{
    input::{
        source::{Command, CommandSource},
        user::UserCommand,
    },
    markdown::{parse::MarkdownParser, process::MarkdownProcessor},
    presentation::Presentation,
    render::{
        draw::{DrawResult, Drawer},
        highlighting::CodeHighlighter,
    },
    resource::Resources,
    theme::PresentationTheme,
};
use std::{
    fs, io,
    path::{Path, PathBuf},
};

#[derive(Parser)]
struct Cli {
    path: PathBuf,

    #[clap(default_value = "dark")]
    theme: String,
}

struct SlideShow<'a> {
    theme: PresentationTheme,
    commands: CommandSource,
    parser: MarkdownParser<'a>,
    resources: Resources,
    highlighter: CodeHighlighter,
}

impl<'a> SlideShow<'a> {
    fn present(mut self, path: &Path) -> DrawResult {
        let mut presentation = self.load_presentation(path);
        let mut drawer = Drawer::new(io::stdout())?;
        loop {
            drawer.render_slide(&self.theme, &presentation)?;

            loop {
                let command = match self.commands.next_command()? {
                    Command::User(command) => command,
                    Command::ReloadPresentation => {
                        let current = presentation.current_slide_index();
                        presentation = self.load_presentation(path);
                        presentation.jump_slide(current);
                        break;
                    }
                    // TODO graceful pls
                    Command::Abort { error } => panic!("need to abort: {error}"),
                };
                let needs_redraw = match command {
                    UserCommand::Redraw => true,
                    UserCommand::JumpNextSlide => presentation.jump_next_slide(),
                    UserCommand::JumpPreviousSlide => presentation.jump_previous_slide(),
                    UserCommand::JumpFirstSlide => presentation.jump_first_slide(),
                    UserCommand::JumpLastSlide => presentation.jump_last_slide(),
                    UserCommand::JumpSlide(number) => presentation.jump_slide(number.saturating_sub(1) as usize),
                    UserCommand::Exit => return Ok(()),
                };
                if needs_redraw {
                    break;
                }
            }
        }
    }

    fn load_presentation(&mut self, path: &Path) -> Presentation {
        // TODO: handle errors!
        let content = fs::read_to_string(path).expect("reading failed");
        let elements = self.parser.parse(&content).expect("parse failed");
        let slides = MarkdownProcessor::new(&self.highlighter, &self.theme, &mut self.resources)
            .transform(elements)
            .expect("processing failed");
        Presentation::new(slides)
    }
}

fn main() {
    let cli = Cli::parse();
    let Some(theme) = PresentationTheme::from_name(&cli.theme) else {
        let mut cmd = Cli::command();
        cmd.error(ErrorKind::InvalidValue, "invalid theme name").exit();
    };

    let arena = Arena::new();
    let parser = MarkdownParser::new(&arena);
    let highlighter = CodeHighlighter::new("base16-ocean.dark").expect("creating highlighter failed");
    let resources = Resources::new(cli.path.parent().expect("no parent"));
    let commands = CommandSource::new(&cli.path);

    let slideshow = SlideShow { theme, commands, parser, resources, highlighter };
    if let Err(e) = slideshow.present(&cli.path) {
        eprintln!("Error running slideshow: {e}");
    };
}
