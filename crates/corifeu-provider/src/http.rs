//! A POST, a JSON body, and a reply. Nothing else.
//!
//! Corifeu's local runtimes listen on the loopback interface over plain HTTP
//! (ADR-0008), so none of what an HTTP library exists for applies: no TLS, no
//! redirects, no cookies, no proxy, no connection pool. Taking a dependency for
//! this would add a transitive tree to a project whose whole dependency set is
//! six crates and a `cargo-deny` gate over them.
//!
//! When direct provider APIs arrive (ADR-0009, MVP 2) they need TLS and this
//! stops being enough. That is the moment to take a real client, with the
//! evidence of having needed it.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0} is not a plain http:// URL; local runtimes are reached over loopback")]
    NotPlainHttp(String),

    #[error("{host}: {source}")]
    Connect {
        host: String,
        #[source]
        source: std::io::Error,
    },

    #[error("{0}")]
    Io(#[from] std::io::Error),

    #[error("the server replied {status} {reason}: {body}")]
    Status {
        status: u16,
        reason: String,
        body: String,
    },

    #[error("the reply was not valid HTTP: {0}")]
    Malformed(String),
}

/// POST `body` as JSON and return the reply body.
pub fn post_json(url: &str, body: &str, timeout: Duration) -> Result<String, Error> {
    request(url, Some(body), timeout)
}

/// GET and return the reply body. Used to ask a router which models it serves.
pub fn get(url: &str, timeout: Duration) -> Result<String, Error> {
    request(url, None, timeout)
}

fn request(url: &str, body: Option<&str>, timeout: Duration) -> Result<String, Error> {
    let (host, path) = split(url)?;
    let address = host
        .to_socket_addrs()
        .map_err(|source| Error::Connect {
            host: host.clone(),
            source,
        })?
        .next()
        .ok_or_else(|| Error::Connect {
            host: host.clone(),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "no address"),
        })?;

    let stream =
        TcpStream::connect_timeout(&address, timeout).map_err(|source| Error::Connect {
            host: host.clone(),
            source,
        })?;
    // A local model generating under a large budget is slow, not stuck. The
    // read timeout is the caller's deadline, not a network one.
    stream.set_read_timeout(Some(timeout))?;
    stream.set_write_timeout(Some(timeout))?;
    stream.set_nodelay(true)?;

    let mut stream = stream;
    match body {
        Some(body) => {
            write!(
                stream,
                "POST {path} HTTP/1.1\r\n\
                 Host: {host}\r\n\
                 Content-Type: application/json\r\n\
                 Content-Length: {}\r\n\
                 Connection: close\r\n\
                 \r\n",
                body.len()
            )?;
            stream.write_all(body.as_bytes())?;
        }
        None => write!(
            stream,
            "GET {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n"
        )?,
    }
    stream.flush()?;

    read_reply(BufReader::new(stream))
}

fn split(url: &str) -> Result<(String, String), Error> {
    let rest = url
        .strip_prefix("http://")
        .ok_or_else(|| Error::NotPlainHttp(url.to_string()))?;
    let (authority, path) = match rest.find('/') {
        Some(at) => (&rest[..at], &rest[at..]),
        None => (rest, "/"),
    };
    let host = if authority.contains(':') {
        authority.to_string()
    } else {
        format!("{authority}:80")
    };
    Ok((host, path.to_string()))
}

fn read_reply<R: Read>(mut reader: BufReader<R>) -> Result<String, Error> {
    let mut line = String::new();
    reader.read_line(&mut line)?;
    let mut status_parts = line.trim_end().splitn(3, ' ');
    let _version = status_parts.next();
    let status: u16 = status_parts
        .next()
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| Error::Malformed(line.trim_end().to_string()))?;
    let reason = status_parts.next().unwrap_or("").to_string();

    let mut length = None;
    let mut chunked = false;
    loop {
        let mut header = String::new();
        if reader.read_line(&mut header)? == 0 {
            return Err(Error::Malformed("the headers ended early".into()));
        }
        let header = header.trim_end();
        if header.is_empty() {
            break;
        }
        let Some((name, value)) = header.split_once(':') else {
            continue;
        };
        match name.trim().to_ascii_lowercase().as_str() {
            "content-length" => length = value.trim().parse::<usize>().ok(),
            "transfer-encoding" => {
                chunked = value.to_ascii_lowercase().contains("chunked");
            }
            _ => {}
        }
    }

    let body = if chunked {
        read_chunked(&mut reader)?
    } else if let Some(length) = length {
        let mut buffer = vec![0; length];
        reader.read_exact(&mut buffer)?;
        String::from_utf8_lossy(&buffer).into_owned()
    } else {
        let mut buffer = Vec::new();
        reader.read_to_end(&mut buffer)?;
        String::from_utf8_lossy(&buffer).into_owned()
    };

    if !(200..300).contains(&status) {
        return Err(Error::Status {
            status,
            reason,
            body,
        });
    }
    Ok(body)
}

/// Ollama answers a non-streaming request with `Content-Length`, but llama.cpp's
/// server does not always, so both framings are handled rather than one.
fn read_chunked<R: Read>(reader: &mut BufReader<R>) -> Result<String, Error> {
    let mut body = Vec::new();
    loop {
        let mut header = String::new();
        reader.read_line(&mut header)?;
        let size =
            usize::from_str_radix(header.trim_end().split(';').next().unwrap_or("").trim(), 16)
                .map_err(|_| Error::Malformed(format!("chunk size {:?}", header.trim_end())))?;
        if size == 0 {
            break;
        }
        let mut chunk = vec![0; size];
        reader.read_exact(&mut chunk)?;
        body.extend_from_slice(&chunk);
        let mut crlf = [0; 2];
        reader.read_exact(&mut crlf)?;
    }
    Ok(String::from_utf8_lossy(&body).into_owned())
}
