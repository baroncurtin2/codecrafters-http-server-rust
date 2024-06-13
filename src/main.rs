use std::{
    env,
    fs::File,
    io::{self, BufRead, Write},
    net::{TcpListener, TcpStream},
    path::PathBuf,
    thread,
};

const ADDRESS: &str = "localhost:4221";
const RESPONSE_200: &str = "HTTP/1.1 200 OK\r\n";
const RESPONSE_404: &str = "HTTP/1.1 404 Not Found\r\n\r\n";
const CONTENT_TYPE_BINARY: &str = "Content-Type: application/octet-stream\r\n";
const CONTENT_LENGTH: &str = "Content-Length: ";

fn main() -> io::Result<()> {
    // Parse command-line arguments
    let mut args = env::args();
    args.next(); // Skip the first argument (program name)
    let directory = match args.next() {
        Some(dir) => dir,
        None => {
            eprintln!("Usage: ./your_server.sh --directory <directory>");
            return Ok(());
        }
    };

    // Bind the server to the address
    let listener = TcpListener::bind(ADDRESS)?;
    println!("Server listening on http://{}", ADDRESS);

    // Accept connections and spawn a new thread for each one
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
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
    // Read the first line of the request
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
        headers.push_str(&line);
    }

    // Handle the request based on the method and path
    match method {
        "GET" => handle_get_request(&mut stream, path, directory, &headers),
        _ => send_response(&mut stream, RESPONSE_404),
    }
}

fn handle_get_request(
    stream: &mut TcpStream,
    path: &str,
    directory: &str,
    _headers: &str,
) -> io::Result<()> {
    if path.starts_with("/files/") {
        let filename = &path[7..]; // Trim "/files/"

        // Construct the full path to the requested file
        let mut file_path = PathBuf::from(directory);
        file_path.push(filename);

        // Try to open the file
        match File::open(file_path) {
            Ok(mut file) => {
                // Get file metadata to determine Content-Length
                let metadata = file.metadata()?;
                let file_size = metadata.len();

                // Prepare the response headers
                let mut response_headers = String::new();
                response_headers.push_str(RESPONSE_200);
                response_headers.push_str(CONTENT_TYPE_BINARY);
                response_headers.push_str(&format!("{}{}\r\n\r\n", CONTENT_LENGTH, file_size));

                // Send headers
                send_response(stream, &response_headers)?;

                // Send file content
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
                send_response(stream, RESPONSE_404)?;
            }
        }
    } else {
        send_response(stream, RESPONSE_404)?;
    }

    Ok(())
}

fn send_response(stream: &mut TcpStream, response: &str) -> io::Result<()> {
    stream.write_all(response.as_bytes())?;
    stream.flush()?;
    Ok(())
}
