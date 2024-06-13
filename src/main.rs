use std::{
    collections::HashMap,
    fs::File,
    io::{self, BufRead, BufReader, Read, Write},
    net::{TcpListener, TcpStream},
    path::{Path, PathBuf},
    sync::mpsc,
    thread,
};

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

fn main() -> io::Result<()> {
    let mut file_directory = DEFAULT_DIRECTORY.to_string();

    // Parse command-line arguments
    let mut args = std::env::args().skip(1);
    if let Some(directory) = args.next() {
        file_directory = directory;
    }

    println!("{}Starting server...", LOG_PREFIX);

    // Bind the server to the address
    let listener = TcpListener::bind(SERVER_ADDRESS)?;
    println!("{}Listening on {}", LOG_PREFIX, SERVER_ADDRESS);

    // Accept connections and spawn a new thread for each one
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let file_directory = file_directory.clone();
                thread::spawn(move || {
                    if let Err(e) = handle_connection(stream, &file_directory) {
                        eprintln!("Failed to handle connection: {}", e);
                    }
                });
            }
            Err(e) => eprintln!("Connection failed: {}", e),
        }
    }

    Ok(())
}

fn handle_connection(mut stream: TcpStream, file_directory: &str) -> io::Result<()> {
    // Read the request
    let mut reader = BufReader::new(&stream);
    let mut request_line = String::new();
    reader.read_line(&mut request_line)?;

    // Parse the request line
    let mut parts = request_line.trim().split_whitespace();
    let method = parts.next().unwrap_or("");
    let path = parts.next().unwrap_or("");

    // Read headers
    let mut headers = HashMap::new();
    loop {
        let mut line = String::new();
        reader.read_line(&mut line)?;
        if line == "\r\n" || line == "" {
            break;
        }
        let parts: Vec<&str> = line.splitn(2, ": ").collect();
        if parts.len() == 2 {
            headers.insert(parts[0].to_string(), parts[1].trim().to_string());
        }
    }

    match method {
        "GET" => handle_get_request(&mut stream, path, file_directory, &headers),
        "POST" => handle_post_request(&mut stream, path, file_directory, &headers, &mut reader),
        _ => send_response(&mut stream, HTTP_STATUS_NOT_FOUND, &HashMap::new(), ""),
    }
}

fn handle_get_request(
    stream: &mut TcpStream,
    path: &str,
    file_directory: &str,
    headers: &HashMap<String, String>,
) -> io::Result<()> {
    match path {
        "/" => send_response(stream, HTTP_STATUS_OK, &HashMap::new(), ""),
        _ if path.starts_with("/echo/") => {
            let echo_string = &path[6..];
            handle_echo_request(stream, echo_string, headers)
        }
        "/user-agent" => handle_user_agent_request(stream, headers),
        _ if path.starts_with("/files/") => {
            let filename = &path[7..];
            handle_file_request(stream, file_directory, filename)
        }
        _ => send_response(stream, HTTP_STATUS_NOT_FOUND, &HashMap::new(), ""),
    }
}

fn handle_echo_request(
    stream: &mut TcpStream,
    echo_string: &str,
    headers: &HashMap<String, String>,
) -> io::Result<()> {
    let content_type = CONTENT_TYPE_TEXT_PLAIN.to_string();
    let content_length = echo_string.len();
    let headers = create_headers(&content_type, content_length, headers, false)?;

    send_response(stream, HTTP_STATUS_OK, &headers, echo_string)
}

fn handle_user_agent_request(
    stream: &mut TcpStream,
    headers: &HashMap<String, String>,
) -> io::Result<()> {
    let user_agent = headers
        .get("User-Agent")
        .unwrap_or(&"".to_string())
        .to_string();
    let content_type = CONTENT_TYPE_TEXT_PLAIN.to_string();
    let content_length = user_agent.len();
    let headers = create_headers(&content_type, content_length, headers, false)?;

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
            let headers = create_headers(&content_type, content_length, &HashMap::new(), true)?;

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

fn handle_post_request(
    stream: &mut TcpStream,
    path: &str,
    file_directory: &str,
    headers: &HashMap<String, String>,
    reader: &mut BufReader<&TcpStream>,
) -> io::Result<()> {
    let content_length = headers
        .get(CONTENT_LENGTH_HEADER)
        .unwrap_or(&"0".to_string())
        .parse::<usize>()
        .unwrap_or(0);
    let mut body = vec![0; content_length];
    reader.read_exact(&mut body)?;

    let file_path = Path::new(file_directory).join(path.trim_start_matches("/"));
    let mut file = File::create(&file_path)?;

    file.write_all(&body)?;

    send_response(stream, HTTP_STATUS_CREATED, &HashMap::new(), "")
}

fn create_headers(
    content_type: &str,
    content_length: usize,
    headers: &HashMap<String, String>,
    gzip: bool,
) -> io::Result<HashMap<String, String>> {
    let mut new_headers = HashMap::new();
    new_headers.insert(CONTENT_TYPE_HEADER.to_string(), content_type.to_string());
    new_headers.insert(
        CONTENT_LENGTH_HEADER.to_string(),
        content_length.to_string(),
    );

    if gzip {
        new_headers.insert(
            CONTENT_ENCODING_HEADER.to_string(),
            CONTENT_ENCODING_GZIP.to_string(),
        );
    }

    for (key, value) in headers.iter() {
        new_headers.insert(key.clone(), value.clone());
    }

    Ok(new_headers)
}

fn send_response(
    stream: &mut TcpStream,
    status: &str,
    headers: &HashMap<String, String>,
    body: &str,
) -> io::Result<()> {
    let mut response = format!("{}\r\n", status);

    for (key, value) in headers.iter() {
        response += &format!("{}: {}\r\n", key, value);
    }

    response += "\r\n";
    response += body;

    stream.write_all(response.as_bytes())?;

    Ok(())
}
