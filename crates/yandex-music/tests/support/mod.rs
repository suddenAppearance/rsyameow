//! A tiny blocking HTTP server used by the integration tests.
//!
//! It intentionally has no dependencies: the client crate must remain usable in
//! minimal environments, and the tests only need to inspect requests and return
//! canned responses.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct Request {
    pub method: String,
    pub path: String,
    pub query: Vec<(String, String)>,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

impl Request {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }

    pub fn form(&self) -> Vec<(String, String)> {
        parse_pairs(&self.body)
    }

    pub fn form_value(&self, name: &str) -> Option<String> {
        self.form()
            .into_iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value)
    }

    pub fn query_value(&self, name: &str) -> Option<&str> {
        self.query
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }

    pub fn form_values(&self, name: &str) -> Vec<String> {
        self.form()
            .into_iter()
            .filter(|(key, _)| key == name)
            .map(|(_, value)| value)
            .collect()
    }
}

pub struct MockResponse {
    pub status: u16,
    pub content_type: String,
    pub body: Vec<u8>,
}

impl MockResponse {
    pub fn new(status: u16, content_type: impl Into<String>, body: impl Into<Vec<u8>>) -> Self {
        Self {
            status,
            content_type: content_type.into(),
            body: body.into(),
        }
    }

    pub fn json(status: u16, value: serde_json::Value) -> Self {
        Self::new(
            status,
            "application/json",
            serde_json::to_vec(&value).unwrap(),
        )
    }

    pub fn xml(body: &str) -> Self {
        Self::new(200, "application/xml", body)
    }

    pub fn bytes(body: impl Into<Vec<u8>>) -> Self {
        Self::new(200, "application/octet-stream", body)
    }
}

pub struct TestServer {
    address: SocketAddr,
    shutdown: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
    pub requests: Arc<Mutex<Vec<Request>>>,
}

impl TestServer {
    pub fn start(handler: impl Fn(&Request) -> MockResponse + Send + Sync + 'static) -> Self {
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind test server");
        let address = listener.local_addr().unwrap();
        listener.set_nonblocking(true).unwrap();

        let shutdown = Arc::new(AtomicBool::new(false));
        let requests = Arc::new(Mutex::new(Vec::new()));

        let thread_shutdown = shutdown.clone();
        let thread_requests = requests.clone();
        let handler = Arc::new(handler);
        let handle = thread::spawn(move || {
            while !thread_shutdown.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        if let Some(request) = read_request(&stream) {
                            thread_requests.lock().unwrap().push(request.clone());
                            let response = handler(&request);
                            let _ = write_response(stream, response);
                        }
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(2));
                    }
                    Err(_) => break,
                }
            }
        });

        Self {
            address,
            shutdown,
            handle: Some(handle),
            requests,
        }
    }

    pub fn base_url(&self) -> String {
        format!("http://{}", self.address)
    }

    pub fn requests(&self) -> Vec<Request> {
        self.requests.lock().unwrap().clone()
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect(self.address);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

fn read_request(stream: &TcpStream) -> Option<Request> {
    stream.set_read_timeout(Some(Duration::from_secs(5))).ok()?;
    let mut reader = BufReader::new(stream.try_clone().ok()?);
    let mut request_line = String::new();
    reader.read_line(&mut request_line).ok()?;
    let mut parts = request_line.split_whitespace();
    let method = parts.next()?.to_string();
    let target = parts.next()?.to_string();

    let (path, query) = match target.split_once('?') {
        Some((path, query)) => (path.to_string(), parse_pairs(query)),
        None => (target, Vec::new()),
    };

    let mut headers = Vec::new();
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).ok()?;
        let trimmed = line.trim_end();
        if trimmed.is_empty() {
            break;
        }
        if let Some((name, value)) = trimmed.split_once(':') {
            headers.push((name.trim().to_string(), value.trim().to_string()));
        }
    }

    let content_length = headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
        .and_then(|(_, value)| value.parse::<usize>().ok())
        .unwrap_or(0);

    let mut body = vec![0; content_length];
    if content_length > 0 {
        reader.read_exact(&mut body).ok()?;
    }

    Some(Request {
        method,
        path,
        query,
        headers,
        body: String::from_utf8_lossy(&body).to_string(),
    })
}

fn write_response(mut stream: TcpStream, response: MockResponse) -> std::io::Result<()> {
    let reason = match response.status {
        200 => "OK",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        500 => "Internal Server Error",
        _ => "Unknown",
    };
    write!(
        stream,
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        response.status,
        reason,
        response.content_type,
        response.body.len()
    )?;
    stream.write_all(&response.body)?;
    stream.flush()
}

fn parse_pairs(input: &str) -> Vec<(String, String)> {
    input
        .split('&')
        .filter(|pair| !pair.is_empty())
        .map(|pair| match pair.split_once('=') {
            Some((key, value)) => (decode(key), decode(value)),
            None => (decode(pair), String::new()),
        })
        .collect()
}

fn decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut output = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'+' => {
                output.push(b' ');
                index += 1;
            }
            b'%' if index + 2 < bytes.len() => {
                let hex = &value[index + 1..index + 3];
                if let Ok(byte) = u8::from_str_radix(hex, 16) {
                    output.push(byte);
                    index += 3;
                } else {
                    output.push(bytes[index]);
                    index += 1;
                }
            }
            byte => {
                output.push(byte);
                index += 1;
            }
        }
    }
    String::from_utf8_lossy(&output).to_string()
}
