<div align="center">

# 🌐 IRBox Client

![IRBox Screenshot](screenshot.png)

**A versatile and secure proxy client built with modern technologies to provide seamless and reliable internet connectivity**

Designed for privacy-conscious users, IRBox offers multi-protocol support, advanced routing capabilities, and intuitive management tools to ensure a smooth and secure browsing experience.

[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](LICENSE) 
[![Releases](https://img.shields.io/github/downloads/frank-vpl/IRBox/total.svg)](https://github.com/frank-vpl/IRBox/releases/latest)
[![Latest Release](https://img.shields.io/github/v/release/frank-vpl/IRBox)](https://github.com/frank-vpl/IRBox/releases/latest)

[Farsi Version](README_FA.md)

</div>

## 🚀 Key Features

### Multi-Protocol Support
- **VLESS**
- **VMess**
- **Shadowsocks**
- **Trojan**
- **Hysteria2**
- **TUIC**
- **SSH**
- **WireGuard**

### Advanced Management
- **Subscription Support** - Import and auto-update subscription URLs
- **Routing Rules** - Domain-based rules (proxy/direct/block/interface) with presets for ad blocking and regional bypass
- **Split Tunneling** - Choose default route: proxy all traffic or selected domains
- **Custom Interface Routing** - Route selected domains into an externally-managed network interface (e.g. a WireGuard/AmneziaWG tunnel)

### Connection Modes
- **System Proxy** - HTTP proxy for system-wide access
- **TUN Mode** - Full VPN capturing all traffic
- **Admin Elevation** - One-click "Run as Administrator" for TUN mode

### User Experience
- **Onboarding** - Interactive guided tour for first-time users
- **TCP Ping** - Bulk server latency testing
- **Auto-select Best Server** - Intelligent server selection
- **Themes** - 2 color themes (Dark, Light)
- **Styles** - Default, Minimal

## 🔀 Custom Interface Routing

IRBox can route selected domains **into a network interface that you bring up and manage yourself** — for example a WireGuard/AmneziaWG tunnel created with `table = off`. IRBox does **not** create or tear down the interface; it only directs matching traffic into it via sing-box. This is sing-box only (with the Xray core, the `interface` action falls back to `proxy`).

**How to use it:**

1. Bring up your interface outside IRBox (e.g. `awg0` / `wg0`). On Linux, configure it with `table = off` and its own firewall mark so the OS does not route everything into it automatically.
2. In IRBox, open the **Routing** page and find the **Custom interface routing** section:
   - **Interface name** — the interface to bind to, e.g. `awg0`.
   - **Endpoint IPs to exclude** — the tunnel server IP(s), comma-separated. In TUN mode these are kept on a direct route so the tunnel's own handshake is not captured back into sing-box (which would otherwise create a routing loop).
   - **Firewall mark (fwmark)** — optional `SO_MARK` to tag the bridged traffic (Linux), matching your interface's mark.
3. Add a routing rule (or edit an existing one) and set its action to **Interface**. Matching domains are now routed into your interface. If no interface name is set, the action safely falls back to `proxy`.

> **Platform note:** solid on **Linux**; binding works on Windows/macOS too, but managing a `table = off` interface there is your responsibility (best-effort).

## 🎁 Gift: Free Xray / sing-box Configs

As a small gift to the community, IRBox provides a **free public subscription** compatible with **Xray** and **sing-box** clients.

🔗 **Subscription URL:**
```
https://raw.githubusercontent.com/frank-vpl/servers/refs/heads/main/irbox
```

## 📥 Download

If you just want to use IRBox, grab a prebuilt installer for your platform from the **[Releases page](https://github.com/creatorofuniverses/IRBox/releases)** — no toolchain or compilation required:

| Platform | Files |
|----------|-------|
| **Windows** | `.exe` (NSIS installer) or `.msi` |
| **macOS** | `.dmg` (Intel & Apple Silicon) |
| **Linux** | `.AppImage`, `.deb`, or `.rpm` |

> ℹ️ IRBox starts in **Proxy Mode** by default (no special permissions). **TUN Mode** routes all traffic and needs elevated privileges — use **Settings → VPN Mode → TUN → Run as Administrator**, or launch the app with `sudo` / as Administrator.

## 🛠️ Build from source

For development or to build the installers yourself.

### Prerequisites
- **Rust and Cargo** (stable)
- **Node.js and npm** (Node 18+)
- **Tauri CLI v2** — `cargo install tauri-cli --version "^2"`
- **Platform dependencies** ([Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)):
  - **Linux:** `libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf`
  - **Windows:** Microsoft C++ Build Tools + WebView2 (preinstalled on Windows 11)
  - **macOS:** Xcode Command Line Tools

### Setup

1. **Clone the repository**
   ```bash
   git clone https://github.com/creatorofuniverses/IRBox.git
   cd IRBox
   ```

2. **Install frontend dependencies**
   ```bash
   npm install
   ```

3. **Download the proxy cores** (sing-box & xray sidecars + geoip/geosite). The target is auto-detected from `rustc`; pass one explicitly to cross-build (e.g. `./cores.sh x86_64-pc-windows-msvc`).

   **Linux/macOS:**
   ```bash
   chmod +x cores.sh
   ./cores.sh
   ```

   **Windows:**
   ```bash
   ./cores.bat
   ```

### Run & build

```bash
# Run in development (hot reload)
cargo tauri dev

# Build release installers for the current platform
cargo tauri build
```

Built installers/packages land in `src-tauri/target/release/bundle/` (or `src-tauri/target/<target>/release/bundle/` when a `--target` is given).

## 📦 Creating a release

Releases are produced automatically by the [`Build` workflow](.github/workflows/build.yaml), which builds for Windows (x86_64 & ARM64), macOS (Intel & Apple Silicon), and Linux (x86_64), then publishes the installers to a GitHub Release. Trigger it by either:

- **Pushing a version tag:**
  ```bash
  git tag v1.0.0
  git push origin v1.0.0
  ```
- **Or** running the workflow manually from the **Actions** tab (*workflow_dispatch*) and entering a tag name.

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request. For major changes, please open an issue first to discuss what you would like to change.

## 📄 License

This project is licensed under the GNU General Public License v3.0 (GPL-3.0) - see the [LICENSE](LICENSE) file for details.

### Core Technologies

IRBox leverages the power of two leading proxy technologies:

<div align="center">

| Core | Description |
|------|-------------|
| [Xray-core](https://github.com/XTLS/Xray-core) | A platform for building proxies to bypass network restrictions |
| [sing-box](https://github.com/SagerNet/sing-box) | The universal proxy platform |

</div>

### Licenses of Third-Party Libraries

- [Rust](https://www.rust-lang.org/) - [License](./licenses/rust.md)
- [Tauri](https://v2.tauri.app/) - [License](./licenses/tauri.md)
- [sing-box](https://github.com/SagerNet/sing-box) - [License](./licenses/sing-box.md)
- [Xray-core](https://github.com/XTLS/Xray-core) - [License](./licenses/xray.md)

## 🙏 Acknowledgments

- Built with [Tauri](https://tauri.app/) - Framework for building secure native apps
- Powered by [sing-box](https://github.com/SagerNet/sing-box) and [Xray-core](https://github.com/XTLS/Xray-core)
- Inspired by the need for secure and flexible VPN solutions

## 📚 Documentation
[IRBox Documentation](./docs/README.md)

## 🎨 Design Assets

<div align="center">

### App Logo & Icons
![PiraIcons](https://img.shields.io/badge/Icons_by-Hossein_Pira-3d85c6?style=for-the-badge&logo=github)

- Icons by Hossein Pira – [PiraIcons](https://github.com/code3-dev/piraicons-assets) - [License](./licenses/piraicons.md)

</div>

## 🧩 Technologies Used

<div align="center">

### Frontend Dependencies
![React](https://img.shields.io/badge/React-20232a?style=for-the-badge&logo=react&logoColor=61DAFB)
![TypeScript](https://img.shields.io/badge/TypeScript-007ACC?style=for-the-badge&logo=typescript&logoColor=white)
![Vite](https://img.shields.io/badge/Vite-B73BFE?style=for-the-badge&logo=vite&logoColor=FFD62E)

### Framework & Core
![Tauri](https://img.shields.io/badge/Tauri-FFD62E?style=for-the-badge&logo=tauri&logoColor=black)
![Rust](https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white)

</div>

### Dependencies
- [react](https://react.dev/) - A JavaScript library for building user interfaces
- [react-dom](https://reactjs.org/docs/react-dom.html) - Provides DOM-specific methods that can be used at the top level of your app
- [@tauri-apps/api](https://github.com/tauri-apps/tauri) - Tauri API bindings
- [@tauri-apps/plugin-deep-link](https://github.com/tauri-apps/plugins-workspace) - Tauri plugin for deep linking
- [@tauri-apps/plugin-shell](https://github.com/tauri-apps/plugins-workspace) - Tauri plugin for shell operations

#### Development Dependencies
- [typescript](https://www.typescriptlang.org/) - TypeScript is a typed superset of JavaScript that compiles to plain JavaScript
- [vite](https://vitejs.dev/) - Next generation frontend tooling
- [@vitejs/plugin-react](https://github.com/vitejs/vite-plugin-react) - Vite plugin for React projects
- [@tauri-apps/cli](https://github.com/tauri-apps/tauri) - Tauri Command Line Interface
- [@types/react](https://www.npmjs.com/package/@types/react) - Type definitions for React
- [@types/react-dom](https://www.npmjs.com/package/@types/react-dom) - Type definitions for ReactDOM
