# Installing VeloBenchmark

VeloBenchmark is a **single-binary** app you run on your own machine, then use entirely from a
browser. Pick one of two paths below.

- **One-line installer** — downloads a prebuilt binary when one is published for your platform,
  otherwise builds from source (installing Rust/Node/protoc for you). Fastest for most people.
- **Build from source** — if you want to build the binary yourself (or fix/patch it).

Both end with the server running and a URL to open, e.g. `http://localhost:13843`.

---

## Option 1 — one-line installer

Linux or macOS, Intel or Apple Silicon/ARM:

```bash
curl -fsSL https://raw.githubusercontent.com/sf-stav/VeloBench/main/install.sh | sh
```

- Downloads the prebuilt binary for your platform **if one is published on GitHub Releases** (see
  the repo's **Releases** page); otherwise **builds from source into `~/.velobenchmark`** (installing
  Rust / Node / `protoc` into your home directory — no root required, except for an optional system
  package-manager assist).
- Starts the server, **health-checks** it, and prints the URL to open.

Stop it later with:

```bash
kill $(cat ~/.velobenchmark/velobench.pid)
```

**Environment overrides** (set before running the installer):

| Variable | Default | Meaning |
|---|---|---|
| `VELOBENCHMARK_REPO` | `sf-stav/VeloBench` | GitHub slug to fetch the release from |
| `VELOBENCHMARK_PORT` | `13843` | Server port |
| `VELOBENCHMARK_DIR` | `~/.velobenchmark` | Install / data root |
| `VELO_SRC` | — | Build an existing source checkout instead of cloning |

---

## Option 2 — build from source

### Prerequisites

- **Rust** toolchain (`cargo build --release`)
- **Node.js** 20+ (to build the embedded Angular frontend)
- **`protoc`** (Protocol Buffers compiler; `prost-build` needs it)

### Build

```bash
# install frontend deps
npm --prefix frontend install

# build the frontend and copy it into assets/ (embeds the UI into the binary)
bash scripts/build-frontend.sh

# build the Rust server
cargo build --release --bin velobench
```

### Run

```bash
./target/release/velobench --host 0.0.0.0 --port 13843
```

Open the printed URL in your browser — e.g. **http://localhost:13843**.

| Flag | Default | Meaning |
|---|---|---|
| `--host` | `0.0.0.0` | Bind address |
| `--port` | `13843` | Bind port |
| `--data-dir` | `velobench_data` | Where settings, sessions and tests are persisted |

---

## First run

1. Open the server URL in a browser.
2. Go to **Settings → add a provider**: an OpenAI-compatible base URL + API key (and pick a model).
   Keys live **server-side** in `velobench_data/settings.json`, never in the browser.
3. **Chat** with the model and watch the live instruments (tok/s, TTFT, per-regime decode graph).
4. **Run a built-in test** (or build your own) and read the session report.

The [user manual](docs/user-manual.md) walks every page in detail.

---

## Notes

- **One binary, browser UI.** After launch you don't touch the terminal — everything is in the
  browser.
- **No auth.** VeloBenchmark is meant for a trusted network; it has no authentication of its own.
- **Data dir.** Settings, sessions, and tests persist under `--data-dir` (`velobench_data/` by
  default).
