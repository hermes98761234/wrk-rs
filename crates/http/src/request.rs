use std::collections::HashMap;

pub struct RequestBuilder {
    pub method: String,
    pub path: String,
    pub host: String,
    pub headers: HashMap<String, String>,
    pub body: Option<Vec<u8>>,
}

impl RequestBuilder {
    pub fn new(
        method: impl Into<String>,
        path: impl Into<String>,
        host: impl Into<String>,
    ) -> Self {
        Self {
            method: method.into(),
            path: path.into(),
            host: host.into(),
            headers: HashMap::new(),
            body: None,
        }
    }

    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(name.into(), value.into());
        self
    }

    pub fn body(mut self, body: Vec<u8>) -> Self {
        self.body = Some(body);
        self
    }

    pub fn build(&self) -> Vec<u8> {
        let mut req = format!(
            "{} {} HTTP/1.1\r\nHost: {}\r\n",
            self.method, self.path, self.host
        );
        for (k, v) in &self.headers {
            req.push_str(&format!("{}: {}\r\n", k, v));
        }
        if let Some(body) = &self.body {
            req.push_str(&format!("Content-Length: {}\r\n", body.len()));
        }
        req.push_str("\r\n");
        let mut bytes = req.into_bytes();
        if let Some(body) = &self.body {
            bytes.extend_from_slice(body);
        }
        bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_minimal_get_request() {
        let req = RequestBuilder::new("GET", "/", "localhost").build();
        let s = String::from_utf8(req).unwrap();
        assert!(s.starts_with("GET / HTTP/1.1\r\n"));
        assert!(s.contains("Host: localhost\r\n"));
        assert!(s.ends_with("\r\n\r\n"));
    }

    #[test]
    fn includes_custom_headers() {
        let req = RequestBuilder::new("GET", "/api", "example.com")
            .header("Accept", "application/json")
            .build();
        let s = String::from_utf8(req).unwrap();
        assert!(s.contains("Accept: application/json\r\n"));
    }

    #[test]
    fn includes_body_with_content_length() {
        let req = RequestBuilder::new("POST", "/data", "example.com")
            .body(b"hello".to_vec())
            .build();
        let s = String::from_utf8(req).unwrap();
        assert!(s.contains("Content-Length: 5\r\n"));
        assert!(s.ends_with("hello"));
    }
}
