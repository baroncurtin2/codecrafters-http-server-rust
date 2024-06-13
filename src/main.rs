use std::{
    io::{self, BufRead, BufReader, Write},
    net::{TcpListener, TcpStream},
    thread,
};

fn main() -> io::Result<()> {
    // Bind the server to the address
    let listener = TcpListener::bind("127.0.0.1:4221")?;
    println!("Server listening on http://127.0.0.1:4221");

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

fn handle_connection(mut stream: TcpStream) -> io::Result<()> {
    // Read the request
    let mut reader = BufReader::new(&stream);
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
    reader.read_to_string(&mut headers)?;

    // Handle the request based on the path
    match method {
        "GET" => handle_get_request(&mut stream, path, &headers),
        _ => send_response(&mut stream, "HTTP/1.1 405 Method Not Allowed\r\n\r\n"),
    }
}

fn handle_get_request(stream: &mut TcpStream, path: &str, headers: &str) -> io::Result<()> {
    if path.starts_with("/echo/") {
        handle_echo_request(stream, &path[6..])
    } else if path == "/" {
        send_response(stream, "HTTP/1.1 200 OK\r\n\r\n")
    } else if path == "/user-agent" {
        handle_user_agent_request(stream, headers)
    } else {
        send_response(stream, "HTTP/1.1 404 Not Found\r\n\r\n")
    }
}

fn handle_echo_request(stream: &mut TcpStream, echo_string: &str) -> io::Result<()> {
    let response_body = echo_string;
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
        response_body.len(),
        response_body
    );
    send_response(stream, &response)
}

fn handle_user_agent_request(stream: &mut TcpStream, headers: &str) -> io::Result<()> {
    // Extract the User-Agent header
    let user_agent = headers
        .lines()
        .find(|line| line.to_lowercase().starts_with("user-agent:"))
        .map(|line| line.splitn(2, ": ").nth(1).unwrap_or("").to_string())
        .unwrap_or_default();

    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
        user_agent.len(),
        user_agent
    );
    send_response(stream, &response)
}

fn send_response(stream: &mut TcpStream, response: &str) -> io::Result<()> {
    stream.write_all(response.as_bytes())?;
    stream.flush()?;
    Ok(())
}
