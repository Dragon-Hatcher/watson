use crate::util::ansi::{ANSI_BOLD, ANSI_GRAY, ANSI_GREEN, ANSI_RESET};
use std::fs;
use std::path::Path;
use tiny_http::{Header, Response, Server};

pub fn serve(book_dir: &Path, port: u16) {
    let server = Server::http(("127.0.0.1", port)).expect("Failed to start server");
    let book_dir = book_dir.to_path_buf();

    println!("{ANSI_BOLD}{ANSI_GREEN}Serving book{ANSI_RESET} at http://localhost:{port}");
    println!("{ANSI_GRAY}Press Ctrl+C to stop the server{ANSI_RESET}");

    for request in server.incoming_requests() {
        let url_path = request.url().trim_start_matches('/');

        // Build the file path
        let mut file_path = book_dir.join(url_path);

        // If the path is a directory, append index.html
        if file_path.is_dir() {
            file_path = file_path.join("index.html");
        }

        // Try to read and serve the file
        match fs::read(&file_path) {
            Ok(content) => {
                let content_type = get_content_type(&file_path);
                let content_type_header =
                    Header::from_bytes(&b"Content-Type"[..], content_type.as_bytes())
                        .expect("Failed to create header");

                let mut response = Response::from_data(content).with_header(content_type_header);

                // Add Last-Modified header for auto-reload support
                if let Ok(metadata) = fs::metadata(&file_path)
                    && let Ok(modified) = metadata.modified()
                {
                    // Format as HTTP date (RFC 7231)
                    let http_date = httpdate::fmt_http_date(modified);
                    if let Ok(last_modified_header) =
                        Header::from_bytes(&b"Last-Modified"[..], http_date.as_bytes())
                    {
                        response.add_header(last_modified_header);
                    }
                }

                if let Err(e) = request.respond(response) {
                    eprintln!("Failed to send response: {}", e);
                }
            }
            Err(_) => {
                // File not found
                let response = Response::from_string("404 Not Found").with_status_code(404);

                if let Err(e) = request.respond(response) {
                    eprintln!("Failed to send 404 response: {}", e);
                }
            }
        }
    }
}

fn get_content_type(path: &Path) -> String {
    match path.extension().and_then(|s| s.to_str()) {
        Some("html") => "text/html; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("js") => "application/javascript; charset=utf-8",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("svg") => "image/svg+xml",
        Some("woff") => "font/woff",
        Some("woff2") => "font/woff2",
        _ => "text/plain; charset=utf-8",
    }
    .to_string()
}
