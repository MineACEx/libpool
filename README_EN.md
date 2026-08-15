<p align="center">
  <img src="https://raw.githubusercontent.com/MineACEx/libpool/master/webroot/icons/icon.png" width="112" height="112" alt="LibPool" />
</p>

<h1 align="center">LibPool</h1>

<p align="center">
  <b>Install common/hot Android native libraries that are missing by default, and bind-mount them to <code>/system/bin</code> (and <code>/system/lib</code>) with one tap.</b><br />
  Ships KsuWebUI: 322+ libraries · light/dark theme · custom wallpaper · cloud update · Rust native, low power
</p>

<p align="center">English · [简体中文](README.md)</p>

<p align="center">
  <a href="https://github.com/MineACEx/libpool/blob/master/LICENSE"><img src="https://img.shields.io/badge/License-Apache--2.0-blue.svg" alt="License" /></a>
  <a href="https://github.com/MineACEx/libpool#installation"><img src="https://img.shields.io/badge/Platform-Magisk%20%7C%20KernelSU%20%7C%20APatch-orange.svg" alt="Platform" /></a>
  <a href="https://github.com/MineACEx/libpool#usage"><img src="https://img.shields.io/badge/UI-KsuWebUI-0099ff.svg" alt="KsuWebUI" /></a>
  <a href="https://github.com/MineACEx/libpool/releases"><img src="https://img.shields.io/github/v/release/MineACEx/libpool?color=0071e3&amp;label=Release" alt="Release" /></a>
  <a href="https://github.com/MineACEx/libpool/releases"><img src="https://img.shields.io/github/downloads/MineACEx/libpool/total?color=34c759&amp;label=Downloads" alt="Downloads" /></a>
  <a href="https://github.com/MineACEx/libpool"><img src="https://img.shields.io/github/stars/MineACEx/libpool?color=e5a00d&amp;label=Stars" alt="Stars" /></a>
  <a href="https://github.com/MineACEx/libpool"><img src="https://img.shields.io/github/forks/MineACEx/libpool?color=5856d6&amp;label=Forks" alt="Forks" /></a>
  <a href="https://github.com/MineACEx/libpool"><img src="https://img.shields.io/github/last-commit/MineACEx/libpool?label=Last%20commit" alt="Last commit" /></a>
  <a href="https://github.com/MineACEx/libpool/issues"><img src="https://img.shields.io/github/issues/MineACEx/libpool?color=ff453a&amp;label=Issues" alt="Issues" /></a>
  <a href="https://github.com/MineACEx/libpool"><img src="https://img.shields.io/github/languages/top/MineACEx/libpool?label=Language" alt="Language" /></a>
  <a href="https://github.com/MineACEx/libpool"><img src="https://img.shields.io/github/languages/code-size/MineACEx/libpool?label=Code%20size" alt="Code size" /></a>
</p>

Supports: **Magisk** / **KernelSU** / **APatch**, including **32-bit (armv7)** devices.

---

## Features

- **322 extension libraries** (10 categories), including useful tools: `adb` / `fastboot` (android-tools), `curl`, `git`, `htop`, `strace`, `gdb`, `tmux`, `rsync`, `tcpdump`, `nmap`, `jq`, `ffmpeg`, `nodejs`, `go`, `rust`, `python`, and more
- **18 built-in core libraries** (curl / git / tar / unzip / awk …) auto-installed at boot
- **Always latest**: libraries are resolved and installed at the newest version from Termux official/mirror repos — no manual updating
- **Instant effect**: bind mounts take effect immediately — toggle on/off without rebooting; restored at boot automatically
- **Slim freely**: delete any library you don't want to free space
- **Low power, zero resident**: the manager is a static Rust binary that runs only during install/mount and exits — near-zero idle footprint
- **32-bit compatible**: armv7 devices automatically use the 32-bit native binary; other architectures fall back to a shell script
- **Cloud update**: compares the local vs. cloud version number and shows an update dialog when a new version is found; the download link opens your default browser
- **Beautiful WebUI**: iOS-18-style frosted glass + G2 continuous rounded corners, touch-first motion (ripple / press / interrupt / scroll reveal), light & dark themes, adjustable wallpaper blur, optimized for phones and tablets

## What Rust Does

`tools/libman` is a native manager compiled with **Rust** (statically linked `aarch64-unknown-linux-musl` and `armv7-unknown-linux-musleabihf` binaries, zero system dependencies). It implements all of the module's core logic:

| Command | Responsibility | Why Rust |
|---------|----------------|----------|
| `list` / `status` | Reads `state.json` and installed libraries, outputs the JSON the WebUI needs | Fast parsing, no interpreter overhead |
| `install` | Resolves the latest version from Termux repos → downloads `.deb` → unpacks (built-in ar/JSON/xz parsers) → extracts `bin`/`lib` | Pure logic; Rust memory safety means no crashes |
| `mount` / `unmount` | `mount --bind` into `/system/bin`, `/system/lib` — instant effect | Stable syscalls, controlled privileges |
| `toggle` | One-tap on/off (auto-installs missing libs) | Atomic state switching |
| `apply` | Idempotently replays mounts at boot from saved state | Local operation, fast, non-blocking boot |
| `ensure-core` | Installs missing core libraries in the background | Runs in background, non-blocking |
| `reset` | Uninstalls everything and clears state | Fault-tolerant cleanup |
| `config` | Reads/writes mirror etc. configuration | Minimal JSON serialization |

**Why Rust instead of shell:** shell forks an interpreter for every operation, has fragile string handling, and error-prone JSON parsing; Rust compiles to a few-KB executable that starts in milliseconds, uses minimal memory, and never dies halfway from a script error. `tools/libman.sh` is kept only as a fallback for extreme environments (where the binary cannot execute).

## Installation

1. Download `libpool-<version>.zip` from [Releases](./releases)
2. Open Magisk / KernelSU manager → Modules → Install from storage → select the zip
3. Reboot (or open the module's WebUI in KernelSU to start managing right away)

> On first boot the core libraries (~18) are installed automatically in the background; watch progress on the "Mounted" tab in the WebUI.

## Usage

Open **KernelSU Manager → Modules → LibPool → WebUI**:

| Page | Description |
|------|-------------|
| Mounted | See downloaded/mounted libraries; toggle and delete them |
| Store | Browse 322 libraries, search, filter by category, one-tap download |
| Settings | Light/dark theme, custom wallpaper (URL or gallery) + wallpaper blur, download mirror, full reset |

### Announcement (optional)

The module directory `/.git/url.txt` is the announcement config file — **you can edit it directly**:
put a plain-text URL (one per line, the first valid `http(s)://` link is used). The WebUI shows the announcement at most once per day.
The site must be plain text with no HTML tags. Leave it empty or delete the file to disable.

### Cloud Update (optional)

- `/.git/update.txt`: plain-text URL of the cloud **version number** (content is one version, e.g. `1.1.0`). On startup the WebUI fetches it and compares with `version=` in the local `module.prop`; if they differ, an update dialog appears (same look as a normal announcement).
- `/.git/download.txt`: URL of the **download/update page**. Tapping the blue underlined link in the update dialog opens it in the system default browser.
- When those files are empty or missing, the built-in constants in `app.js` (`UPDATE_URL` / `UPDATE_DOWNLOAD_URL` / `DEFAULT_RELEASES_URL`) are used instead.
- A given cloud version is only reminded once; a newer cloud version will trigger the reminder again.

### Download Mirror

If you are in mainland China, choose the Tsinghua or BFSU mirror in **Settings → Download Mirror** for more stable speeds.

## Architecture Support

| Device arch | Used mode |
|-------------|-----------|
| aarch64 / arm64 | `tools/libman` (native) |
| armv7 / arm (32-bit) | `tools/libman-arm` (32-bit native) |
| other (x86, …) | `tools/libman.sh` (shell fallback) |

The scripts auto-select the right one by `uname -m` at install/boot; no manual config.

## Build

### Prerequisites

- Rust toolchain (stable)
- Cross-compilation targets:
  ```bash
  rustup target add aarch64-unknown-linux-musl        # 64-bit
  rustup target add armv7-unknown-linux-musleabihf    # 32-bit (optional)
  ```

### Windows

```powershell
powershell -ExecutionPolicy Bypass -File scripts/build.ps1
```

### Linux / macOS

```bash
sh scripts/build.sh
```

Output: `dist/libpool-<version>.zip`.

## Directory Layout

```
libpool/
├── module.prop            # Module metadata
├── customize.sh           # Install: init + arch selection + background core-lib install
├── service.sh             # Boot: replay mounts (timeout-guarded) + background core-lib install
├── post-fs-data.sh        # Early-boot placeholder (very light, non-blocking)
├── uninstall.sh           # Uninstall: release all bind mounts
├── sepolicy.rule          # SELinux (least privilege)
├── module-config/         # Source-level announcement/update config (url.txt / update.txt / download.txt)
│                          # Packaged into the module as .git/ where users can edit it
├── system/bin|lib         # Magic-mount targets
├── libs/<id>/bin|lib      # Actual library files (created after install)
├── tools/
│   ├── libman             # Rust native manager (aarch64)
│   ├── libman-arm         # Rust native manager (armv7 32-bit)
│   └── libman.sh          # Shell fallback (extreme environments)
├── webroot/               # KsuWebUI (fully offline)
│   └── data/repos.json    # 322-library database
├── repo_src/              # Build source data
├── src/rust-libman/       # Rust source
└── scripts/               # Build scripts
```

## FAQ

**Q: Does this slow down boot or risk a boot loop?**
No. `customize.sh` returns within seconds, `service.sh` mount replay is guarded by `timeout 60`, and core-library installs are fully background. Any failure only writes a log — **disabling the module fully restores boot**.

**Q: Does it drain battery / use resources?**
No. There is no daemon, no polling, and no resident process; idle CPU/RAM usage is near zero.

**Q: Can it run on 32-bit phones?**
Yes. armv7 devices use `libman-arm` automatically; other architectures fall back to the shell script.

**Q: Some libraries are installed but the command is not found?**
Some libraries (e.g. python) also install dependent shared libraries into `/system/lib`. If it still reports not found, toggle the library off/on once in the WebUI.

**Q: Downloads are slow in mainland China?**
Switch to the Tsinghua or BFSU mirror in **Settings**.

**Q: Worried the module could break the system?**
The module only bind-mounts libraries you enable into `/system/bin`; it never modifies the system partition. Deleting a library does not affect the original system files.

**Q: Want to report a bug or request a feature?**
Open an issue on [Issues](./issues) with: device model / Android version / arch (`uname -m`), module version, reproduction steps, and WebUI console logs.

## Contributing

Pull requests and issues are welcome. Code follows the Apache-2.0 license; every source file declares the author (MINO · Himer).

## License

This project is open source under the **Apache License 2.0**. See [LICENSE](./LICENSE).
