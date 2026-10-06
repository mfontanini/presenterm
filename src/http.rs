//! A minimal HTTP server used to expose a presentation online as a single self contained HTML
//! document.
//!
//! The document produced by the HTML exporter embeds every image as an inline base64 data URI and
//! all of its styles inline, so serving it only requires handing out that single string. This
//! keeps the server tiny: no routing of assets, no templating and no extra dependencies beyond the
//! standard library.

use std::{
    borrow::Cow,
    io::{self, BufRead, BufReader, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    sync::Arc,
    thread,
};

/// Serves a presentation rendered as a self contained HTML document over HTTP.
pub struct PresentationServer {
    html: Arc<String>,
    listener: TcpListener,
}

impl PresentationServer {
    /// Bind the server to the given address.
    ///
    /// Binding eagerly lets callers learn the actual address (useful when binding to port `0`)
    /// before entering the blocking [`PresentationServer::run`] loop.
    pub fn bind(html: String, address: SocketAddr) -> io::Result<Self> {
        let listener = TcpListener::bind(address)?;
        Ok(Self { html: Arc::new(html), listener })
    }

    /// The address the server is listening on.
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.listener.local_addr()
    }

    /// Accept and serve connections forever.
    ///
    /// Each connection is handled on its own thread so multiple viewers can load the presentation
    /// concurrently. A single misbehaving client never brings the server down: per connection
    /// errors are swallowed.
    pub fn run(self) -> io::Result<()> {
        for stream in self.listener.incoming() {
            let stream = match stream {
                Ok(stream) => stream,
                // A failure to accept one connection shouldn't tear the whole server down.
                Err(_) => continue,
            };
            let html = self.html.clone();
            thread::spawn(move || {
                let _ = Self::handle_connection(stream, &html);
            });
        }
        Ok(())
    }

    fn handle_connection(mut stream: TcpStream, html: &str) -> io::Result<()> {
        // Read the request from a clone so we can still write the response to the original stream.
        let mut reader = BufReader::new(stream.try_clone()?);
        let request = HttpRequest::parse(&mut reader)?;
        let response = Self::route(&request, html);
        response.write(&mut stream, request.method.wants_body())
    }

    fn route<'a>(request: &HttpRequest, html: &'a str) -> HttpResponse<'a> {
        match (&request.method, request.path.as_str()) {
            (Method::Get | Method::Head, "/" | "/index.html") => {
                HttpResponse::ok("text/html; charset=utf-8", Cow::Borrowed(html.as_bytes()))
            }
            (Method::Get | Method::Head, "/healthz") => {
                HttpResponse::ok("text/plain; charset=utf-8", Cow::Borrowed(b"ok"))
            }
            _ => HttpResponse::not_found(),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Method {
    Get,
    Head,
    Other,
}

impl Method {
    fn parse(raw: &str) -> Self {
        match raw {
            "GET" => Self::Get,
            "HEAD" => Self::Head,
            _ => Self::Other,
        }
    }

    /// Whether a response to a request with this method should include a body.
    fn wants_body(&self) -> bool {
        !matches!(self, Self::Head)
    }
}

#[derive(Debug, PartialEq, Eq)]
struct HttpRequest {
    method: Method,
    path: String,
}

impl HttpRequest {
    fn parse<R: BufRead>(reader: &mut R) -> io::Result<Self> {
        let mut request_line = String::new();
        if reader.read_line(&mut request_line)? == 0 {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "empty request"));
        }
        let mut parts = request_line.split_whitespace();
        let method = Method::parse(parts.next().unwrap_or_default());
        // Strip the query string and fragment, keeping only the path.
        let target = parts.next().unwrap_or("/");
        let path = target.split(['?', '#']).next().unwrap_or("/").to_string();

        // Drain the remaining request headers so clients that check for a clean read don't error
        // out. We don't make use of any of them.
        let mut line = String::new();
        loop {
            line.clear();
            let read = reader.read_line(&mut line)?;
            if read == 0 || line == "\r\n" || line == "\n" {
                break;
            }
        }
        Ok(Self { method, path })
    }
}

struct HttpResponse<'a> {
    status: &'static str,
    content_type: &'static str,
    body: Cow<'a, [u8]>,
}

impl<'a> HttpResponse<'a> {
    fn ok(content_type: &'static str, body: Cow<'a, [u8]>) -> Self {
        Self { status: "200 OK", content_type, body }
    }

    fn not_found() -> Self {
        Self { status: "404 Not Found", content_type: "text/plain; charset=utf-8", body: Cow::Borrowed(b"not found") }
    }

    fn write<W: Write>(&self, writer: &mut W, include_body: bool) -> io::Result<()> {
        let headers = format!(
            "HTTP/1.1 {}\r\n\
             Content-Type: {}\r\n\
             Content-Length: {}\r\n\
             Cache-Control: no-store\r\n\
             Connection: close\r\n\
             \r\n",
            self.status,
            self.content_type,
            self.body.len(),
        );
        writer.write_all(headers.as_bytes())?;
        if include_body {
            writer.write_all(&self.body)?;
        }
        writer.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    #[test]
    fn parse_request_strips_query_string() {
        let raw = "GET /index.html?foo=bar HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let mut reader = BufReader::new(raw.as_bytes());
        let request = HttpRequest::parse(&mut reader).expect("parse failed");
        assert_eq!(request.method, Method::Get);
        assert_eq!(request.path, "/index.html");
    }

    #[test]
    fn parse_head_request() {
        let raw = "HEAD / HTTP/1.1\r\n\r\n";
        let mut reader = BufReader::new(raw.as_bytes());
        let request = HttpRequest::parse(&mut reader).expect("parse failed");
        assert_eq!(request.method, Method::Head);
        assert!(!request.method.wants_body());
    }

    #[test]
    fn route_serves_html_on_root() {
        let html = "<html>hello</html>";
        let request = HttpRequest { method: Method::Get, path: "/".into() };
        let response = PresentationServer::route(&request, html);
        assert_eq!(response.status, "200 OK");
        assert_eq!(response.content_type, "text/html; charset=utf-8");
        assert_eq!(response.body.as_ref(), html.as_bytes());
    }

    #[test]
    fn route_unknown_path_is_not_found() {
        let request = HttpRequest { method: Method::Get, path: "/missing".into() };
        let response = PresentationServer::route(&request, "<html></html>");
        assert_eq!(response.status, "404 Not Found");
    }

    #[test]
    fn serves_html_over_the_wire() {
        let html = "<html><body>slides</body></html>".to_string();
        let server = PresentationServer::bind(html.clone(), "127.0.0.1:0".parse().unwrap()).expect("failed to bind");
        let address = server.local_addr().expect("no local addr");
        thread::spawn(move || {
            let _ = server.run();
        });

        let mut stream = TcpStream::connect(address).expect("failed to connect");
        stream.write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n").expect("failed to write");
        let mut response = String::new();
        stream.read_to_string(&mut response).expect("failed to read");

        assert!(response.contains("200 OK"), "unexpected response: {response}");
        assert!(response.contains("Content-Type: text/html"), "unexpected response: {response}");
        assert!(response.ends_with(&html), "unexpected response: {response}");
    }

    #[test]
    fn head_request_omits_body() {
        let html = "<html><body>slides</body></html>".to_string();
        let server = PresentationServer::bind(html.clone(), "127.0.0.1:0".parse().unwrap()).expect("failed to bind");
        let address = server.local_addr().expect("no local addr");
        thread::spawn(move || {
            let _ = server.run();
        });

        let mut stream = TcpStream::connect(address).expect("failed to connect");
        stream.write_all(b"HEAD / HTTP/1.1\r\nHost: localhost\r\n\r\n").expect("failed to write");
        let mut response = String::new();
        stream.read_to_string(&mut response).expect("failed to read");

        assert!(response.contains("200 OK"), "unexpected response: {response}");
        assert!(!response.contains("slides"), "body should be omitted: {response}");
    }
}
