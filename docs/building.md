# Building & installing

## One-line install (Linux & macOS, x86_64 + arm64)

```bash
curl -fsSL https://raw.githubusercontent.com/sf-stav/VeloBench/main/install.sh | sh
```

What it does:

1. Detects your OS/architecture.
2. **Fast path** — downloads a prebuilt binary from this repository's GitHub
   Releases (`velobenchmark-<os>-<arch>.tar.gz`) when one exists for your
   platform.
3. **Fallback** — clones the repo into `~/.velobenchmark/src` and builds from
   source, installing anything missing into your home directory (Rust via
   rustup, Node via the official tarball, protoc via the system package
   manager or a userland copy) — no root required except an optional
   passwordless `sudo` assist for `protobuf-compiler`.
4. Starts the server (default port 13843, moves to the next free port if
   busy), health-checks it, and prints the URL to open — local and LAN.

Environment overrides: `VELOBENCHMARK_PORT`, `VELOBENCHMARK_DIR` (default
`~/.velobenchmark`), `VELO_SRC` (build an existing checkout), and
`VELOBENCHMARK_REPO` (alternative `owner/repo`). Stop the server with
`kill $(cat ~/.velobenchmark/velobench.pid)`; the data directory it uses is
`~/.velobenchmark/velobench_data`.

## Publishing prebuilt binaries

> A ready-to-paste release-session brief lives in
> [RELEASING.md](RELEASING.md).

Release artifacts are plain tarballs named
`velobenchmark-<os>-<arch>.tar.gz` (containing the `velobench` binary) —
the installer looks for exactly that name on the latest release. Three ways
to produce them:

- **Automatically** — push a tag (`git tag v0.1.0 && git push --tags`); the
  included GitHub Actions workflow (`.github/workflows/release.yml`) builds
  all four platforms and attaches the artifacts to the release.
- **Linux builds on an aarch64 Linux box** (e.g. a ARM server/Pi) — both
  Linux targets cross-compile with zig as the C compiler, fully static
  (no glibc dependency, runs on any distribution):

  ```bash
  pip install --user ziglang
  printf '#!/bin/sh\nexec python3 -m ziglang "$@"\n' > ~/.local/bin/zig && chmod +x ~/.local/bin/zig
  cargo install cargo-zigbuild
  rustup target add x86_64-unknown-linux-musl aarch64-unknown-linux-musl

  bash scripts/dist.sh x86_64-unknown-linux-musl    # -> dist/velobenchmark-linux-x86_64.tar.gz
  bash scripts/dist.sh aarch64-unknown-linux-musl   # -> dist/velobenchmark-linux-arm64.tar.gz
  ```

  (`scripts/dist.sh <triple>` also accepts the GNU triples when the matching
  `gcc-<arch>-linux-gnu` cross compiler is installed; on an x86_64 host the
  native build covers linux-x86_64 directly.)
- **Manually, per machine** — run `bash scripts/dist.sh` on each machine and
  upload everything in `dist/` to the release.

Then attach all of `dist/*` to a GitHub release:

```bash
gh release create v<version> dist/velobenchmark-*.tar.gz dist/*.sha256 \
  --title "v<version>" --notes "..."
```

macOS artifacts require building on macOS (Apple SDK licensing) — use the
CI workflow or a Mac; the installer falls back to a source build when a
platform's artifact is missing.

---

## Building from source

The deliverable is **one self-contained binary**. It embeds the entire
frontend and serves it; at runtime it only creates the data files it needs
(`velobench_data/` next to the working directory) — nothing else is required
on the host.

## Prerequisites

| Tool | Used for |
|------|----------|
| **Rust** (stable, recent) | the server — `rustup` install is fine |
| **Node.js** ≥ 20 + npm | the Angular frontend |
| **protoc** (protobuf compiler ≥ 3.x) | code generation of the wire protocol (`prost-build` shells out to it) |

Debian/Ubuntu: `sudo apt install protobuf-compiler` (or grab a release binary
of `protoc`). Check with `protoc --version`.

> `protoc` is only needed because the build compiles `proto/velobench.proto`
> from source. The generated frontend types are refreshed by the build script
> itself (`pbjs`/`pbts` ship with the frontend dev-dependencies).

## Build

From the repository root:

```bash
# 1. frontend → embedded assets/
npm --prefix frontend install        # first time only
bash scripts/build-frontend.sh

# 2. the binary (frontend is embedded at compile time)
cargo build --release
```

The result is `target/release/velobench`.

Notes:

- **Order matters**: the frontend is embedded via `include_dir!` when cargo
  runs, so always run `build-frontend.sh` before `cargo build` if the
  frontend changed.
- Anything under `assets/` is embedded too — including `assets/test_images/`
  (the vision tests' image library). **New images require a rebuild** to show
  up.
- A user-provided `assets/favicon.ico` is preserved across frontend builds.

## Run

```bash
./target/release/velobench --host 0.0.0.0 --port 13843
```

- Open `http://localhost:13843` (or `http://<host>:13843` from the network).
- On first boot the server creates `velobench_data/` in the **current working
  directory** — settings, sessions, tests, comparisons. Run it from the
  directory where you want that data to live (or point your service manager
  at a dedicated working directory).
- Built-in tests are (re-)seeded at every start; your own tests, favourites
  and recorded sessions are kept.

### Running as a service (systemd example)

```ini
[Unit]
Description=VeloBenchmarkmark
After=network.target

[Service]
WorkingDirectory=/opt/velobenchmark
ExecStart=/opt/velobenchmark/velobench --host 0.0.0.0 --port 13843
Restart=on-failure

[Install]
WantedBy=multi-user.target
```

## Development workflow

```bash
cd frontend && npm start        # Angular dev server with live reload
cargo run --release             # backend; proxy/dev server config in frontend/
```

The frontend talks to the backend over same-origin APIs plus one WebSocket;
when using the Angular dev server, proxy the API/WebSocket to the backend
port as usual.

## Regenerating the wire protocol

`proto/velobench.proto` is the single source of truth (Rust side via
`build.rs`/`prost`, browser side via protobufjs). `scripts/build-frontend.sh`
regenerates the browser types automatically; the Rust side regenerates on
`cargo build`.
