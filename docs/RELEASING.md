# Release brief — publishing prebuilt binaries for all platforms

Copy everything below the line into the release session as its prompt.

---

You are the release session for **VeloBenchmark**, a single-binary LLM
benchmarking console at **https://github.com/sf-stav/VeloBench**. Your job is
to publish prebuilt release artifacts for all four supported platforms so the
project's one-line installer takes its fast path on every machine.

## The contract (do not deviate)

`install.sh` in the repo root downloads, per platform:

```
https://github.com/sf-stav/VeloBench/releases/latest/download/velobenchmark-<os>-<arch>.tar.gz
```

- `<os>` ∈ `linux`, `macos`; `<arch>` ∈ `x86_64`, `arm64` — so exactly four
  tarballs:
  `velobenchmark-linux-x86_64.tar.gz`, `velobenchmark-linux-arm64.tar.gz`,
  `velobenchmark-macos-x86_64.tar.gz`, `velobenchmark-macos-arm64.tar.gz`.
- Each tarball must contain **one file named `velobench`** at its root (the
  release binary; ideally `strip`ped). A sibling `<name>.sha256` checksum file
  is nice to have.
- Names and layout are a hard contract with `install.sh` — if you change
  them, change the installer too.

## How to produce the artifacts

### Option A — CI (preferred)

The repo contains `.github/workflows/release.yml`. It triggers on a `v*` tag,
builds on four runners (`ubuntu-latest`, `ubuntu-24.04-arm`, `macos-13`,
`macos-14`), packages each platform, and attaches everything to a release
with generated notes.

Steps:

1. Make sure `main` is pushed and the workflow file exists on the default
   branch (the workflow is read from the tagged commit, so the tag must be
   created on a commit that contains it).
2. `git tag v<MAJOR.MINOR.PATCH> && git push origin v<MAJOR.MINOR.PATCH>`
   (semver, `v`-prefixed — e.g. `v0.1.0`).
3. Watch the Actions run. Each matrix job uploads a
   `velobenchmark-<os>-<arch>` artifact; the final `release` job creates the
   GitHub release with all tarballs + checksums.
4. If the release job succeeded but you want to fix an asset, delete the tag,
   or use `gh release upload <tag> <files> --clobber`.

### Option B — local builds (fallback or supplement)

`scripts/dist.sh <optional-rust-triple>` builds and packages
`dist/velobenchmark-<os>-<arch>.tar.gz` (+ `.sha256`). Details and
prerequisites are in **docs/building.md → "Publishing prebuilt binaries"**.
Quick facts:

- Native run on any machine covers that machine's platform.
- On an **aarch64 Linux host** (the dev server is one), both Linux targets
  cross-compile as **fully static musl** binaries — the recommended artifact
  kind, since they run on any distribution:

  ```bash
  pip install --user ziglang
  printf '#!/bin/sh\nexec python3 -m ziglang "$@"\n' > ~/.local/bin/zig && chmod +x ~/.local/bin/zig
  cargo install cargo-zigbuild
  rustup target add x86_64-unknown-linux-musl aarch64-unknown-linux-musl

  bash scripts/dist.sh x86_64-unknown-linux-musl    # linux-x86_64
  bash scripts/dist.sh aarch64-unknown-linux-musl   # linux-arm64
  ```

- The GNU cross triples also work when the matching
  `gcc-<arch>-linux-gnu` package is installed
  (`x86_64-unknown-linux-gnu` / `aarch64-unknown-linux-gnu`).
- **macOS must be built on macOS** (Apple SDK licensing). Use the CI
  matrix, or any Mac with `bash scripts/dist.sh`.

Then attach to a release:

```bash
gh release create v<version> dist/velobenchmark-*.tar.gz dist/*.sha256 \
  --title "v<version>" --notes "..."
```

## Verification (always do this)

1. All four URLs answer 200:

   ```bash
   for a in linux-x86_64 linux-arm64 macos-x86_64 macos-arm64; do
     curl -sI -o /dev/null -w "%{http_code} $a\n" \
       "https://github.com/sf-stav/VeloBench/releases/latest/download/velobenchmark-$a.tar.gz"
   done
   ```

2. Spot-check an artifact: download one, `tar -tzf` it — it must list a
   single `velobench` entry; run `file` on the extracted binary and confirm
   the architecture matches the tag.
3. End-to-end on a clean machine (and ideally once on each OS/arch you can
   reach):

   ```bash
   curl -fsSL https://raw.githubusercontent.com/sf-stav/VeloBench/main/install.sh | sh
   ```

   It must print "prebuilt binary ready", start the server, and show the URL
   banner (`http://localhost:13843`).

## Rules and fallbacks

- **Never break the contract.** If an artifact name has to change, update
  `install.sh` (`try_prebuilt`) in the same release.
- **Missing platform ≠ broken installer.** `install.sh` falls back to a
  source build when a tarball is missing — mention any missing platform in
  the release notes.
- The source-build fallback installs Rust/Node/protoc into the user's home
  directory; it needs `git`, `curl`, and (on Linux) a passwordless-sudo or
  pre-installed `protoc` is helpful but has a userland fallback too.
- Version tags are `v`-prefixed semver; the installer hits the **latest**
  release, so the newest tag becomes what users get.
- Build/reference documentation: **docs/building.md** (build, packaging,
  cross-compile setup), **docs/user-manual.md** (what users do after
  install), `install.sh` header comment (installer contract).

## Deliverables for this session

1. All four tarballs (+ checksums) attached to a GitHub release.
2. The verification loop above printing `200` four times.
3. A short report: release tag, artifact list with sizes, verification
   output, and any platform that could not be produced (with the reason).
