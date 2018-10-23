extern crate clap;
extern crate regex;

use clap::{App, Arg};
use regex::Regex;
use std::fs;
use std::io::prelude::*;
use std::net::TcpListener;
use std::net::TcpStream;

fn main() {
    let re = Regex::new(r"([^ ]+) /([^ ]*) (.*)").unwrap();

    let matches = App::new("lwebservr")
        .version("1.0")
        .author("chriss@frankeonline.net")
        .about("Serve local files as webserver")
        .arg(
            Arg::with_name("port")
                .short("p")
                .long("port")
                .value_name("PORT")
                .help("port")
                .takes_value(true),
        ).arg(
            Arg::with_name("verbose")
                .short("v")
                .long("verbose")
                .help("verbose"),
        ).get_matches();
    let port = matches
        .value_of("port")
        .unwrap_or("8080")
        .parse::<u16>()
        .expect("port must be a number between 1 and 65535");

    let verbose = matches.is_present("verbose");

    println!(
        "Starting webserver on port {}, files will be served from {}",
        port,
        std::env::current_dir().unwrap().display()
    );

    let listener = TcpListener::bind(format!("127.0.0.1:{}", port)).unwrap();

    for stream in listener.incoming() {
        let stream = stream.unwrap();

        handle_connection(stream, verbose, &re);
    }
}

fn handle_connection(mut stream: TcpStream, verbose: bool, re: &Regex) {
    let mut buffer = [0; 512];

    stream.read(&mut buffer).unwrap();
    let request = String::from_utf8_lossy(&buffer[..]);
    let mut request_lines = request.lines();

    let first_line = request_lines.next().unwrap();

    let caps = re.captures(&first_line).unwrap();
    let method = caps.get(1).unwrap().as_str();
    let path = caps.get(2).unwrap().as_str();

    let filename = match path {
        "" => "index.html",
        p => p,
    };

    let mut path_to_file = std::env::current_dir().unwrap();
    path_to_file.push(filename);

    let ip = stream.peer_addr().unwrap().ip();
    println!(
        "{} /{} from {} -> {}",
        method,
        path,
        ip,
        path_to_file
            .strip_prefix(std::env::current_dir().unwrap())
            .unwrap()
            .display()
    );

    let response = match fs::read_to_string(path_to_file) {
        Ok(c) => format!(
            "HTTP/1.0 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\n\r\n{}",
            c.len(),
            c
        ),
        Err(_) => format!("HTTP/1.0 404 OK\r\n\r\n"),
    };

    stream.write(response.as_bytes()).unwrap();
    stream.flush().unwrap();

    if verbose {
        println!("{}", request);
    }
}
