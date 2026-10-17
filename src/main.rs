use std::env;
use std::fs;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::exit;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use std::time::Duration;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

const HELP: &str = "srv — serve the current folder over HTTP

usage: srv [port] [--public]

  port       port to listen on (default 8000)
  --public   listen on all interfaces (default: localhost only)
  help       show this help
";

const MAX_HEADER_BYTES: u64 = 16 * 1024;
const MAX_CONNECTIONS: usize = 64;
const IO_TIMEOUT: Duration = Duration::from_secs(10);

/// Percent-decode into raw bytes, then validate as UTF-8.  Returns None for
/// malformed escapes or invalid UTF-8.
fn url_decode(s: &str) -> Option<String> {
    let mut out: Vec<u8> = Vec::with_capacity(s.len());
    let mut bytes = s.bytes();
    while let Some(b) = bytes.next() {
        match b {
            b'%' => {
                let h1 = bytes.next().and_then(|c| (c as char).to_digit(16))?;
                let h2 = bytes.next().and_then(|c| (c as char).to_digit(16))?;
                out.push((h1 * 16 + h2) as u8);
            }
            b'+' => out.push(b' '),
            _    => out.push(b),
        }
    }
    String::from_utf8(out).ok()
}

fn mime_type(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()) {
        Some("html") | Some("htm") => "text/html; charset=utf-8",
        Some("css")                => "text/css",
        Some("js")                 => "application/javascript",
        Some("json")               => "application/json",
        Some("png")                => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif")                => "image/gif",
        Some("svg")                => "image/svg+xml",
        Some("pdf")                => "application/pdf",
        Some("txt") | Some("md")   => "text/plain; charset=utf-8",
        Some("woff")               => "font/woff",
        Some("woff2")              => "font/woff2",
        _                          => "application/octet-stream",
    }
}

/// Map a request path to a file under `root` (which must be canonical).
/// Refuses `..`, hidden (dot-prefixed) components, and anything whose
/// canonical location — after resolving symlinks — lies outside `root`.
fn resolve_request(root: &Path, raw_path: &str) -> std::result::Result<PathBuf, u16> {
    use std::path::Component;
    let decoded = url_decode(raw_path).ok_or(400u16)?;
    if decoded.contains('\0') { return Err(400); }

    let mut file_path = root.to_path_buf();
    for component in Path::new(decoded.trim_start_matches('/')).components() {
        match component {
            Component::Normal(c) => {
                if c.to_string_lossy().starts_with('.') { return Err(404); }
                file_path.push(c);
            }
            Component::CurDir | Component::RootDir => {}
            Component::ParentDir | Component::Prefix(_) => return Err(400),
        }
    }

    if file_path.is_dir() { file_path.push("index.html"); }
    if !file_path.exists() {
        let with_html = file_path.with_extension("html");
        if with_html.exists() { file_path = with_html; }
    }

    let canonical = file_path.canonicalize().map_err(|_| 404u16)?;
    if !canonical.starts_with(root) || !canonical.is_file() { return Err(404); }
    Ok(canonical)
}

fn status_text(code: u16) -> &'static str {
    match code {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        405 => "Method Not Allowed",
        431 => "Request Header Fields Too Large",
        500 => "Internal Server Error",
        _   => "Error",
    }
}

fn write_head(stream: &mut TcpStream, code: u16, content_type: &str, len: u64) -> io::Result<()> {
    let extra = if code == 405 { "Allow: GET, HEAD\r\n" } else { "" };
    write!(
        stream,
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\n\
         X-Content-Type-Options: nosniff\r\n{}Connection: close\r\n\r\n",
        code, status_text(code), content_type, len, extra
    )
}

fn serve_connection(mut stream: TcpStream, root: &Path) {
    let _ = stream.set_read_timeout(Some(IO_TIMEOUT));
    let _ = stream.set_write_timeout(Some(IO_TIMEOUT));

    // Cap total header bytes so a never-ending line can't exhaust memory.
    let mut reader = BufReader::new((&stream).take(MAX_HEADER_BYTES));

    let mut request_line = String::new();
    if reader.read_line(&mut request_line).is_err() { return; }

    let mut headers_done = false;
    loop {
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) | Err(_)                          => break,
            Ok(_) if line == "\r\n" || line == "\n" => { headers_done = true; break; }
            _ => {}
        }
    }
    if !headers_done || !request_line.ends_with('\n') {
        let _ = write_head(&mut stream, 431, "text/plain", 0);
        return;
    }

    // "GET /path HTTP/1.x"
    let mut parts = request_line.split_whitespace();
    let method   = parts.next().unwrap_or("");
    let raw_path = parts.next().unwrap_or("/");
    let raw_path = raw_path.split(['?', '#']).next().unwrap_or("/");

    if method != "GET" && method != "HEAD" {
        let _ = write_head(&mut stream, 405, "text/plain", 0);
        return;
    }

    let path = match resolve_request(root, raw_path) {
        Ok(p)     => p,
        Err(code) => {
            let msg = status_text(code);
            let _ = write_head(&mut stream, code, "text/plain", msg.len() as u64);
            if method == "GET" { let _ = stream.write_all(msg.as_bytes()); }
            return;
        }
    };

    let file = match fs::File::open(&path) {
        Ok(f)  => f,
        Err(_) => { let _ = write_head(&mut stream, 500, "text/plain", 0); return; }
    };
    let len = file.metadata().map(|m| m.len()).unwrap_or(0);
    if write_head(&mut stream, 200, mime_type(&path), len).is_err() { return; }
    if method == "GET" {
        // Stream rather than loading the whole file into memory.
        let _ = io::copy(&mut file.take(len), &mut stream);
    }
}

/// Decrements the live-connection counter when a handler thread finishes.
struct ConnGuard(Arc<AtomicUsize>);
impl Drop for ConnGuard {
    fn drop(&mut self) { self.0.fetch_sub(1, Ordering::SeqCst); }
}

fn serve(port: u16, public: bool) -> Result<()> {
    let root = env::current_dir()?.canonicalize()?;
    let host = if public { "0.0.0.0" } else { "127.0.0.1" };
    let listener = TcpListener::bind((host, port))
        .map_err(|e| format!("Cannot bind to {}:{}: {}", host, port, e))?;
    println!("Serving {} on http://localhost:{} — Ctrl-C to stop", root.display(), port);
    if public {
        eprintln!("warning: --public exposes this folder to every host that can reach this machine");
    }

    let root = Arc::new(root);
    let live = Arc::new(AtomicUsize::new(0));
    for stream in listener.incoming().flatten() {
        // Over the cap: drop the connection instead of spawning another thread.
        if live.fetch_add(1, Ordering::SeqCst) >= MAX_CONNECTIONS {
            live.fetch_sub(1, Ordering::SeqCst);
            continue;
        }
        let guard = ConnGuard(Arc::clone(&live));
        let root  = Arc::clone(&root);
        std::thread::spawn(move || {
            let _guard = guard;
            serve_connection(stream, &root);
        });
    }
    Ok(())
}


fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.iter().any(|a| matches!(a.as_str(), "help" | "--help" | "-h")) {
        print!("{}", HELP);
        return;
    }
    let mut port = 8000u16;
    let mut public = false;
    for a in &args {
        if a == "--public" {
            public = true;
        } else if let Ok(p) = a.parse::<u16>() {
            port = p;
        } else {
            eprintln!("srv: unknown argument '{}'\n\n{}", a, HELP);
            exit(1);
        }
    }
    if let Err(e) = serve(port, public) {
        eprintln!("error: {}", e);
        exit(1);
    }
}
