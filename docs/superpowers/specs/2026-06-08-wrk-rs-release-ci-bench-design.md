# wrk-rs: Release, CI, Benchmarks & Repo Polish

**Date:** 2026-06-08
**Project:** `/home/user/projects/wrk-rs`
**Repo:** `hermes98761234/wrk-rs`

## Problem

The existing release workflow only builds Linux binaries. No macOS or Windows targets exist. No performance benchmarks exist. The GitHub repo lacks topic tags.

## Goals

1. Release workflow produces binaries for Linux, macOS, and Windows
2. Criterion integration benchmark measures real req/s throughput against a live HTTP server
3. GitHub repo has polished description and topic tags
4. A `v0.1.0` release tag is pushed, triggering the full cross-platform release

## Task Breakdown

### T1 — Fix release.yml: add macOS and Windows targets

**File:** `.github/workflows/release.yml`

Add three new matrix entries to the existing build matrix:

```yaml
- os: macos-latest
  target: x86_64-apple-darwin
  use_cross: false
- os: macos-latest
  target: aarch64-apple-darwin
  use_cross: false
- os: windows-latest
  target: x86_64-pc-windows-msvc
  use_cross: false
  bin_ext: .exe
```

Platform notes:
- macOS aarch64 cross-compiles natively via Apple SDK (no `cross` tool needed, just add the target)
- Windows binary is `wrk.exe`; skip the `strip` step on Windows
- The existing `|| true` on strip handles macOS gracefully

The `Rename binary` step must append `${{ matrix.bin_ext }}` to handle `.exe` on Windows.

Commit message: `ci: add macOS and Windows release targets`

### T2 — Criterion integration benchmark

**New workspace member:** `crates/bench`

Structure:
```
crates/bench/
  Cargo.toml
  benches/
    throughput.rs
```

`Cargo.toml`:
- `[package]` name = `bench`, publish = false
- `[dev-dependencies]`: criterion (features = ["html_reports"]), hyper (features = ["server", "http1"]), tokio (features = ["full"]), engine (path = "../engine"), stats (path = "../stats")
- `[[bench]]` name = "throughput", harness = false

`benches/throughput.rs`:
1. Start a `hyper` HTTP/1.1 server on `127.0.0.1:0` (random port) in a background tokio task, handler returns `200 OK` with `"hello"` body
2. Build engine `Config` with 2 threads, 10 connections, 5s duration, no script, targeting the server's bound address
3. Run `engine::run(config)` inside `criterion::Criterion::bench_function`
4. Extract `req/s` and `p99_latency_ms` from the returned `AggregateStats` and report via `criterion::black_box`
5. `sample_size(10)`, `measurement_time(Duration::from_secs(8))`

Add `crates/bench` to workspace `members` in root `Cargo.toml`.

Not included in CI (excluded via `--workspace --exclude bench` in ci.yml if needed). Run manually with `cargo bench -p bench`.

Commit message: `bench: add criterion integration benchmark (hyper server + engine)`

### T3 — GitHub repo decoration

Commands:
```bash
gh repo edit hermes98761234/wrk-rs \
  --description "⚡ Modern HTTP benchmarking tool — Rust rewrite of wrk"

gh repo edit hermes98761234/wrk-rs \
  --add-topic rust \
  --add-topic http \
  --add-topic benchmarking \
  --add-topic load-testing \
  --add-topic performance \
  --add-topic cli \
  --add-topic wrk \
  --add-topic tokio \
  --add-topic lua
```

No git commit needed — GitHub API only.

### T4 — Push v0.1.0 release tag

Depends on T1 and T2 being committed and pushed to `main`.

```bash
git tag v0.1.0 -m "Release v0.1.0"
git push origin v0.1.0
```

This triggers the fixed `release.yml` workflow, which builds binaries for all 5 targets and creates the GitHub release with checksums.

## Success Criteria

- GitHub Actions release workflow runs for all 5 targets on `v0.1.0` push
- `cargo bench -p bench` runs without errors and prints req/s output
- `gh repo view hermes98761234/wrk-rs` shows updated description and topics
- GitHub release page for `v0.1.0` contains 5 binary artifacts + SHA256SUMS.txt
