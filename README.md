# wrk-rs

A Rust rewrite of [wrk](https://github.com/wg/wrk) — a modern HTTP benchmarking tool.

## Features

- Drop-in CLI replacement for wrk (`-t`, `-c`, `-d`, `-s`, `--latency`, `--timeout`, `-H`, `-v`, long forms, SI units)
- Full Lua scripting API parity (`request`, `response`, `delay`, `done`, `setup`, `init` hooks)
- Multi-threaded: one Tokio runtime per thread, isolated Lua state per thread
- HTTP/1.1 keep-alive connection reuse
- TLS via rustls (pure Rust, no OpenSSL dependency)
- HDR histogram latency tracking
- Socket error (connect/read/write/timeout) and non-2xx/3xx response reporting
- Identical output format to wrk

## Install

```bash
cargo install --git https://github.com/hermes98761234/wrk-rs
```

Or download a pre-built binary from [Releases](https://github.com/hermes98761234/wrk-rs/releases).

## Usage

```
wrk <options> <url>

Options:
  -t, --threads     <N>  Number of threads to use [default: 2]
  -c, --connections <N>  Connections to keep open [default: 10]
  -d, --duration    <T>  Duration of test, e.g. 30s, 1m, 2h [default: 10s]
  -s, --script      <S>  Load Lua script file
  -H, --header      <H>  Add header to request (repeatable)
      --latency          Print latency statistics
      --timeout     <T>  Socket/request timeout [default: 2s]
  -v, --version          Print version details

  Numeric arguments may include an SI unit (1k, 1M, 1G)
  Time arguments may include a time unit (ms, s, m, h)
```

## Examples

```bash
# 30s benchmark with 12 threads and 400 connections
wrk -t12 -c400 -d30s http://localhost:8080/

# With latency distribution
wrk -t4 -c100 -d10s --latency https://example.com/

# With Lua script
wrk -t4 -c100 -d10s -s post.lua http://localhost:8080/api
```

## Lua Scripting

```lua
-- post.lua
wrk.method = "POST"
wrk.body   = '{"key":"value"}'
wrk.headers["Content-Type"] = "application/json"

function response(status, headers, body)
  if status ~= 200 then
    print("Error: " .. status)
  end
end
```

## Architecture

N independent OS threads, each with a `tokio::current_thread` runtime and an isolated mlua Lua state. Stats are merged via HDR histogram after all threads complete.

## HTTP/2

HTTP/2 support is tracked as a future milestone.
