use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use crossterm::{style, QueueableCommand};
use std::{io, rc::Rc};

#[derive(Clone)]
pub struct Image {
    contents: Rc<Vec<u8>>,
}

impl Image {
    pub fn new(contents: Vec<u8>) -> Self {
        let contents = Rc::new(contents);
        Self { contents }
    }
}

pub trait DrawMedia {
    fn draw_image(&self, image: &Image, writer: &mut dyn io::Write) -> io::Result<()>;
}

pub struct KittyTerminal;

impl DrawMedia for KittyTerminal {
    fn draw_image(&self, image: &Image, writer: &mut dyn io::Write) -> io::Result<()> {
        let contents = BASE64.encode(image.contents.as_ref());
        let count = contents.as_bytes().chunks(4096).count();
        for (index, chunk) in contents.as_bytes().chunks(4096).enumerate() {
            let more = (index < count - 1) as u8;
            let mut data = Vec::<u8>::new();
            data.extend(b"\x1b_G");
            data.extend(format!("m={more},a=T,f=100").as_bytes());
            data.push(b';');
            data.extend(chunk);
            data.extend(b"\x1b\\");

            let data = String::from_utf8(data).expect("not utf8");
            writer.queue(style::Print(data))?;
        }
        Ok(())
    }
}
