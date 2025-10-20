use serde::{Deserialize, Serialize};
use socket2::{Domain, Protocol, Socket, Type};
use std::{
    io,
    net::{SocketAddr, UdpSocket},
    path::PathBuf,
};

pub struct SpeakerNotesEventPublisher {
    socket: UdpSocket,
    presentation_path: PathBuf,
}

impl SpeakerNotesEventPublisher {
    pub fn new(address: SocketAddr, presentation_path: PathBuf) -> io::Result<Self> {
        let socket = UdpSocket::bind("127.0.0.1:0")?;
        socket.set_broadcast(true)?;
        socket.connect(address)?;
        Ok(Self { socket, presentation_path })
    }

    pub(crate) fn send(&self, event: SpeakerNotesEvent) -> io::Result<()> {
        // Wrap this event in an envelope that contains the presentation path so listeners can
        // ignore unrelated events.
        let envelope = SpeakerNotesEventEnvelope { event, presentation_path: self.presentation_path.clone() };
        let data = serde_json::to_string(&envelope).expect("serialization failed");
        match self.socket.send(data.as_bytes()) {
            Ok(_) => Ok(()),
            Err(e) if e.kind() == io::ErrorKind::ConnectionRefused => Ok(()),
            Err(e) => Err(e),
        }
    }
}

pub struct SpeakerNotesEventListener {
    socket: UdpSocket,
    presentation_path: PathBuf,
}

impl SpeakerNotesEventListener {
    pub fn new(address: SocketAddr, presentation_path: PathBuf) -> io::Result<Self> {
        let s = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
        // Use SO_REUSEADDR so we can have multiple listeners on the same port.
        #[cfg(not(target_os = "macos"))]
        s.set_reuse_address(true)?;
        // Don't block so we can listen to the keyboard and this socket at the same time.
        s.set_nonblocking(true)?;
        s.bind(&address.into())?;
        Ok(Self { socket: s.into(), presentation_path })
    }

    pub(crate) fn try_recv(&self) -> io::Result<Option<SpeakerNotesEvent>> {
        let mut buffer = [0; 1024];
        let bytes_read = match self.socket.recv(&mut buffer) {
            Ok(bytes_read) => bytes_read,
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => return Ok(None),
            Err(e) => return Err(e),
        };
        // Ignore garbage. Odds are this is someone else sending garbage rather than presenterm
        // itself.
        let Ok(envelope) = serde_json::from_slice::<SpeakerNotesEventEnvelope>(&buffer[0..bytes_read]) else {
            return Ok(None);
        };
        if envelope.presentation_path == self.presentation_path { Ok(Some(envelope.event)) } else { Ok(None) }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "command")]
pub(crate) enum SpeakerNotesEvent {
    GoToSlide { slide: u32, chunk: u32 },
    Exit,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct SpeakerNotesEventEnvelope {
    presentation_path: PathBuf,
    event: SpeakerNotesEvent,
}

#[cfg(not(target_os = "macos"))]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{default_speaker_notes_listen_address, default_speaker_notes_publish_address};
    use std::{thread::sleep, time::Duration};

    fn make_listener(path: PathBuf) -> SpeakerNotesEventListener {
        SpeakerNotesEventListener::new(default_speaker_notes_listen_address(), path).expect("building listener")
    }

    fn make_publisher(path: PathBuf) -> SpeakerNotesEventPublisher {
        SpeakerNotesEventPublisher::new(default_speaker_notes_publish_address(), path).expect("building publisher")
    }

    #[test]
    fn bind_multiple() {
        let _l1 = make_listener("".into());
        let _l2 = make_listener("".into());
    }

    #[test]
    fn multicast() {
        let path = PathBuf::from("/tmp/test.md");
        let l1 = make_listener(path.clone());
        let l2 = make_listener(path.clone());
        let publisher = make_publisher(path);
        let event = SpeakerNotesEvent::Exit;
        publisher.send(event.clone()).expect("send failed");
        sleep(Duration::from_millis(100));

        assert_eq!(l1.try_recv().expect("recv first failed"), Some(event.clone()));
        assert_eq!(l2.try_recv().expect("recv second failed"), Some(event));
    }
}
