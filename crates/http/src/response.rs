use httparse::{Response, EMPTY_HEADER};

pub struct ParsedResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub keep_alive: bool,
    pub content_length: Option<usize>,
    pub header_len: usize,
}

pub fn parse_response(buf: &[u8]) -> Option<ParsedResponse> {
    let mut headers = [EMPTY_HEADER; 64];
    let mut resp = Response::new(&mut headers);
    match resp.parse(buf) {
        Ok(httparse::Status::Complete(header_len)) => {
            let status = resp.code.unwrap_or(0);
            let mut content_length = None;
            let mut keep_alive = true;
            let mut parsed_headers = Vec::new();
            for h in resp.headers.iter() {
                let name_lower = h.name.to_lowercase();
                let value = std::str::from_utf8(h.value).unwrap_or("").to_string();
                if name_lower == "content-length" {
                    content_length = value.parse().ok();
                }
                if name_lower == "connection" && value.to_lowercase() == "close" {
                    keep_alive = false;
                }
                parsed_headers.push((h.name.to_string(), value));
            }
            Some(ParsedResponse {
                status,
                headers: parsed_headers,
                keep_alive,
                content_length,
                header_len,
            })
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_200_response() {
        let raw = b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\n\r\nhello";
        let r = parse_response(raw).unwrap();
        assert_eq!(r.status, 200);
        assert_eq!(r.content_length, Some(5));
        assert!(r.keep_alive);
    }

    #[test]
    fn detects_connection_close() {
        let raw = b"HTTP/1.1 200 OK\r\nConnection: close\r\nContent-Length: 0\r\n\r\n";
        let r = parse_response(raw).unwrap();
        assert!(!r.keep_alive);
    }

    #[test]
    fn returns_none_for_incomplete_response() {
        let raw = b"HTTP/1.1 200 OK\r\n";
        assert!(parse_response(raw).is_none());
    }
}
