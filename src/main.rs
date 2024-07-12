use std::io::{prelude::*, BufReader};
use std::net::{TcpListener, TcpStream};

fn main() -> std::io::Result<()> {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");

    let listener = TcpListener::bind("127.0.0.1:4221").expect("Could not bind server.");

    for stream in listener.incoming() {
        handle_request(stream?);
    }

    Ok(())
}

fn handle_request(mut stream: TcpStream) {
    let buf_reader = BufReader::new(&mut stream);

    let requested_line = buf_reader.lines().next().unwrap().unwrap();
    let route = requested_line.split_whitespace().nth(1).unwrap();

    dbg!(&requested_line);
    dbg!(route);

    if route != "/" {
        stream
            .write_all("HTTP/1.1 404 Not Found\r\n\r\n".as_bytes())
            .unwrap();

        return;
    }

    let response = "HTTP/1.1 200 OK\r\n\r\n";
    stream.write_all(response.as_bytes()).unwrap();
}
