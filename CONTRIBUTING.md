**English** | [Français](CONTRIBUTING.fr.md)

# Contributing to Old App(le) Detector

Looking for the user guide instead? See [README.md](README.md).

## How it works

Old App(le) Detector reads the **Mach-O header** directly from each binary (thin or universal) to know which architectures it contains. It doesn't shell out to `file`, `lipo`, or rely on the Xcode Command Line Tools being installed — see [`src-tauri/src/macho.rs`](src-tauri/src/macho.rs).

It scans several sources, each independently toggleable:

- `/Applications` and `~/Applications`
- Spotlight (`mdfind`) — catches apps installed elsewhere on disk
- `$PATH`, resolved from the **login shell** — an app launched from Finder doesn't inherit the extended `PATH` set in `.zshrc`, so the scanner spawns a login shell to get the real one
- Homebrew (`/opt/homebrew` and `/usr/local`, including keg-only formulas under `Cellar/`)
- MacPorts (`/opt/local`)
- Any additional folders the user adds

Edge cases handled explicitly (see [`src-tauri/src/scan.rs`](src-tauri/src/scan.rs)):

- **iOS apps installed on Apple Silicon** (the "iPhone & iPad Apps" category on the Mac App Store) use a different bundle layout (`Wrapper/<Name>.app`, no `Contents/`). They're always arm64, so they're never flagged.
- **Web shortcuts** (Safari "Add to Dock", Chrome PWAs) sometimes have no executable of their own — they're excluded from the report rather than reported as "Unknown".
- A shell-script launcher that execs a separate binary (e.g. `my-tool` → `my-tool-bin`) stays classified as **Unknown**: the real architecture can't be inferred automatically, and guessing would be worse than flagging it.

## Project layout

```
src/                      Frontend (vanilla HTML/CSS/JS, no bundler)
  index.html
  main.js
  styles.css
  i18n.js                 Locale detection + translation helper
  locales/
    en.js, fr.js           One file per UI language

src-tauri/src/
  macho.rs                 Reads architectures from the Mach-O header
  scan.rs                  Walks the sources + classifies each binary
  lib.rs                   Tauri commands (host_info, scan, export_report) + TXT/CSV export
  locale/
    mod.rs                 Strings struct + locale dispatch (falls back to English)
    en.rs, fr.rs            One file per export language

Makefile                   help / secu / app / pkg / clean / mrproper targets
.github/workflows/
  release.yml               Builds + publishes a GitHub release on a `v*` tag
  security.yml               Runs `make secu` on push/PR + weekly
```

## Prerequisites

- macOS (Apple Silicon or Intel)
- [Rust](https://rustup.rs) with both Apple targets: `rustup target add aarch64-apple-darwin x86_64-apple-darwin`
- [`tauri-cli`](https://v2.tauri.app): `cargo install tauri-cli --locked` (auto-installed by `make app`/`make pkg` if missing)

No frontend dependency — vanilla HTML/CSS/JS, no Node/npm needed to build the app. Node is only useful if you're changing the frontend tooling itself (there isn't any today).

## Development loop

```bash
cargo tauri dev
```

Hot-reloads the frontend; see [README.md](README.md) for the UI walkthrough.

## Building

```bash
make help   # list available targets
make secu   # security + lint (fmt --check, clippy -D warnings, cargo audit)
make app    # build the universal .app (arm64 + x86_64)
make pkg    # build the .dmg (includes the .app)
```

The `.app` is ad-hoc signed (`signingIdentity: "-"` in `tauri.conf.json`), which is fine for local use or internal distribution (MDM, network share). Public distribution outside the Mac App Store would need a Developer ID signature and notarization — not configured here.

## Testing

```bash
cd src-tauri && cargo test
```

23 tests across `macho.rs`, `scan.rs`, and `lib.rs`. **Not currently run in CI** — run it locally before opening a PR.

## Linting & security

`make secu` runs `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo audit`. This **is** what CI runs: [`security.yml`](.github/workflows/security.yml) triggers on push/PR to `main` touching `src-tauri/**` or the `Makefile`, plus a weekly cron audit for newly-disclosed vulnerabilities.

## Adding a language

The app auto-detects the user's browser locale and falls back to English; adding a language means adding one file on each side:

**Frontend (UI):**
1. Copy `src/locales/en.js` to `src/locales/xx.js` and translate every value (keep the keys).
2. Register it in the `LOCALES` map in [`src/i18n.js`](src/i18n.js) — `detectLocale()` picks it up automatically from `navigator.languages`.

**Backend (TXT/CSV export):**
1. Copy `src-tauri/src/locale/en.rs` to `xx.rs` and fill in every `Strings` field.
2. Declare `mod xx;` and add a match arm in `for_locale()` in [`src-tauri/src/locale/mod.rs`](src-tauri/src/locale/mod.rs).

`write_aligned_rows` in `lib.rs` pads the TXT export columns dynamically, so you don't need to hand-count spaces for a translation of a different length. The frontend detects the locale independently and passes it to the `export_report` command as the `locale` argument — keep both locale sets in sync for a given language code.

## Release process

Maintainer-only: push a `v*` tag. [`release.yml`](.github/workflows/release.yml) builds the universal `.dmg` (`make pkg`) and publishes it as a GitHub release with auto-generated notes. Contributors don't need to do this themselves.

## Contribution guidelines

- Keep PRs small and focused.
- Match the existing code style.
- Run `make secu` and `cargo test` locally before opening a PR (only `secu` runs in CI).
- New source files get an `// SPDX-License-Identifier: MIT` header, matching the rest of the codebase.
- Bugs and proposals: [GitHub Issues](https://github.com/nicoolaj/Old-App-le-Detector/issues).

## License

MIT — see [LICENSE](LICENSE). By contributing, you agree your changes are licensed under it.
