use std::{
    env,
    fs::{self, File},
    io::{BufRead, BufReader, Read, Write},
    net::TcpListener,
    path::Path,
    thread,
};

use itertools::Itertools;

fn handle_client(mut stream: std::net::TcpStream) {
    println!("accepted new connection");

    let mut req = String::new();
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    if let Err(e) = reader.read_line(&mut req) {
        eprintln!("Error reading request: {}", e);
        return;
    }

    if let Some(path) = req.split_whitespace().nth(1) {
        let parts: Vec<_> = path.split_terminator('/').skip(1).collect();

        match parts.as_slice() {
            ["echo", s] => {
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
                    s.len(),
                    s
                );
                if let Err(e) = stream.write_all(response.as_bytes()) {
                    eprintln!("Error writing response: {}", e);
                }
            }
            ["user-agent"] => {
                let mut lines = reader.lines();
                while let Some(Ok(line)) = lines.next() {
                    if let Some(s) = line.strip_prefix("User-Agent: ") {
                        let response = format!("HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}", s.len(), s);
                        if let Err(e) = stream.write_all(response.as_bytes()) {
                            eprintln!("Error writing response: {}", e);
                        }
                        break;
                    }
                }
            }
            ["files", filename] => {
                if req.starts_with("POST") {
                    // Handle POST request to create a new file
                    let mut content_length = 0;
                    let mut lines = reader.lines();
                    while let Some(Ok(line)) = lines.next() {
                        if line.starts_with("Content-Length: ") {
                            if let Ok(len) =
                                line.trim_start_matches("Content-Length: ").parse::<usize>()
                            {
                                content_length = len;
                            }
                        }
                        if line == "\r\n" {
                            break;
                        }
                    }
                    let mut body = vec![0; content_length];
                    if let Err(e) = reader.read_exact(&mut body) {
                        eprintln!("Error reading request body: {}", e);
                        return;
                    }

                    // Create or overwrite the file with the received content
                    if let Some(dir) = env::args().nth(2) {
                        let file_path = Path::new(&dir).join(filename);
                        if let Err(e) = fs::write(&file_path, &body) {
                            eprintln!("Error writing to file: {}", e);
                            return;
                        }
                        let response = format!("HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\nFile created successfully\n", body.len());
                        if let Err(e) = stream.write_all(response.as_bytes()) {
                            eprintln!("Error writing response: {}", e);
                        }
                    } else {
                        if let Err(e) =
                            stream.write_all(b"HTTP/1.1 500 Internal Server Error\r\n\r\n")
                        {
                            eprintln!("Error writing response: {}", e);
                        }
                    }
                } else {
                    // Handle GET request to serve existing file
                    if let Some(dir) = env::args().nth(2) {
                        let file_path = Path::new(&dir).join(filename);
                        if let Ok(mut file) = File::open(&file_path) {
                            let mut buf = Vec::new();
                            if let Err(e) = file.read_to_end(&mut buf) {
                                eprintln!("Error reading file: {}", e);
                                if let Err(e) =
                                    stream.write_all(b"HTTP/1.1 500 Internal Server Error\r\n\r\n")
                                {
                                    eprintln!("Error writing response: {}", e);
                                }
                                return;
                            }
                            let response = format!("HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\n\r\n", buf.len());
                            if let Err(e) = stream.write_all(response.as_bytes()) {
                                eprintln!("Error writing response: {}", e);
                            }
                            if let Err(e) = stream.write_all(&buf) {
                                eprintln!("Error writing file data: {}", e);
                            }
                        } else {
                            if let Err(e) = stream.write_all(b"HTTP/1.1 404 Not Found\r\n\r\n") {
                                eprintln!("Error writing response: {}", e);
                            }
                        }
                    } else {
                        if let Err(e) =
                            stream.write_all(b"HTTP/1.1 500 Internal Server Error\r\n\r\n")
                        {
                            eprintln!("Error writing response: {}", e);
                        }
                    }
                }
            }
            [] => {
                if let Err(e) = stream.write_all(b"HTTP/1.1 200 OK\r\n\r\n") {
                    eprintln!("Error writing response: {}", e);
                }
            }
            _ => {
                if let Err(e) = stream.write_all(b"HTTP/1.1 404 Not Found\r\n\r\n") {
                    eprintln!("Error writing response: {}", e);
                }
            }
        }
    }
}

fn main() {
    println!("Logs from your program will appear here!");

    if let Ok(listener) = TcpListener::bind("127.0.0.1:4221") {
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    thread::spawn(move || {
                        handle_client(stream);
                    });
                }
                Err(e) => {
                    eprintln!("Error accepting connection: {}", e);
                }
            }
        }
    } else {
        eprintln!("Could not bind to address");
    }
}
