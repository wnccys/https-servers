use std::io::{prelude::*, BufReader};
use std::net::{TcpListener, TcpStream};
use std::{
    env, fs,
    path::Path,
    sync::{mpsc, Arc, Mutex},
    thread,
};

fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:4221").expect("Could not bind server.");
    println!("Server Initialized.");

    let pool = ThreadPool::new(4);

    for stream in listener.incoming() {
        let stream = stream.unwrap();

        pool.execute(|| {
            handle_request(stream);
        });
    }

    Ok(())
}

struct ThreadPool {
    workers: Vec<Worker>,
    sender: mpsc::Sender<Job>,
}

struct Worker {
    id: usize,
    thread: thread::JoinHandle<()>,
}

type Job = Box<dyn FnOnce() + Send + 'static>;

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Job>>>) -> Worker {
        Worker {
            id,
            thread: thread::spawn(move || loop {
                let job = receiver
                    .lock()
                    .expect("Error trying to lock thread. Probably in use by other thread.")
                    .recv()
                    .unwrap();

                println!("Worker {id} got a job; executing.");

                job();
            }),
        }
    }
}

impl ThreadPool {
    fn new(size: usize) -> ThreadPool {
        assert!(size > 0);

        let (sender, receiver) = mpsc::channel();

        let receiver = Arc::new(Mutex::new(receiver));

        let mut workers = Vec::with_capacity(size);

        for id in 0..size {
            workers.push(Worker::new(id, Arc::clone(&receiver)));
        }

        ThreadPool { workers, sender }
    }

    fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(f);

        self.sender.send(job).unwrap();
    }
}

fn handle_request(mut stream: TcpStream) {
    let mut buf_reader = BufReader::new(&mut stream).lines();
    // 0 => meta, 1 => headers, 2 => body;
    let mut request = [String::new(), String::new(), String::new()];
    request[0] = buf_reader.next().unwrap().unwrap();

    let route = request[0].split(' ').nth(1).unwrap().to_owned();

    for line in buf_reader {
        let line = line.unwrap().to_owned();

        println!("line: {}", line);

        if line.is_empty() {
            request[2].push_str(&line);
            break;
        } else {
            request[1].push_str(&line);
            request[1].push_str("\r\n");
        }
    }

    let response = match &route[..] {
        "/" => "HTTP/1.1 200 OK\r\n\r\n".to_string(),
        _ if route.split('/').nth(1).unwrap() == "echo" => handle_route(&route),
        _ if route.split('/').nth(1).unwrap() == "files" => handle_files(&route),
        "/user-agent" => handle_user_agent(&request[1]),
        _ => "HTTP/1.1 404 Not Found\r\n\r\n".to_string(),
    };

    dbg!(&response);
    stream.write_all(response.as_bytes()).unwrap();
    println!("response sent");
}

fn handle_route(route: &str) -> String {
    let str = route.split_once("/echo/").unwrap().1;
    let str_len = str.len().to_string().to_owned();
    let formatted_string = format!(
        "{}{}{}{}",
        "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: ", str_len, "\r\n\r\n", str
    );

    formatted_string
}

fn handle_user_agent(headers: &str) -> String {
    let header = headers
        .split("\r\n")
        .nth(1)
        .unwrap()
        .split_once(' ')
        .unwrap()
        .1;

    let header_len = header.len();

    format!(
        "{}{}{}{}",
        "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: ",
        header_len,
        "\r\n\r\n",
        header
    )
}

fn handle_files(route: &str) -> String {
    let args: Vec<String> = env::args().collect();
    if args[1] != "--directory" {
        return "HTTP/1.1 404 Not Found\r\n\r\n".to_string();
    }

    let file = route.split('/').nth(2).unwrap();
    let mut file_path = args[2].clone();
    file_path.push_str(file);

    let current_dir = env::current_dir().unwrap();
    let mut absolute_path = args[0].clone();
    absolute_path.push_str(&file_path);
    dbg!(&absolute_path);
    dbg!(&current_dir);
    // let mut absolute_path = current_dir.to_str().unwrap().to_string();
    // absolute_path.push_str(&file_path);
    // dbg!(&absolute_path);

    let requested_file = fs::read_to_string(absolute_path);
    dbg!(&requested_file);

    match requested_file {
        Ok(file_content) => {
            let file_len = file_content.len().to_string();
            format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\n\r\n{}",
                file_len, file_content,
            )
        }
        _ => "HTTP/1.1 404 Not Found\r\n\r\n".to_string(),
    }
}
