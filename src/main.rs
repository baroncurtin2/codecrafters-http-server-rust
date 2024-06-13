use itertools::Itertools;
use std::{
    env,
    fs::File,
    io::{BufRead, BufReader, Read, Write},
    net::TcpListener,
    path::Path,
    thread,
};
fn main() {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");
    // Uncomment this block to pass the first stage
    //
    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                thread::spawn(move || {
                    println!("accepted new connection");
                    let mut req = String::new();
                    let mut reader = BufReader::new(stream.try_clone().unwrap());
                    reader.read_line(&mut req).unwrap();
                    if let Some(path) = req.split_whitespace().nth(1) {
                        let parts = path.split_terminator('/').skip(1).collect_vec();
                        match parts.as_slice() {
                        ["echo", s] => stream.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}", s.len(), s).as_bytes()).unwrap(),
                        ["user-agent"] => {
                            let mut lines = reader.lines();
                            while let Some(Ok(line)) = lines.next() {
                                if let Some(s) = line.strip_prefix("User-Agent: ")  {
                                    stream.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}", s.len(), s).as_bytes()).unwrap();
                                    break;
                                }
                            }
                        }
                        ["files", f] => {
                            if let Some(dir) = env::args().nth(2) {
                                if let Ok(mut file) = File::open(Path::new(&dir).join(f)) {
                                    let mut buf = Vec::new();
                                    file.read_to_end(&mut buf).unwrap();
                                    stream.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\n\r\n", buf.len()).as_bytes()).unwrap();
                                    stream.write_all(buf.as_slice()).unwrap();
                                } else {
                                    stream.write_all(b"HTTP/1.1 404 Not Found\r\n\r\n").unwrap();
                                }
                            }
                        }
                        [] => stream.write_all(b"HTTP/1.1 200 OK\r\n\r\n").unwrap(),
                        _ => stream.write_all(b"HTTP/1.1 404 Not Found\r\n\r\n").unwrap()
                    }
                    }
                });
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}
