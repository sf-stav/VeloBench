#!/bin/sh
# ============================================================================
#  VeloBenchmark — one-line installer
#
#    curl -fsSL https://raw.githubusercontent.com/<USER>/velobenchmark/main/install.sh | sh
#
#  Works on Linux and macOS, x86_64 and arm64. Two paths:
#    1. FAST:  download a prebuilt binary from GitHub Releases (if one exists
#              for your platform)
#    2. SLOW:  clone + build from source (installs Rust/Node/protoc into your
#              home directory if they are missing — no root required except
#              for an optional system package-manager assist)
#
#  Environment overrides:
#    VELOBENCHMARK_REPO   GitHub slug           (default: see GITHUB_REPO below)
#    VELOBENCHMARK_PORT   server port           (default: 13843)
#    VELOBENCHMARK_DIR    install/data root     (default: ~/.velobenchmark)
#    VELO_SRC             build an existing source checkout instead of cloning
#
#  The script is POSIX sh: it works with `curl ... | sh` (dash, bash, zsh).
# ============================================================================
set -eu

# ── Set this once before publishing ─────────────────────────────────────────
GITHUB_REPO="${VELOBENCHMARK_REPO:-sf-stav/VeloBench}"
# ────────────────────────────────────────────────────────────────────────────

PORT="${VELOBENCHMARK_PORT:-13843}"
ROOT="${VELOBENCHMARK_DIR:-$HOME/.velobenchmark}"
SRC="${VELO_SRC:-$ROOT/src}"
BIN="$ROOT/velobench"

log()  { printf '\n\033[1;36m==>\033[0m \033[1m%s\033[0m\n' "$*"; }
info() { printf '    %s\n' "$*"; }
warn() { printf '\033[1;33mwarning:\033[0m %s\n' "$*"; }
die()  { printf '\033[1;31merror:\033[0m %s\n' "$*" >&2; exit 1; }

# ── platform detection ──────────────────────────────────────────────────────
OS="$(uname -s)"
ARCH="$(uname -m)"
case "$OS" in
  Linux) OS_TAG="linux" ;;
  Darwin) OS_TAG="macos" ;;
  *) die "unsupported OS: $OS (Linux and macOS are supported)" ;;
esac
case "$ARCH" in
  x86_64|amd64) ARCH_TAG="x86_64" ;;
  aarch64|arm64) ARCH_TAG="arm64" ;;
  *) die "unsupported architecture: $ARCH (x86_64 and arm64 are supported)" ;;
esac
info "platform: $OS_TAG/$ARCH_TAG"

fetch() { # fetch <url> <outfile>
  if command -v curl >/dev/null 2>&1; then
    curl -fsSL "$1" -o "$2"
  elif command -v wget >/dev/null 2>&1; then
    wget -qO "$2" "$1"
  else
    return 127
  fi
}

# ── path additions for toolchains this script may install ───────────────────
export PATH="$HOME/.cargo/bin:$HOME/.local/bin:$HOME/.local/node/bin:$PATH"

have() { command -v "$1" >/dev/null 2>&1; }

# ── prerequisite installers (best effort, home-directory first) ─────────────
ensure_curl() {
  if ! have curl && ! have wget; then
    if [ "$OS_TAG" = "macos" ]; then die "curl missing (it ships with macOS — is your PATH broken?)"; fi
    root_assist "curl" || die "install curl (apt-get install curl) and re-run"
  fi
}

root_assist() { # try the system package manager for $1 (needs root or sudo -n)
  SUDO=""
  if [ "$(id -u)" != "0" ]; then
    have sudo || return 1
    sudo -n true 2>/dev/null || { warn "no passwordless sudo — cannot install $1 via the package manager"; return 1; }
    SUDO="sudo -n"
  fi
  if have apt-get;  then $SUDO apt-get update -qq && $SUDO apt-get install -y "$@" && return 0; fi
  if have dnf;      then $SUDO dnf install -y "$@" && return 0; fi
  if have pacman;   then $SUDO pacman -Sy --noconfirm "$@" && return 0; fi
  if have apk;      then $SUDO apk add "$@" && return 0; fi
  if [ "$OS_TAG" = "macos" ] && have brew; then brew install "$@" && return 0; fi
  return 1
}

ensure_rust() {
  if have cargo; then info "rust: $(cargo --version | awk '{print $2}')"; return 0; fi
  log "installing Rust (rustup, into ~/.cargo)"
  ensure_curl
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable --profile minimal \
    || die "rustup install failed — install Rust from https://rustup.rs and re-run"
  export PATH="$HOME/.cargo/bin:$PATH"
  have cargo || die "cargo still not found after rustup"
  info "rust: $(cargo --version | awk '{print $2}')"
}

NODE_VERSION="20.19.0"
ensure_node() {
  if have node && have npm; then
    info "node: $(node --version)"
    return 0
  fi
  log "installing Node.js $NODE_VERSION (into ~/.local/node)"
  ensure_curl
  NODE_ARCH="$ARCH_TAG"
  NODE_OS="$OS_TAG"
  TARBALL="node-v$NODE_VERSION-$NODE_OS-$NODE_ARCH.tar.gz"
  mkdir -p "$HOME/.local/node"
  fetch "https://nodejs.org/dist/v$NODE_VERSION/$TARBALL" "$ROOT/$TARBALL" \
    || die "could not download Node.js — install Node 18+ and npm, then re-run"
  tar -xzf "$ROOT/$TARBALL" -C "$HOME/.local/node" --strip-components=1
  rm -f "$ROOT/$TARBALL"
  export PATH="$HOME/.local/node/bin:$PATH"
  have node || die "node still not found after install"
  info "node: $(node --version)"
}

ensure_protoc() {
  if have protoc; then info "protoc: $(protoc --version | awk '{print $NF}')"; return 0; fi
  log "installing protoc"
  # 1) system package manager (needs root)
  if [ "$OS_TAG" = "macos" ]; then
    have brew && { brew install protobuf && return 0; }
  else
    if root_assist "protobuf-compiler" && have protoc; then return 0; fi
  fi
  # 2) userland install from the GitHub release (no root needed)
  ensure_curl
  PROTOC_VERSION="25.3"
  case "$OS_TAG-$ARCH_TAG" in
    linux-x86_64) PCTAG="linux-x86_64" ;;
    linux-arm64)  PCTAG="linux-aarch_64" ;;
    macos-x86_64) PCTAG="osx-x86_64" ;;
    macos-arm64)  PCTAG="osx-aarch_64" ;;
  esac
  ZIP="protoc-$PROTOC_VERSION-$PCTAG.zip"
  mkdir -p "$HOME/.local"
  fetch "https://github.com/protocolbuffers/protobuf/releases/download/v$PROTOC_VERSION/$ZIP" "$ROOT/$ZIP" \
    || die "could not download protoc — install it (brew install protobuf / apt-get install protobuf-compiler) and re-run"
  (command -v unzip >/dev/null 2>&1 && unzip -oq "$ROOT/$ZIP" -d "$HOME/.local") \
    || (tar -xf "$ROOT/$ZIP" -C "$HOME/.local" 2>/dev/null) \
    || die "could not unpack protoc (need unzip) — install unzip or protoc and re-run"
  rm -f "$ROOT/$ZIP"
  export PATH="$HOME/.local/bin:$PATH"
  have protoc || die "protoc still not found"
  info "protoc: $(protoc --version | awk '{print $NF}')"
}

# ── fast path: prebuilt release binary ──────────────────────────────────────
try_prebuilt() {
  [ "$GITHUB_REPO" = "YOUR_GITHUB_USER/velobenchmark" ] && return 1
  URL="https://github.com/$GITHUB_REPO/releases/latest/download/velobenchmark-$OS_TAG-$ARCH_TAG.tar.gz"
  log "trying prebuilt binary for $OS_TAG/$ARCH_TAG"
  ensure_curl
  if fetch "$URL" "$ROOT/vb.tar.gz" 2>/dev/null && [ -s "$ROOT/vb.tar.gz" ]; then
    tar -xzf "$ROOT/vb.tar.gz" -C "$ROOT" && rm -f "$ROOT/vb.tar.gz"
    if [ -f "$ROOT/velobench" ] || [ -f "$ROOT/velobench.exe" ]; then
      [ -f "$ROOT/velobench" ] && BIN="$ROOT/velobench"
      return 0
    fi
  fi
  rm -f "$ROOT/vb.tar.gz"
  info "no prebuilt binary available — building from source"
  return 1
}

# ── slow path: build from source ────────────────────────────────────────────
build_from_source() {
  log "fetching the source"
  ensure_curl
  have git || { root_assist "git" || die "git is required to build from source"; }
  if [ "$SRC" != "$ROOT/src" ] && [ -d "$SRC/.git" ]; then
    info "using existing checkout at $SRC"
  elif [ -d "$SRC/.git" ]; then
    info "updating existing checkout at $SRC"
    if git -C "$SRC" fetch --depth 1 origin main 2>/dev/null || git -C "$SRC" fetch --depth 1 origin master 2>/dev/null; then
      git -C "$SRC" reset --hard FETCH_HEAD
    else
      warn "could not fetch updates — building the existing checkout"
    fi
  else
    rm -rf "$SRC"
    git clone --depth 1 "https://github.com/$GITHUB_REPO.git" "$SRC" 2>/dev/null \
      || git clone --depth 1 "https://github.com/$GITHUB_REPO.git" "$SRC" \
      || die "could not clone https://github.com/$GITHUB_REPO"
  fi

  ensure_rust
  ensure_node
  ensure_protoc

  log "building the frontend"
  ( cd "$SRC" && npm --prefix frontend install --no-audit --no-fund --loglevel=error ) \
    || die "npm install failed"
  ( cd "$SRC" && bash scripts/build-frontend.sh ) || die "frontend build failed"

  log "building the server (release mode — this can take several minutes)"
  info "cargo build --release"
  ( cd "$SRC" && cargo build --release --bin velobench ) || die "cargo build failed"
  cp "$SRC/target/release/velobench" "$BIN"
}

# ── serve ───────────────────────────────────────────────────────────────────
wait_healthy() { # wait_healthy <port> <seconds>
  i=0
  while [ "$i" -lt "$2" ]; do
    if have curl; then
      curl -fs "http://127.0.0.1:$1/" >/dev/null 2>&1 && return 0
    else
      wget -qO- "http://127.0.0.1:$1/" >/dev/null 2>&1 && return 0
    fi
    sleep 1; i=$((i + 1))
  done
  return 1
}

lan_ip() {
  if [ "$OS_TAG" = "macos" ]; then
    ipconfig getifaddr en0 2>/dev/null || ipconfig getifaddr en1 2>/dev/null || true
  else
    hostname -I 2>/dev/null | awk '{print $1}' || true
  fi
}

start_server() {
  # a previous installer-run server on the same port? stop it first.
  [ -f "$ROOT/velobench.pid" ] && kill "$(cat "$ROOT/velobench.pid")" 2>/dev/null || true

  # if something already ANSWERS on the port, move on (busy probe is
  # deliberately connection-based — /dev/tcp is a bashism we cannot use here)
  if have curl && curl -fs -m 2 "http://127.0.0.1:$PORT/" >/dev/null 2>&1; then
    PORT=$((PORT + 1))
    warn "port busy — trying $PORT"
  fi

  log "starting VeloBenchmark on port $PORT"
  ( cd "$ROOT" && nohup "$BIN" --host 0.0.0.0 --port "$PORT" > "$ROOT/velobench.log" 2>&1 & echo $! > "$ROOT/velobench.pid" )

  if wait_healthy "$PORT" 30; then
    LAN="$(lan_ip)"
    printf '\n'
    printf '\033[1;32m'
    printf '  ┌──────────────────────────────────────────────────────┐\n'
    printf '  │                                                      │\n'
    printf '  │   VeloBenchmark is running                           │\n'
    printf '  │                                                      │\n'
    printf '  │   ➜  open:  http://localhost:%-23s │\n' "$PORT"
    if [ -n "${LAN:-}" ]; then
      printf '  │            http://%-34s │\n' "$LAN:$PORT"
    fi
    printf '  │                                                      │\n'
    printf '  └──────────────────────────────────────────────────────┘\n'
    printf '\033[0m'
    info "data directory: $ROOT/velobench_data"
    info "logs:           $ROOT/velobench.log"
    info "stop:           kill \$(cat $ROOT/velobench.pid)"
    info "re-start:       $BIN --host 0.0.0.0 --port $PORT   (run it from $ROOT)"
  else
    warn "the server did not answer on port $PORT within 30 s — check $ROOT/velobench.log"
    tail -n 20 "$ROOT/velobench.log" 2>/dev/null || true
    exit 1
  fi
}

# ── main ─────────────────────────────────────────────────────────────────────
main() {
  printf '\n\033[1mVeloBenchmark installer\033[0m — single-binary LLM benchmarking console\n'
  mkdir -p "$ROOT"

  if try_prebuilt; then
    info "prebuilt binary ready: $BIN"
  else
    build_from_source
    info "binary ready: $BIN"
  fi

  start_server
}

main "$@"
