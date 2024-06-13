use std::{
    io::{self, BufRead, Write},
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
    // read the first line of the request
    let mut reader = io::BufReader::new(&stream);
    let mut request_line = String::new();
    reader.read_line(&mut request_line)?;

    // parse the request line
    let parts: Vec<&str> = request_line.trim_end().split_whitespace().collect();
    if parts.len() < 3 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Invalid HTTP request line",
        ));
    }

    let method = parts[0];
    let path = parts[1];
    let _version = parts[2];

    let response = match path {
        "/" => "HTTP/1.1 200 OK\r\n\r\n",
        _ => "HTTP/1.1 404 Not Found\r\n\r\n",
    };

    // Write the response to the stream
    stream.write_all(response.as_bytes())?;
    stream.flush()?;

    Ok(())
}
