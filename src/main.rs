use std::{
    env, fs,
    io::{self, Read, Write},
    net::{TcpListener, TcpStream},
    thread,
};

const OK_RESPONSE: &str = "HTTP/1.1 200 OK\r\n\r\n";
const NOT_FOUND_RESPONSE: &str = "HTTP/1.1 404 NOT FOUND\r\n\r\n";
const METHOD_NOT_ALLOWED_RESPONSE: &str = "HTTP/1.1 405 Method Not Allowed\r\n\r\n";
const CONTENT_TYPE_PLAIN_TEXT: &str = "Content-Type: text/plain\r\n";
const CONTENT_TYPE_OCTET_STREAM: &str = "Content-Type: application/octet-stream\r\n";

#[derive(Debug)]
struct HttpRequest {
    method: String,
    path: String,
    version: String,
    host: Option<String>,
    user_agent: Option<String>,
}

impl HttpRequest {
    fn new(method: String, path: String, version: String) -> Self {
        HttpRequest {
            method,
            path,
            version,
            host: None,
            user_agent: None,
        }
    }

    fn with_host(mut self, host: String) -> Self {
        self.host = Some(host);
        self
    }

    fn with_user_agent(mut self, user_agent: String) -> Self {
        self.user_agent = Some(user_agent);
        self
    }
}

fn main() -> io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:4221")?;
    println!("Server listening on http://127.0.0.1:4221");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                thread::spawn(|| {
                    if let Err(e) = handle_connection(stream) {
                        eprintln!("Error handling connection: {}", e);
                    }
                });
            }
            Err(e) => {
                eprintln!("Error accepting connection: {}", e);
            }
        }
    }

    Ok(())
}

fn handle_connection(mut stream: TcpStream) -> io::Result<()> {
    let mut buffer = [0; 1024];
    stream.read(&mut buffer)?;
    let req_str = String::from_utf8_lossy(&buffer[..]);
    let http_request = parse_request(&req_str)?;

    let response = match http_request.method.as_str() {
        "GET" => handle_get_request(&http_request),
        _ => METHOD_NOT_ALLOWED_RESPONSE.to_string(),
    };

    stream.write_all(response.as_bytes())?;
    Ok(())
}

fn parse_request(request: &str) -> io::Result<HttpRequest> {
    let mut lines = request.lines();
    let first_line = lines
        .next()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Empty request"))?;

    let mut parts = first_line.split_whitespace();
    let method = parts.next().unwrap_or_default().to_string();
    let path = parts.next().unwrap_or_default().to_string();
    let version = parts.next().unwrap_or_default().to_string();

    let (host, user_agent) = lines.fold((None, None), |acc, line| {
        if line.starts_with("Host: ") {
            (Some(line.replace("Host: ", "").trim().to_string()), acc.1)
        } else if line.starts_with("User-Agent: ") {
            (
                acc.0,
                Some(line.replace("User-Agent: ", "").trim().to_string()),
            )
        } else {
            acc
        }
    });

    Ok(HttpRequest::new(method, path, version)
        .with_host(host.unwrap_or_default())
        .with_user_agent(user_agent.unwrap_or_default()))
}

fn handle_get_request(http_request: &HttpRequest) -> String {
    match http_request.path.as_str() {
        "/" => OK_RESPONSE.to_string(),
        path if path.starts_with("/echo/") => handle_echo_request(path),
        "/user-agent" => handle_user_agent_request(http_request),
        path if path.starts_with("/files/") => handle_file_request(http_request),
        _ => NOT_FOUND_RESPONSE.to_string(),
    }
}

fn handle_echo_request(path: &str) -> String {
    let body = path.replace("/echo/", "");
    format!(
        "{}{}Content-Length: {}\r\n\r\n{}",
        OK_RESPONSE,
        CONTENT_TYPE_PLAIN_TEXT,
        body.len(),
        body
    )
}

fn handle_user_agent_request(http_request: &HttpRequest) -> String {
    if let Some(user_agent) = &http_request.user_agent {
        format!(
            "{}{}Content-Length: {}\r\n\r\n{}",
            OK_RESPONSE,
            CONTENT_TYPE_PLAIN_TEXT,
            user_agent.len(),
            user_agent
        )
    } else {
        NOT_FOUND_RESPONSE.to_string()
    }
}

fn handle_file_request(http_request: &HttpRequest) -> String {
    let file_name = http_request.path.replace("/files/", "");
    let mut dir = match env::args().nth(1) {
        Some(dir) => dir,
        None => {
            eprintln!("Error: --directory argument not provided.");
            return NOT_FOUND_RESPONSE.to_string();
        }
    };

    dir.push_str(&file_name);
    match fs::read(dir) {
        Ok(contents) => {
            format!(
                "{}{}Content-Length: {}\r\n\r\n{}",
                OK_RESPONSE,
                CONTENT_TYPE_OCTET_STREAM,
                contents.len(),
                String::from_utf8_lossy(&contents)
            )
        }
        Err(_) => NOT_FOUND_RESPONSE.to_string(),
    }
}
