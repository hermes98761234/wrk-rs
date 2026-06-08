use mlua::{Lua, Result};

pub fn call_setup(lua: &Lua, thread_id: u32) -> Result<()> {
    if let Ok(f) = lua.globals().get::<mlua::Function>("setup") {
        let t = lua.create_table()?;
        t.set("id", thread_id)?;
        f.call::<()>(t)?;
    }
    Ok(())
}

pub fn call_init(lua: &Lua, args: &[String]) -> Result<()> {
    if let Ok(f) = lua.globals().get::<mlua::Function>("init") {
        let t = lua.create_table()?;
        for (i, arg) in args.iter().enumerate() {
            t.set(i + 1, arg.as_str())?;
        }
        f.call::<()>(t)?;
    }
    Ok(())
}

pub fn call_request(lua: &Lua, default_request: &[u8]) -> Result<Vec<u8>> {
    if let Ok(f) = lua.globals().get::<mlua::Function>("request") {
        let result: String = f.call(())?;
        Ok(result.into_bytes())
    } else {
        Ok(default_request.to_vec())
    }
}

pub fn call_response(lua: &Lua, status: u16, headers: &[(String, String)], body: &[u8]) -> Result<()> {
    if let Ok(f) = lua.globals().get::<mlua::Function>("response") {
        let h = lua.create_table()?;
        for (k, v) in headers {
            h.set(k.as_str(), v.as_str())?;
        }
        let body_str = String::from_utf8_lossy(body).to_string();
        f.call::<()>((status, h, body_str))?;
    }
    Ok(())
}

pub fn call_delay(lua: &Lua) -> Result<u64> {
    if let Ok(f) = lua.globals().get::<mlua::Function>("delay") {
        let ms: u64 = f.call(())?;
        Ok(ms)
    } else {
        Ok(0)
    }
}

pub fn call_done(lua: &Lua, requests: u64, duration_us: u64, bytes: u64) -> Result<()> {
    if let Ok(f) = lua.globals().get::<mlua::Function>("done") {
        let summary = lua.create_table()?;
        summary.set("requests", requests)?;
        summary.set("duration", duration_us)?;
        summary.set("bytes", bytes)?;
        f.call::<()>((summary, lua.create_table()?, lua.create_table()?))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::table::setup_wrk_table;

    #[test]
    fn call_request_returns_default_when_no_hook() {
        let lua = Lua::new();
        setup_wrk_table(&lua, "http", "localhost", 80).unwrap();
        let default = b"GET / HTTP/1.1\r\n\r\n";
        let result = call_request(&lua, default).unwrap();
        assert_eq!(result, default);
    }

    #[test]
    fn call_request_uses_hook_when_defined() {
        let lua = Lua::new();
        setup_wrk_table(&lua, "http", "localhost", 80).unwrap();
        lua.load(r#"function request() return "GET /custom HTTP/1.1\r\n\r\n" end"#)
            .exec()
            .unwrap();
        let result = call_request(&lua, b"").unwrap();
        assert_eq!(result, b"GET /custom HTTP/1.1\r\n\r\n");
    }

    #[test]
    fn call_delay_returns_zero_without_hook() {
        let lua = Lua::new();
        let ms = call_delay(&lua).unwrap();
        assert_eq!(ms, 0);
    }

    #[test]
    fn call_delay_returns_hook_value() {
        let lua = Lua::new();
        lua.load("function delay() return 50 end").exec().unwrap();
        let ms = call_delay(&lua).unwrap();
        assert_eq!(ms, 50);
    }

    #[test]
    fn call_response_invokes_hook() {
        let lua = Lua::new();
        lua.load("_resp_status = 0; function response(s, h, b) _resp_status = s end")
            .exec()
            .unwrap();
        call_response(&lua, 200, &[], b"").unwrap();
        let status: u32 = lua.globals().get("_resp_status").unwrap();
        assert_eq!(status, 200);
    }
}
