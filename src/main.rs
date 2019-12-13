extern crate clap;
extern crate regex;

#[macro_use]
extern crate error_chain;

use clap::{App, Arg};
use regex::Regex;
use std::fs;
use std::io::prelude::*;
use std::net::TcpListener;
use std::net::TcpStream;

mod errors {
    error_chain!{}
}

use errors::*;

const VERSION: &'static str = env!("CARGO_PKG_VERSION");
fn main() {
    if let Err(ref e) = run() {
        println!("error: {}", e);
        for e in e.iter().skip(1) {
            println!("caused by: {}", e)
        }
        if let Some(backtrace) = e.backtrace() {
            println!("backtrace: {:?}", backtrace);
        }
        std::process::exit(1);
    }
}
fn take(s:String) ->() {
}
fn run() -> Result<()> {
    let re = Regex::new(r"([^ ]+) /([^ ]*) (.*)").unwrap();

    let matches = App::new("lwebservr")
        .version(VERSION)
        .author("chriss@frankeonline.net")
        .about("Serve local files via http")
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
        ).arg(
            Arg::with_name("silent")
                .short("s")
                .long("silent")
                .help("silent"),
        ).get_matches();
    let port = matches
        .value_of("port")
        .unwrap_or("8080")
        .parse::<u16>()
        .chain_err(|| "port must be a number between 1 and 65535")?;

    let verbose = matches.is_present("verbose");
    let silent = matches.is_present("silent");
    let a = "HALLO".to_string();
    take(a.clone());
    println!("{}",a);
    if !silent {
        println!(
            "Starting webserver on port {}, files will be served from {}",
            port,
            std::env::current_dir().unwrap().display()
        );
    }

    let listener = TcpListener::bind(format!("127.0.0.1:{}", port))
        .chain_err(|| format!("could not bind to 127.0.0.1:{}", port))?;

    for stream in listener.incoming() {
        let stream = stream.unwrap();

        handle_connection(stream, verbose, silent, &re)?;
    }
    Ok(())
}

#[derive(Debug)]
struct HttpResult<'a> {
    status: u16,
    msg: &'a str,
    body: String,
    content_type: Option<&'a str>,
}
impl<'a> HttpResult<'a> {
    fn ok(content_type: &'a str, body: String) -> HttpResult<'a> {
        HttpResult {
            status: 200,
            msg: "OK",
            body: body,
            content_type: Some(content_type),
        }
    }
    fn not_found() -> HttpResult<'a> {
        HttpResult {
            status: 404,
            msg: "Not Found",
            body: "".to_owned(),
            content_type: None,
        }
    }
    fn method_not_allowed() -> HttpResult<'a> {
        HttpResult {
            status: 405,
            msg: "Method not allowed",
            body: "".to_owned(),
            content_type: None,
        }
    }
}

fn handle_connection(mut stream: TcpStream, verbose: bool, silent: bool, re: &Regex) -> Result<()> {
    let mut buffer = [0; 512];

    stream
        .read(&mut buffer)
        .chain_err(|| "could not read from tcp stream")?;
    let request = String::from_utf8_lossy(&buffer[..]);
    let mut request_lines = request.lines();

    let first_line = request_lines.next().ok_or("no next line")?;

    let caps = re.captures(first_line).ok_or("bad first http line")?;
    let method = caps.get(1).ok_or("could not get method part")?.as_str();
    let path = caps.get(2).ok_or("could not get method part")?.as_str();

    let ip = stream.peer_addr().chain_err(|| "could not get ip")?.ip();

    let result = (match method.to_uppercase().as_str() {
        "GET" => handle_get(path),
        _x => Ok(HttpResult::method_not_allowed()),
    }).unwrap();

    if !silent {
        println!(
            "{method} /{path} from {ip} -> {status} {msg}",
            method = method,
            path = path,
            ip = ip,
            status = result.status,
            msg = result.msg
        );
    }
    let response_body = match result.status {
        200 => format!(
            "HTTP/1.0 {status}\r\n{content_type}Content-Length: {length}\r\n\r\n{body}",
            status = result.status,
            length = result.body.len(),
            body = result.body,
            content_type = result
                .content_type
                .map(|c| format!("Content-Type: {}\r\n", c))
                .unwrap_or("".to_owned())
        ),
        _e => format!(
            "HTTP/1.0 {status} {msg}\r\n",
            status = result.status,
            msg = result.msg
        ),
    };

    stream.write(response_body.as_bytes()).unwrap();
    stream.flush().unwrap();

    if verbose {
        println!("{}", request);
    }
    Ok(())
}
fn calculate_content_type(filename: &str) -> &str {
    let mut v: Vec<&str> = filename.split('.').collect();
    match v.pop() {
        Some("html") => "text/html",
        Some("png") => "image/png",
        Some("txt") => "text/plain",
        Some("js") => "text/javascript",
        Some("css") => "text/css",
        _x => "application/octet-stream",
    }
}
fn handle_get(path: &str) -> Result<HttpResult> {
    let filename = match path {
        "" => "index.html",
        p => p,
    };
    let content_type = calculate_content_type(filename);
    let mut path_to_file = std::env::current_dir().unwrap();
    path_to_file.push(filename);
    fs::read_to_string(path_to_file)
        .map(|content| HttpResult::ok(content_type, content))
        .or(Ok(HttpResult::not_found()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calculate_content_type_test() {
        assert_eq!("text/html".to_owned(), calculate_content_type("index.html"));
        assert_eq!(
            "application/octet-stream".to_owned(),
            calculate_content_type("index.pdf")
        );
    }
}
