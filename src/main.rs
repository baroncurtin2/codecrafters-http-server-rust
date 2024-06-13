use std::{
    fs::File,
    io::{self, BufRead, Read, Write},
    net::{TcpListener, TcpStream},
    path::{Path, PathBuf},
    thread,
};

// Constants for HTTP responses
const HTTP_OK: &str = "HTTP/1.1 200 OK\r\n";
const HTTP_NOT_FOUND: &str = "HTTP/1.1 404 Not Found\r\n";
const HTTP_METHOD_NOT_ALLOWED: &str = "HTTP/1.1 405 Method Not Allowed\r\n";
const CONTENT_TYPE_OCTET_STREAM: &str = "Content-Type: application/octet-stream\r\n";

fn main() -> io::Result<()> {
    // Parse command-line arguments
    let mut args = std::env::args().skip(1);
    let directory = match args.next() {
        Some(dir) => dir,
        None => {
            eprintln!("Missing directory argument");
            return Ok(());
        }
    };

    // Bind the server to the address
    let listener = TcpListener::bind("localhost:4221")?;
    println!("Server listening on http://localhost:4221");

    // Accept connections and spawn a new thread for each one
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let directory = directory.clone(); // Clone for each thread
                thread::spawn(move || {
                    if let Err(e) = handle_connection(stream, &directory) {
                        eprintln!("Failed to handle connection: {}", e);
                    }
                });
            }
            Err(e) => eprintln!("Connection failed: {}", e),
        }
    }

    Ok(())
}

fn handle_connection(mut stream: TcpStream, directory: &str) -> io::Result<()> {
    // Read the request line
    let mut reader = io::BufReader::new(&stream);
    let mut request_line = String::new();
    reader.read_line(&mut request_line)?;

    // Parse the request line
    let parts: Vec<&str> = request_line.trim_end().split_whitespace().collect();
    if parts.len() < 3 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Invalid HTTP request line",
        ));
    }

    let method = parts[0];
    let path = parts[1];

    // Read headers
    let mut headers = String::new();
    loop {
        let mut line = String::new();
        reader.read_line(&mut line)?;
        if line == "\r\n" {
            break;
        }
        headers.push_str(&line)
    }

    handle_request(&mut stream, method, path, directory, &headers)?;

    Ok(())
}

fn handle_request(
    stream: &mut TcpStream,
    method: &str,
    path: &str,
    directory: &str,
    headers: &str,
) -> io::Result<()> {
    match method {
        "GET" => handle_get_request(stream, path, directory),
        _ => send_response(stream, HTTP_METHOD_NOT_ALLOWED),
    }
}

fn handle_get_request(stream: &mut TcpStream, path: &str, directory: &str) -> io::Result<()> {
    if path.starts_with("/files/") {
        let filename = &path[7..]; // Trim "/files/"

        // Construct the full path to the requested file
        let mut file_path = PathBuf::from(directory);
        file_path.push(filename);

        // Try to open the file
        match File::open(&file_path) {
            Ok(mut file) => {
                // Get file metadata to determine Content-Length
                let metadata = file.metadata()?;
                let file_size = metadata.len() as usize; // Convert to usize for Content-Length

                // Prepare the response headers
                let response_headers = format!(
                    "{}{}Content-Length: {}\r\n\r\n",
                    HTTP_OK, CONTENT_TYPE_OCTET_STREAM, file_size
                );

                // Send headers
                stream.write_all(response_headers.as_bytes())?;

                // Send file content in chunks to avoid large memory allocations
                let mut buffer = vec![0; 1024];
                loop {
                    let bytes_read = file.read(&mut buffer)?;
                    if bytes_read == 0 {
                        break;
                    }
                    stream.write_all(&buffer[..bytes_read])?;
                }
            }
            Err(_) => {
                send_response(stream, HTTP_NOT_FOUND)?;
            }
        }
    } else {
        send_response(stream, HTTP_NOT_FOUND)?;
    }

    Ok(())
}

fn send_response(stream: &mut TcpStream, response: &str) -> io::Result<()> {
    stream.write_all(response.as_bytes())?;
    stream.flush()?;
    Ok(())
}
