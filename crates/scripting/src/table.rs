use mlua::{Lua, Result, Table};

pub fn setup_wrk_table(lua: &Lua, scheme: &str, host: &str, port: u16) -> Result<()> {
    let wrk: Table = lua.create_table()?;
    wrk.set("scheme", scheme)?;
    wrk.set("host", host)?;
    wrk.set("port", port)?;
    wrk.set("method", "GET")?;
    wrk.set("path", "/")?;
    wrk.set("headers", lua.create_table()?)?;
    wrk.set("body", "")?;

    let host_s = host.to_string();
    let format_fn = lua.create_function(
        move |_lua, (method, path, headers, body): (String, String, Option<Table>, Option<String>)| {
            let mut req = format!("{} {} HTTP/1.1\r\nHost: {}\r\n", method, path, host_s);
            if let Some(h) = headers {
                for pair in h.pairs::<String, String>() {
                    let (k, v) = pair?;
                    req.push_str(&format!("{}: {}\r\n", k, v));
                }
            }
            if let Some(ref b) = body {
                req.push_str(&format!("Content-Length: {}\r\n", b.len()));
            }
            req.push_str("\r\n");
            if let Some(b) = body {
                req.push_str(&b);
            }
            Ok(req)
        },
    )?;
    wrk.set("format", format_fn)?;

    lua.globals().set("wrk", wrk)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sets_wrk_globals() {
        let lua = Lua::new();
        setup_wrk_table(&lua, "http", "localhost", 8080).unwrap();
        let wrk: Table = lua.globals().get("wrk").unwrap();
        assert_eq!(wrk.get::<String>("scheme").unwrap(), "http");
        assert_eq!(wrk.get::<String>("host").unwrap(), "localhost");
        assert_eq!(wrk.get::<u16>("port").unwrap(), 8080);
        assert_eq!(wrk.get::<String>("method").unwrap(), "GET");
        assert_eq!(wrk.get::<String>("path").unwrap(), "/");
    }

    #[test]
    fn format_builds_request_string() {
        let lua = Lua::new();
        setup_wrk_table(&lua, "http", "example.com", 80).unwrap();
        let result: String = lua
            .load(r#"wrk.format("POST", "/api", nil, "data")"#)
            .eval()
            .unwrap();
        assert!(result.contains("POST /api HTTP/1.1"));
        assert!(result.contains("Host: example.com"));
        assert!(result.contains("Content-Length: 4"));
        assert!(result.ends_with("data"));
    }
}
