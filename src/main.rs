use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

const CRLF: &str = "\r\n";
const HTTP_STATUS_OK: &str = "HTTP/1.1 200 OK";
const HTTP_STATUS_CREATED: &str = "HTTP/1.1 201 Created";
const HTTP_STATUS_NOT_FOUND: &str = "HTTP/1.1 404 Not Found";
const CONTENT_TYPE_HEADER: &str = "Content-Type";
const CONTENT_LENGTH_HEADER: &str = "Content-Length";
const CONTENT_ENCODING_HEADER: &str = "Content-Encoding";
const CONTENT_TYPE_TEXT_PLAIN: &str = "text/plain";
const CONTENT_TYPE_OCTET_STREAM: &str = "application/octet-stream";
const CONTENT_ENCODING_GZIP: &str = "gzip";
const LOG_PREFIX: &str = "[SERVER] ";
const DEFAULT_DIRECTORY: &str = ".";
const SERVER_ADDRESS: &str = "localhost:4221";

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let file_directory = if args.len() > 1 {
        &args[1]
    } else {
        DEFAULT_DIRECTORY
    };

    println!("{}Starting server...", LOG_PREFIX);

    let listener = TcpListener::bind(SERVER_ADDRESS).expect("Failed to bind to port 4221");

    println!("{}Listening on {}", LOG_PREFIX, SERVER_ADDRESS);

    handle_graceful_shutdown(
        listener.try_clone().expect("Failed to clone listener"),
        listener,
    );

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                thread::spawn(move || {
                    handle_connection(stream, file_directory);
                });
            }
            Err(e) => {
                eprintln!("{}Error accepting connection: {}", LOG_PREFIX, e);
            }
        }
    }
}

fn handle_graceful_shutdown(listener1: TcpListener, listener2: TcpListener) {
    let (shutdown_tx, shutdown_rx) = mpsc::channel::<()>();
    let thread_listener1 = listener1
        .try_clone()
        .expect("Failed to clone listener for shutdown");

    thread::spawn(move || {
        if let Ok(_) = shutdown_rx.recv() {
            println!("{}Shutting down server...", LOG_PREFIX);
            drop(thread_listener1);
            drop(listener2);
        }
    });

    ctrlc::set_handler(move || {
        if let Err(_) = shutdown_tx.send(()) {
            eprintln!("{}Error sending shutdown signal", LOG_PREFIX);
        }
    })
    .expect("Error setting Ctrl-C handler");
}

fn handle_connection(mut stream: TcpStream, file_directory: &str) {
    let mut reader = BufReader::new(&stream);
    let mut request_line = String::new();

    if reader.read_line(&mut request_line).is_err() {
        eprintln!("{}Error reading request", LOG_PREFIX);
        return;
    }

    let request_parts: Vec<&str> = request_line.split_whitespace().collect();
    if request_parts.len() < 3 {
        eprintln!("{}Invalid request", LOG_PREFIX);
        return;
    }

    let method = request_parts[0];
    let path = request_parts[1];
    let _http_version = request_parts[2];

    println!("{}Request: {} {}", LOG_PREFIX, method, path);

    match method {
        "GET" => handle_get_request(stream, path, file_directory),
        "POST" => handle_post_request(&mut stream, path, file_directory, &mut reader),
        _ => send_response(&mut stream, HTTP_STATUS_NOT_FOUND, &HashMap::new(), ""),
    }
    .unwrap_or_else(|err| eprintln!("{}Error handling request: {}", LOG_PREFIX, err));
}

fn handle_get_request(mut stream: TcpStream, path: &str, file_directory: &str) -> io::Result<()> {
    match path {
        "/" => send_response(&mut stream, HTTP_STATUS_OK, &HashMap::new(), ""),
        _ if path.starts_with("/echo/") => {
            let echo_string = &path[6..];
            handle_echo_request(&mut stream, echo_string)
        }
        "/user-agent" => handle_user_agent_request(&mut stream),
        _ if path.starts_with("/files/") => {
            let filename = &path[7..];
            handle_file_request(&mut stream, file_directory, filename)
        }
        _ => send_response(&mut stream, HTTP_STATUS_NOT_FOUND, &HashMap::new(), ""),
    }
}

fn handle_post_request(
    stream: &mut TcpStream,
    path: &str,
    file_directory: &str,
    reader: &mut BufReader<&TcpStream>,
) -> io::Result<()> {
    let content_length = headers
        .get(CONTENT_LENGTH_HEADER)
        .unwrap_or(&"0".to_string())
        .parse::<usize>()
        .unwrap_or(0);
    let mut body = vec![0; content_length];
    reader.take(content_length as u64).read_exact(&mut body)?;

    let file_path = Path::new(file_directory).join(path.trim_start_matches("/"));
    let mut file = File::create(&file_path)?;

    file.write_all(&body)?;

    send_response(stream, HTTP_STATUS_CREATED, &HashMap::new(), "")
}

fn handle_echo_request(stream: &mut TcpStream, echo_string: &str) -> io::Result<()> {
    let content_type = CONTENT_TYPE_TEXT_PLAIN.to_string();
    let content_length = echo_string.len();
    let headers = create_headers(&content_type, content_length)?;

    send_response(stream, HTTP_STATUS_OK, &headers, echo_string)
}

fn handle_user_agent_request(stream: &mut TcpStream) -> io::Result<()> {
    let user_agent = format!("{}", stream.peer_addr()?.ip());
    let content_type = CONTENT_TYPE_TEXT_PLAIN.to_string();
    let content_length = user_agent.len();
    let headers = create_headers(&content_type, content_length)?;

    send_response(stream, HTTP_STATUS_OK, &headers, &user_agent)
}

fn handle_file_request(
    stream: &mut TcpStream,
    file_directory: &str,
    filename: &str,
) -> io::Result<()> {
    let file_path = Path::new(file_directory).join(filename);
    match File::open(&file_path) {
        Ok(mut file) => {
            let mut file_data = Vec::new();
            file.read_to_end(&mut file_data)?;

            let content_type = CONTENT_TYPE_OCTET_STREAM.to_string();
            let content_length = file_data.len();
            let headers = create_headers(&content_type, content_length)?;

            send_response(
                stream,
                HTTP_STATUS_OK,
                &headers,
                &String::from_utf8_lossy(&file_data),
            )
        }
        Err(_) => send_response(stream, HTTP_STATUS_NOT_FOUND, &HashMap::new(), ""),
    }
}

fn create_headers(
    content_type: &str,
    content_length: usize,
) -> io::Result<HashMap<String, String>> {
    let mut headers = HashMap::new();
    headers.insert(CONTENT_TYPE_HEADER.to_string(), content_type.to_string());
    headers.insert(
        CONTENT_LENGTH_HEADER.to_string(),
        content_length.to_string(),
    );
    Ok(headers)
}

fn send_response(
    stream: &mut TcpStream,
    status: &str,
    headers: &HashMap<String, String>,
    body: &str,
) -> io::Result<()> {
    let mut response = format!("{}{}", status, CRLF);
    for (key, value) in headers.iter() {
        response.push_str(&format!("{}: {}{}", key, value, CRLF));
    }
    response.push_str(CRLF);
    response.push_str(body);

    stream.write_all(response.as_bytes())?;
    stream.flush()
}
