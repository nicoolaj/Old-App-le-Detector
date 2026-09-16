**English** | [Français](README.fr.md)

# Old App(le) Detector

[![Latest release](https://img.shields.io/github/v/release/nicoolaj/Old-App-le-Detector?label=latest%20release)](https://github.com/nicoolaj/Old-App-le-Detector/releases/latest)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue)](LICENSE)

Find out which apps on your Mac will stop working when Apple retires Rosetta 2.

Rosetta 2 is the compatibility layer that lets Apple Silicon Macs (M1, M2, M3...) run older Intel-only software. Apple is phasing it out — once it's gone, any app or command-line tool without a native Apple Silicon version will simply stop launching. Old App(le) Detector scans your Mac and tells you exactly what's affected, **not just apps in your Applications folder**, but also command-line tools installed via Homebrew, MacPorts, or anywhere on your `PATH`.

## Download

👉 [**Latest version (.dmg)**](https://github.com/nicoolaj/Old-App-le-Detector/releases/latest)

1. Download the `.dmg`
2. Open it
3. Drag the app into your `Applications` folder

All versions: [releases page](https://github.com/nicoolaj/Old-App-le-Detector/releases).

## How to use it

1. Open the app and tick the sources you want to scan — Applications, Homebrew, MacPorts, your `PATH`, or any custom folder.
2. Click **Scan**.
3. Review the results.
4. Optionally click **Export as TXT** or **Export as CSV** if you want to keep a copy.

## Understanding the results

| Status | What it means |
|---|---|
| **Requires Rosetta 2** | Intel-only today — will stop working once Rosetta 2 is retired |
| **Obsolete** | Even older architecture (32-bit/PowerPC) — already broken on this Mac |
| **Unknown** | Couldn't be read — worth checking by hand |
| **Native (arm64)** | Already Apple Silicon — nothing to do |

## Good to know

- Helper executables tucked inside an app (plugins, extensions) aren't inspected — only the app or tool itself.
- This is a snapshot of what's installed on disk, not of what's currently running — it won't tell you which apps are running under Rosetta *right now* (Activity Monitor does that).

## For developers

Want to build it from source, understand how it works, or contribute? See [CONTRIBUTING.md](CONTRIBUTING.md) (English) or [CONTRIBUTING.fr.md](CONTRIBUTING.fr.md) (Français).

## License

MIT — see [LICENSE](LICENSE).
