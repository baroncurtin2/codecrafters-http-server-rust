use std::{
    fs::File,
    io::{self, BufRead, BufReader, Read, Write},
    net::{TcpListener, TcpStream},
    path::PathBuf,
    thread,
};

const ADDRESS: &str = "localhost:4221";
const RESPONSE_200: &str = "HTTP/1.1 200 OK\r\n";
const RESPONSE_404: &str = "HTTP/1.1 404 Not Found\r\n";
const RESPONSE_405: &str = "HTTP/1.1 405 Method Not Allowed\r\n";

// Directory where files are stored
static mut FILES_DIRECTORY: Option<String> = None;

fn main() -> io::Result<()> {
    // Parse command-line arguments
    parse_args();

    // Bind the server to the address
    let listener = TcpListener::bind(ADDRESS)?;
    println!("Server listening on http://{}", ADDRESS);

    // Accept connections and spawn a new thread for each one
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                thread::spawn(|| {
                    if let Err(e) = handle_connection(stream) {
                        eprintln!("Failed to handle connection: {}", e);
                    }
                });
            }
            Err(e) => eprintln!("Connection failed: {}", e),
        }
    }

    Ok(())
}

fn parse_args() {
    let args: Vec<String> = std::env::args().collect();
    let mut i = 1;
    while i < args.len() {
        if let Some(val) = args.get(i) {
            if val == "--directory" {
                unsafe {
                    if let Some(dir) = args.get(i + 1) {
                        FILES_DIRECTORY = Some(dir.clone());
                    }
                }
            }
        }
        i += 1;
    }
}

fn handle_connection(mut stream: TcpStream) -> io::Result<()> {
    let mut reader = io::BufReader::new(&stream);
    let mut request_line = String::new();
    reader.read_line(&mut request_line)?;

    let parts: Vec<&str> = request_line.trim_end().split_whitespace().collect();
    if parts.len() < 3 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Invalid HTTP request line",
        ));
    }

    let method = parts[0];
    let path = parts[1];

    let mut headers = String::new();
    loop {
        let mut line = String::new();
        reader.read_line(&mut line)?;
        if line == "\r\n" {
            break;
        }
        headers.push_str(&line);
    }

    match method {
        "GET" => handle_get_request(&mut stream, path),
        _ => send_response(&mut stream, RESPONSE_405),
    }
}

fn handle_get_request(stream: &mut TcpStream, path: &str) -> io::Result<()> {
    let files_directory = unsafe {
        match &FILES_DIRECTORY {
            Some(dir) => dir.clone(),
            None => {
                eprintln!("Error: --directory argument not provided.");
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    "--directory argument not provided.",
                ));
            }
        }
    };

    let file_path = format!("{}{}", files_directory, path);

    let response = if path.starts_with("/files/") {
        let filename = &path[7..]; // Strip "/files/" prefix
        let full_path = PathBuf::from(&file_path);

        if let Ok(file) = File::open(&full_path) {
            let metadata = file.metadata()?;
            let content_length = metadata.len();

            let mut reader = BufReader::new(file);
            let mut contents = Vec::new();
            reader.read_to_end(&mut contents)?;

            format!(
                "{}Content-Type: application/octet-stream\r\nContent-Length: {}\r\n\r\n",
                RESPONSE_200, content_length
            ) + &String::from_utf8_lossy(&contents)
        } else {
            RESPONSE_404.to_string()
        }
    } else {
        RESPONSE_404.to_string()
    };

    send_response(stream, &response)
}

fn send_response(stream: &mut TcpStream, response: &str) -> io::Result<()> {
    stream.write_all(response.as_bytes())?;
    stream.write_all(b"\r\n")?; // Add CRLF after headers
    stream.flush()?;
    Ok(())
}
