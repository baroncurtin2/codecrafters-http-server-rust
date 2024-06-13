// Uncomment this block to pass the first stage
use std::{
    io::{self, Write},
    net::{TcpListener, TcpStream},
    thread,
};

fn main() -> io::Result<()> {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");

    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream in listener.incoming() {
        let stream = stream?;
        thread::spawn(|| {
            handle_connection(stream);
        });
    }

    Ok(())
}

fn handle_connection(mut stream: TcpStream) {
    let response = "HTTP/1.1 200 OK\r\n\r\n";

    // write the response to the stream
    stream.write_all(response.as_bytes()).unwrap();
    stream.flush().unwrap();
}
