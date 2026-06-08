# wrk-rs: Release, CI, Benchmarks & Repo Polish — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship a cross-platform (Linux/macOS/Windows) v0.1.0 release of wrk-rs, add a criterion integration benchmark, and polish the GitHub repo.

**Architecture:** Four independent tasks — fix the release workflow matrix, add a benchmark crate to the workspace, decorate the GitHub repo via CLI, then tag v0.1.0 to trigger the release. T1 and T2 modify the repo and must both land on `main` before T4 tags.

**Tech Stack:** Rust, GitHub Actions, criterion 0.5, raw TCP echo server (no extra deps), `gh` CLI

---

## File Map

| File | Action | Purpose |
|------|--------|---------|
| `.github/workflows/release.yml` | Modify | Add macOS x86_64, macOS aarch64, Windows x86_64 targets |
| `Cargo.toml` (workspace root) | Modify | Add `crates/bench` to `members` |
| `crates/bench/Cargo.toml` | Create | Bench crate manifest with criterion |
| `crates/bench/benches/throughput.rs` | Create | Integration benchmark: TCP server + engine |

---

## Task 1: Fix release.yml — add macOS and Windows targets

**Files:**
- Modify: `.github/workflows/release.yml`

**Context:** The current release workflow builds only 4 Linux targets. It is missing macOS and Windows. This task adds 3 new matrix entries and updates the Strip/Rename steps to handle platform differences. `macos-13` runs on Intel (native x86_64); `macos-latest` runs on ARM64 M1 (native aarch64). Windows binary has a `.exe` suffix.

- [ ] **Step 1: Add `bin_ext` field and 3 new matrix entries**

Open `.github/workflows/release.yml`. In the `strategy.matrix.include` list, after the last Linux entry, add:

```yaml
          - os: macos-13
            target: x86_64-apple-darwin
            use_cross: false
            bin_ext: ""
          - os: macos-latest
            target: aarch64-apple-darwin
            use_cross: false
            bin_ext: ""
          - os: windows-latest
            target: x86_64-pc-windows-msvc
            use_cross: false
            bin_ext: .exe
```

Also add `bin_ext: ""` to each of the four existing Linux matrix entries so the variable is always defined.

- [ ] **Step 2: Update the Strip step to skip on Windows**

Replace the existing Strip step:
```yaml
      - name: Strip binary
        run: strip target/${{ matrix.target }}/release/${{ env.BIN_NAME }} || true
```
with:
```yaml
      - name: Strip binary
        if: runner.os != 'Windows'
        run: strip target/${{ matrix.target }}/release/${{ env.BIN_NAME }}${{ matrix.bin_ext }} || true
```

- [ ] **Step 3: Update the Rename step to handle `.exe` and use bash shell**

Replace the existing Rename step:
```yaml
      - name: Rename binary
        run: |
          mv target/${{ matrix.target }}/release/${{ env.BIN_NAME }} \
             target/${{ matrix.target }}/release/${{ env.BIN_NAME }}-${{ matrix.target }}
```
with:
```yaml
      - name: Rename binary
        shell: bash
        run: |
          mv "target/${{ matrix.target }}/release/${{ env.BIN_NAME }}${{ matrix.bin_ext }}" \
             "target/${{ matrix.target }}/release/${{ env.BIN_NAME }}-${{ matrix.target }}${{ matrix.bin_ext }}"
```

- [ ] **Step 4: Update the Upload artifact step to include `.exe`**

Replace:
```yaml
      - name: Upload artifact
        uses: actions/upload-artifact@v4
        with:
          name: ${{ env.BIN_NAME }}-${{ matrix.target }}
          path: target/${{ matrix.target }}/release/${{ env.BIN_NAME }}-${{ matrix.target }}
```
with:
```yaml
      - name: Upload artifact
        uses: actions/upload-artifact@v4
        with:
          name: ${{ env.BIN_NAME }}-${{ matrix.target }}
          path: target/${{ matrix.target }}/release/${{ env.BIN_NAME }}-${{ matrix.target }}${{ matrix.bin_ext }}
```

- [ ] **Step 5: Commit and push**

```bash
git add .github/workflows/release.yml
git commit -m "ci: add macOS and Windows release targets"
git push origin main
```

Expected: commit lands on main, no local errors.

---

## Task 2: Add criterion integration benchmark

**Files:**
- Modify: `Cargo.toml` (workspace root)
- Create: `crates/bench/Cargo.toml`
- Create: `crates/bench/benches/throughput.rs`

**Context:** `wrk-rs` is at `/home/user/projects/wrk-rs`. Package names: engine = `wrk-engine`, stats = `wrk-stats`. `spawn_threads(config: BenchConfig) -> Vec<ThreadResult>` where `ThreadResult { stats: ThreadStats }`. `merge(Vec<ThreadStats>) -> AggregateStats` where `AggregateStats { requests, errors, bytes, duration_us, latency }`. The benchmark spins up a minimal raw TCP HTTP/1.1 keep-alive echo server (no extra deps), then drives the engine against it for 5s and reports req/s.

Work dir: `/home/user/projects/wrk-rs`

- [ ] **Step 1: Add `crates/bench` to workspace members**

In `Cargo.toml` (workspace root), in the `members` array, add `"crates/bench"`:

```toml
members = [
    "crates/stats",
    "crates/http",
    "crates/scripting",
    "crates/engine",
    "crates/cli",
    "crates/bench",
]
```

- [ ] **Step 2: Create `crates/bench/Cargo.toml`**

```toml
[package]
name = "bench"
version = "0.1.0"
edition = "2021"
publish = false

[[bench]]
name = "throughput"
harness = false

[dev-dependencies]
criterion = { version = "0.5", features = ["html_reports"] }
wrk-engine = { path = "../engine" }
wrk-stats = { path = "../stats" }
tokio = { version = "1", features = ["full"] }
```

- [ ] **Step 3: Create `crates/bench/benches/throughput.rs`**

```rust
use criterion::{criterion_group, criterion_main, Criterion};
use std::collections::HashMap;
use std::time::Duration;
use wrk_engine::{BenchConfig, spawn_threads};
use wrk_stats::aggregate::merge;

fn start_echo_server() -> u16 {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    std::thread::spawn(move || {
        use std::io::{Read, Write};
        let response =
            b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\nConnection: keep-alive\r\n\r\nhello";
        loop {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buf = [0u8; 4096];
                loop {
                    match stream.read(&mut buf) {
                        Ok(0) | Err(_) => break,
                        Ok(_) => {
                            if stream.write_all(response).is_err() {
                                break;
                            }
                        }
                    }
                }
            }
        }
    });
    std::thread::sleep(Duration::from_millis(50));
    port
}

fn bench_throughput(c: &mut Criterion) {
    let port = start_echo_server();

    let mut group = c.benchmark_group("engine");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(8));

    group.bench_function("2t_10c_5s", |b| {
        b.iter(|| {
            let config = BenchConfig {
                threads: 2,
                connections: 10,
                duration: Duration::from_secs(5),
                timeout: Duration::from_secs(5),
                url: format!("http://127.0.0.1:{}/", port),
                scheme: "http".to_string(),
                host: "127.0.0.1".to_string(),
                port,
                path: "/".to_string(),
                method: "GET".to_string(),
                headers: HashMap::new(),
                body: None,
                script_source: None,
                script_args: vec![],
                print_latency: false,
            };
            let results = spawn_threads(config);
            let thread_stats: Vec<_> = results.into_iter().map(|r| r.stats).collect();
            let agg = merge(thread_stats);
            let req_per_sec =
                agg.requests as f64 / (agg.duration_us as f64 / 1_000_000.0);
            criterion::black_box(req_per_sec)
        });
    });

    group.finish();
}

criterion_group!(benches, bench_throughput);
criterion_main!(benches);
```

- [ ] **Step 4: Verify it compiles**

```bash
cargo build -p bench
```

Expected: compiles without errors. (Running the full bench takes ~90s; skip during this step.)

- [ ] **Step 5: Commit and push**

```bash
git add Cargo.toml Cargo.lock crates/bench/
git commit -m "bench: add criterion integration benchmark (TCP echo server + engine)"
git push origin main
```

Expected: commit lands on main.

---

## Task 3: Decorate GitHub repo

**Context:** Repo is `hermes98761234/wrk-rs`. Use `gh` CLI. No git commits needed — this is GitHub metadata only.

Work dir: `/home/user/projects/wrk-rs`

- [ ] **Step 1: Update description**

```bash
gh repo edit hermes98761234/wrk-rs \
  --description "⚡ Modern HTTP benchmarking tool — Rust rewrite of wrk"
```

Expected: exits 0, no error output.

- [ ] **Step 2: Add topics**

```bash
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

Expected: exits 0.

- [ ] **Step 3: Verify**

```bash
gh repo view hermes98761234/wrk-rs --json description,repositoryTopics \
  | python3 -c "import json,sys; d=json.load(sys.stdin); print(d['description']); print([t['name'] for t in d['repositoryTopics']])"
```

Expected: prints the description and a list containing all 9 topics.

---

## Task 4: Push v0.1.0 release tag

**Context:** This task depends on T1 and T2 being committed and pushed to `main`. Pushing this tag triggers the fixed `release.yml` workflow, which builds 7 binary artifacts (4 Linux + 2 macOS + 1 Windows) and creates the GitHub Release.

Work dir: `/home/user/projects/wrk-rs`

- [ ] **Step 1: Confirm T1 and T2 are on main**

```bash
git log --oneline origin/main | head -5
```

Expected: the two commits from T1 and T2 (`ci: add macOS...` and `bench: add criterion...`) appear at the top.

- [ ] **Step 2: Pull latest main**

```bash
git pull origin main
```

Expected: already up to date or fast-forwards to include T1+T2 commits.

- [ ] **Step 3: Create and push the tag**

```bash
git tag v0.1.0 -m "Release v0.1.0"
git push origin v0.1.0
```

Expected: tag is pushed, GitHub Actions workflow `Release` is triggered.

- [ ] **Step 4: Verify the release workflow started**

```bash
gh run list --repo hermes98761234/wrk-rs --workflow release.yml --limit 3
```

Expected: a run appears with status `in_progress` or `queued` for the `v0.1.0` tag.

- [ ] **Step 5: Report the release URL**

```bash
gh release view v0.1.0 --repo hermes98761234/wrk-rs --web 2>/dev/null || \
  echo "Release will appear at: https://github.com/hermes98761234/wrk-rs/releases/tag/v0.1.0"
```
