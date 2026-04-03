<div align="center">

<svg xmlns="http://www.w3.org/2000/svg" width="120" height="120" viewBox="0 0 120 120">
  <defs>
    <linearGradient id="g" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#00d4ff"/>
      <stop offset="100%" stop-color="#7c3aed"/>
    </linearGradient>
  </defs>
  <circle cx="46" cy="46" r="30" fill="none" stroke="url(#g)" stroke-width="2.5" opacity="0.25"/>
  <circle cx="46" cy="46" r="22" fill="none" stroke="url(#g)" stroke-width="4"/>
  <line x1="63" y1="63" x2="95" y2="95" stroke="url(#g)" stroke-width="7" stroke-linecap="round"/>
</svg>

# SHERLOCK-RS

**Hunt social media accounts by username — Rust Edition**

[![Rust](https://img.shields.io/badge/Rust-1.94+-orange?style=flat-square&logo=rust)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-MIT-blue?style=flat-square)](LICENSE)
[![Sites](https://img.shields.io/badge/Sites-478+-brightgreen?style=flat-square)](https://github.com/sherlock-project/sherlock)
[![Platform](https://img.shields.io/badge/Platform-Windows-0078D4?style=flat-square&logo=windows)](https://github.com/Oli97430/sherlock-rs/releases)
[![Author](https://img.shields.io/badge/Author-Olivier%20Hoarau-purple?style=flat-square)](mailto:tarraw974@gmail.com)

*A complete Rust rewrite of [Sherlock](https://github.com/sherlock-project/sherlock) with a modern dark web UI — single `.exe`, zero installation.*

</div>

---

## Overview

**Sherlock-RS** scans **478+ social platforms** in parallel to check whether a username exists. Just run the exe: a local server starts, your browser opens automatically, and results stream in real time.

> **New:** Simultaneous multi-username search with tabs, automatic rotation of 25 real User-Agents, and smart retry logic on network errors.

---

## Features

| Feature | Details |
|---|---|
| 🔍 **478+ sites scanned** | Official Sherlock database, one-click update from the UI |
| 👥 **Multi-username** | Enter multiple names at once (comma or newline), results per tab |
| ⚡ **Parallel scanning** | 20 concurrent requests via Tokio async — full scan in minutes |
| 🔄 **User-Agent rotation** | 25 real browsers (Chrome, Firefox, Edge, Safari, Opera…) rotated randomly per request |
| 🔁 **Smart retry** | 3 attempts with exponential backoff (500 ms / 1 000 ms) on network errors only |
| 🎨 **Modern UI** | Dark-themed web interface with real-time results (Server-Sent Events) |
| 🛡️ **WAF detection** | Cloudflare, PerimeterX, AWS CloudFront detected and flagged |
| 🧅 **Proxy / Tor** | Native SOCKS5 support (`socks5://127.0.0.1:9050` for Tor) |
| 📥 **Export** | Download results as CSV (spreadsheet) or TXT |
| 🔎 **Filter & sort** | Sort by name, status or response time — live text filter |
| 📦 **Zero install** | Single self-contained 5 MB `.exe`, no dependencies required |

---

## Installation

### Quick method — Download the binary

1. Download the latest release from the [**Releases**](https://github.com/Oli97430/sherlock-rs/releases) page
2. Double-click `sherlock-rs.exe`
3. Your browser opens automatically — that's it

### Build from source

**Requirements:**
- [Rust](https://rustup.rs/) (install via `rustup`)
- [Visual Studio Build Tools 2022](https://visualstudio.microsoft.com/downloads/) with the *Desktop development with C++* workload

```bash
git clone https://github.com/Oli97430/sherlock-rs.git
cd sherlock-rs
cargo build --release
```

The binary will be located at `target/release/sherlock-rs.exe`.

---

## Usage

```bash
sherlock-rs.exe
```

The program starts a local server on a random port and opens your default browser. No extra commands needed.

### Basic steps

1. Enter one or more usernames to search (separated by comma or newline)
2. Adjust options if needed (timeout, proxy, NSFW)
3. Click **Hunt** or press `Enter`
4. Results appear in real time, site by site
5. Export results as CSV or TXT via the dedicated buttons

### Multi-username search

The input field accepts multiple names at once:

```
johndoe
janedoe, alice
```

Each username gets its own tab with a live counter of accounts found. The tab being scanned pulses in blue.

### Interface options

| Option | Description |
|---|---|
| **Timeout** | Maximum wait time per request (default: 30 s, min: 5 s) |
| **NSFW** | Include adult content platforms in the search |
| **Proxy** | SOCKS5 or HTTP proxy URL, e.g. `socks5://127.0.0.1:9050` for Tor |
| **Update DB** | Downloads the latest site database from GitHub |

### Keyboard shortcuts

| Key | Action |
|---|---|
| `Enter` | Start search (from the username field) |
| `Shift + Enter` | Add a new line (multi-username input) |
| `Escape` | Stop the current search |

---

## How it works

### Detection methods

Sherlock-RS faithfully implements the 3 detection methods from the original project:

| Type | Logic |
|---|---|
| `status_code` | HTTP 404 (or custom code) → not found; 200-299 → found |
| `message` | Specific error text found in response body → not found |
| `response_url` | Redirects disabled; 200-299 → found, otherwise not found |

WAF detection (Cloudflare, PerimeterX…) is applied **first**, before any other logic, to avoid false positives. Blocked results are reported separately with the **Blocked** status.

### User-Agent rotation

Each individual request randomly picks a User-Agent from 25 real modern browsers:

- Chrome 128–131 (Windows, macOS, Linux, Android)
- Firefox 130–133 (Windows, macOS, Linux)
- Edge 130–131 (Windows, macOS)
- Safari 17 (macOS, iOS)
- Opera 116, Brave

This significantly reduces bot-detection-based blocking.

### Exponential backoff retry

On network errors (timeout, connection refused, DNS failure):

```
Attempt 1  →  fails  →  wait 500 ms
Attempt 2  →  fails  →  wait 1 000 ms
Attempt 3  →  final result (success or error displayed)
```

Valid HTTP responses (even 403 or 404) do **not** trigger a retry.

---

## Result statuses

| Status | Meaning | Tip |
|---|---|---|
| ✅ **Found** | Account detected on the platform | Click the URL to open the profile |
| ❌ **Not found** | No account under this name | — |
| ⚠️ **Blocked** | Blocked by a WAF (Cloudflare…) | Retry with a proxy or Tor |
| 🔴 **Error** | Network error or timeout after 3 attempts | Increase the timeout |
| ⬜ **Invalid** | Username format doesn't match the site's rules | Normal for some sites |

---

## Code architecture

```
sherlock-rs/
├── Cargo.toml              # Dependencies and project metadata
├── src/
│   ├── main.rs             # Entry point, console banner, server startup
│   ├── server.rs           # Axum server: REST routes + SSE streaming
│   ├── checker.rs          # Async scan engine: UA rotation, retry, detection
│   ├── sites.rs            # Load and parse data.json (local cache + GitHub)
│   ├── result.rs           # Types: QueryStatus (enum), QueryResult (struct)
│   └── export.rs           # CSV and TXT export grouped by username
└── frontend/
    └── index.html          # Full UI embedded in the binary (HTML/CSS/JS)
```

### Rust crates used

| Role | Crate |
|---|---|
| Async runtime | `tokio 1` |
| Web server + SSE | `axum 0.7` |
| HTTP client | `reqwest 0.12` |
| JSON serialization | `serde` + `serde_json` |
| Regular expressions | `regex` |
| Randomness (UA rotation) | `rand 0.8` |
| CSV export | `csv` |
| Browser launcher | `open` |
| System directories | `dirs` |
| Error handling | `anyhow` |

---

## Credits

- **Author**: Olivier Hoarau — [tarraw974@gmail.com](mailto:tarraw974@gmail.com)
- **Original project**: [Sherlock Project](https://github.com/sherlock-project/sherlock) by [@sdushantha](https://github.com/sdushantha) and the community (MIT license)
- **Database**: `data.json` maintained by the Sherlock Project community

---

## License

MIT — see the [LICENSE](LICENSE) file

---

<div align="center">
  <sub>Built with passion and Rust 🦀 — Olivier Hoarau</sub>
</div>
