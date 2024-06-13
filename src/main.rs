use itertools::Itertools;
use std::{
    env,
    fs::File,
    io::{self, BufRead, BufReader, Read, Write},
    net::TcpListener,
    path::Path,
    thread,
};

fn main() -> io::Result<()> {
    println!("Logs from your program will appear here!");

    let listener = TcpListener::bind("127.0.0.1:4221")?;

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

fn handle_connection(stream: std::net::TcpStream) -> io::Result<()> {
    println!("Accepted new connection");

    let mut reader = BufReader::new(&stream);
    let mut req = String::new();
    reader.read_line(&mut req)?;

    if let Some(path) = req.split_whitespace().nth(1) {
        match path {
            "/user-agent" => handle_user_agent(&mut reader, &stream)?,
            "/echo" => handle_echo(&req, &stream)?,
            p if p.starts_with("/files/") => handle_file_request(p, &stream)?,
            "/" => stream.write_all(b"HTTP/1.1 200 OK\r\n\r\n")?,
            _ => stream.write_all(b"HTTP/1.1 404 Not Found\r\n\r\n")?,
        }
    }

    Ok(())
}

fn handle_user_agent(reader: &mut BufReader<&TcpStream>, stream: &TcpStream) -> io::Result<()> {
    let user_agent = reader
        .lines()
        .find_map(|line| {
            line.ok()
                .and_then(|l| l.strip_prefix("User-Agent: ").map(|s| s.to_string()))
        })
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "User-Agent header not found"))?;

    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
        user_agent.len(),
        user_agent
    );
    stream.write_all(response.as_bytes())?;

    Ok(())
}

fn handle_echo(req: &str, stream: &TcpStream) -> io::Result<()> {
    let s = req.splitn(2, ' ').nth(1).unwrap_or("");
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
        s.len(),
        s
    );
    stream.write_all(response.as_bytes())?;

    Ok(())
}

fn handle_file_request(path: &str, stream: &TcpStream) -> io::Result<()> {
    let file_path = path.trim_start_matches("/files/");
    let directory = env::args()
        .nth(2)
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Directory argument missing"))?;
    let full_path = Path::new(&directory).join(file_path);

    let mut file = File::open(&full_path).map_err(|e| {
        eprintln!("Error opening file {}: {}", full_path.display(), e);
        io::Error::new(io::ErrorKind::NotFound, "File not found")
    })?;

    let metadata = file.metadata()?;
    let mut buffer = vec![0; metadata.len() as usize];
    file.read_exact(&mut buffer)?;

    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\n\r\n",
        buffer.len()
    );
    stream.write_all(response.as_bytes())?;
    stream.write_all(&buffer)?;

    Ok(())
}
