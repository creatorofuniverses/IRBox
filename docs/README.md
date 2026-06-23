# IRBox Documentation

<p align="center">
  <strong>A modern, cross-platform proxy client with advanced protocol support</strong>
</p>

<p align="center">
  <a href="#system-requirements">System Requirements</a> •
  <a href="#configuration">Configuration</a> •
  <a href="#protocols">Supported Protocols</a>
</p>

---

## 🚀 Overview

IRBox is a powerful, cross-platform proxy client built with Tauri and Rust. It supports multiple proxy protocols including VLESS, VMess, Shadowsocks, Trojan, Hysteria2, TUIC, SSH, and WireGuard through both Sing-box and Xray cores.

### Key Features

- **Multi-Protocol Support**: VLESS, VMess, Shadowsocks, Trojan, Hysteria2, TUIC, SSH, WireGuard
- **Dual Core Engine**: Sing-box and Xray support
- **Cross-Platform**: Windows, macOS, Linux
- **TUN Mode**: System-wide traffic routing
- **Advanced TLS**: Reality, custom certificates, and advanced security features
- **User-Friendly Interface**: Modern GUI with theme support

---

## 🖥️ System Requirements

### Minimum Requirements

- **Windows**: Windows 10 or later
- **macOS**: macOS 10.15 (Catalina) or later
- **Linux**: glibc 2.17 or later

### Recommended Specifications

- **RAM**: 512MB minimum, 1GB recommended
- **Storage**: 100MB available space
- **Network**: Stable internet connection

### TUN Mode Requirements

For TUN mode (system-wide proxy), additional privileges are required:

- **Windows**: Administrator privileges
- **macOS**: Administrator access via sudo or system preferences
- **Linux**: root privileges or appropriate capabilities

---

## 🔧 Configuration

### Enabling TUN Mode (Routes All Traffic)

When switching to **TUN Mode**, IRBox needs elevated system privileges to manage network traffic.

#### Method 1 — Grant Admin Access from Inside the App

1. Launch **IRBox**
2. Navigate to **Settings → VPN Mode**
3. Choose **TUN**
4. Click **Run as Administrator**

#### Method 2 — Start via Terminal

```bash
# macOS
sudo /Applications/IRBox.app/Contents/MacOS/IRBox
```

### Configuration File Locations

- **Windows**: `%APPDATA%/irbox/`
- **macOS**: `~/Library/Application Support/irbox/`
- **Linux**: `~/.config/irbox/`

### Basic Configuration

IRBox supports various proxy link formats:

```
# VLESS
vless://uuid@server:port?security=tls&sni=domain.com#Remark

# VMess
vmess://base64-encoded-config

# Shadowsocks
ss://method:password@server:port#Remark

# Trojan
trojan://password@server:port#Remark

# TUIC
tuic://uuid:password@server:port?congestion_control=bbr&udp_relay_mode=native#Remark

# SSH
ssh://user:password@server:port/?private_key_path=/path/to/key&private_key_passphrase=passphrase&client_version=SSH-2.0-OpenSSH_8.9&host_key=AAAAB3NzaC1yc2E&host_key_algorithms=rsa-sha2-256,rsa-sha2-512#Remark

# WireGuard
wg://private_key@server:port/?public_key=pubkey&allowed_ips=0.0.0.0/0#Remark

# Alternative WireGuard format
wireguard://private_key@server:port/?public_key=pubkey&allowed_ips=0.0.0.0/0#Remark

# Hysteria2
hy2://password@server:port/?upmbps=100&downmbps=200&obfs=salamander&sni=example.com#Remark

# Alternative Hysteria2 format
hysteria2://password@server:port/?upmbps=100&downmbps=200&obfs=salamander&sni=example.com#Remark
```

---

## Custom Interface Routing

IRBox can route selected domains into an externally-managed interface (e.g. an
AmneziaWG tunnel). IRBox does not manage the interface lifecycle.

**OS-side setup (Linux):** bring the tunnel up with `table = off` so the kernel
doesn't install a catch-all route, and (optionally) set a `fwmark` so IRBox can
tag the bridge egress. The interface must already exist and be up.

**In IRBox:**
- **Interfaces page** — add one or more named interfaces and mark one **active**.
- **Routing page** — give rules the **Interface** action to route into the
  active interface.

**Anti-loop endpoints:** in TUN mode, list the tunnel server's IP/CIDRs as
endpoints so that traffic to the tunnel server itself stays `direct` and doesn't
loop back into the tunnel.

Only the **active** interface gets a bridge outbound; switching the active
interface (or editing it while connected) reconnects the core to apply it.

**Interface-only mode:** with an active interface and no proxy server selected,
Connect starts sing-box with no proxy outbound and a `direct` default route —
only `Interface`-action rules are sent into the interface, everything else stays
direct. This mode always uses the sing-box core (xray/custom cannot route into a
bridge outbound). Removing or deactivating the active interface while connected
stops the core.

## 🌐 Supported Protocols

### Core Protocols

| Protocol | Sing-box | Xray | Notes |
|----------|----------|------|-------|
| VLESS | ✅ | ✅ | With flow control support |
| VMess | ✅ | ✅ | Standard and security variants |
| Shadowsocks | ✅ | ✅ | Multiple encryption methods |
| Trojan | ✅ | ✅ | Standard implementation |
| Hysteria2 | ✅ | ❌ | Sing-box only |
| TUIC | ✅ | ❌ | Sing-box only |
| SSH | ✅ | ❌ | Sing-box only with full parameter support |
| WireGuard | ✅ | ❌ | Sing-box only |

### SSH Configuration Parameters

| Parameter | Type | Description | Example |
|-----------|------|-------------|---------|
| `user` | String | Username for SSH authentication | `user=john` |
| `password` | String | Password for SSH authentication | `password=mypassword` |
| `private_key` | String | Inline private key content | `private_key=-----BEGIN OPENSSH PRIVATE KEY-----...` |
| `private_key_path` | String | Path to private key file | `private_key_path=/home/user/.ssh/id_rsa` |
| `private_key_passphrase` | String | Passphrase for encrypted private key | `private_key_passphrase=keypass` |
| `host_key` | String | Expected server host key | `host_key=AAAAB3NzaC1yc2E...` |
| `host_key_algorithms` | String | Comma-separated list of allowed algorithms | `host_key_algorithms=rsa-sha2-256,rsa-sha2-512` |
| `client_version` | String | Custom SSH client version string | `client_version=SSH-2.0-MyClient` |

### SSH Examples

```bash
# Basic SSH connection
ssh://myuser:mypass@example.com:22#SSH-Server

# SSH with private key
ssh://myuser@example.com:22/?private_key_path=/home/user/.ssh/id_rsa#SSH-Key-Auth

# SSH with multiple host key algorithms
ssh://admin@example.com:2222/?private_key_path=/path/to/key&host_key_algorithms=rsa-sha2-256,rsa-sha2-512&client_version=SSH-2.0-Custom#Secure-SSH

# SSH with passphrase protected key
ssh://user@example.com:22/?private_key_path=/secure/key&private_key_passphrase=secret123#Protected-Key-SSH
```

### WireGuard Configuration Parameters

| Parameter | Type | Description | Example |
|-----------|------|-------------|---------|
| `private_key` | String | Client's private key | `private_key=CLIENT_PRIVATE_KEY` |
| `public_key` | String | Server's public key | `public_key=SERVER_PUBLIC_KEY` |
| `allowed_ips` | String | IPs to route through tunnel | `allowed_ips=0.0.0.0/0` |
| `address` | String | Local tunnel IP address(es) | `address=192.168.1.2` |
| `pre_shared_key` | String | Pre-shared key for additional security | `pre_shared_key=PRESHARED_KEY` |
| `persistent_keepalive_interval` | Integer | Keepalive interval in seconds | `persistent_keepalive_interval=25` |
| `mtu` | Integer | Maximum transmission unit | `mtu=1420` |
| `system` | Boolean | Use system interface | `system=true` |
| `name` | String | Interface name | `name=wg0` |
| `listen_port` | Integer | Local listening port | `listen_port=12345` |
| `udp_timeout` | String | UDP idle timeout | `udp_timeout=300s` |
| `workers` | Integer | Number of worker threads | `workers=4` |
| `reserved` | String | Reserved bytes (comma-separated) | `reserved=0,0,0` |

**Note**: In sing-box WireGuard configuration, peer objects do not include `address` or `port` fields. The server endpoint is configured using the URL host and port (`server.com:port` in the URL). The `address` parameter in the URL query string refers to the local tunnel IP address(es) assigned to the WireGuard interface.

### WireGuard Examples

```bash
# Basic WireGuard connection
wg://CLIENT_PRIVATE_KEY@server.com:51820/?public_key=SERVER_PUBLIC_KEY&allowed_ips=0.0.0.0/0#WG-Server

# WireGuard with additional parameters
wg://KEY@server.com:51820/?public_key=SERVER_KEY&allowed_ips=10.0.0.0/24&mtu=1280&persistent_keepalive_interval=25#Secure-WG

# WireGuard with system interface
wg://KEY@server.com:51820/?public_key=SERVER_KEY&allowed_ips=0.0.0.0/0&system=true&name=mywg0#System-WG
```

### Hysteria2 Configuration Parameters

| Parameter | Type | Description | Example |
|-----------|------|-------------|---------|
| `password` | String | Authentication password | `password=auth_password` |
| `upmbps` | Integer | Upload speed limit (Mbps) | `upmbps=100` |
| `downmbps` | Integer | Download speed limit (Mbps) | `downmbps=200` |
| `obfs` | String | Obfuscation type | `obfs=salamander` |
| `obfs-password` | String | Obfuscation password | `obfs-password=obfs_password` |
| `sni` | String | Server name indication | `sni=example.com` |
| `insecure` | Boolean | Allow insecure connections | `insecure=1` |
| `alpn` | String | Application-Layer Protocol Negotiation | `alpn=h3` |
| `fp` | String | Fingerprint for uTLS | `fp=chrome` |

### Hysteria2 Examples

```bash
# Basic Hysteria2 connection
hy2://password@server.com:443#HY2-Server

# Hysteria2 with speed limits
hy2://password@server.com:443/?upmbps=100&downmbps=200&sni=example.com#HY2-Speed-Limited

# Hysteria2 with obfuscation
hy2://password@server.com:443/?upmbps=50&downmbps=100&obfs=salamander&obfs-password=obfspass#HY2-Obfuscated
```

### Transport Types

- TCP (default)
- WebSocket (ws)
- gRPC
- HTTP/2
- QUIC
- KCP
- XHTTP
- HTTPUpgrade

### Security Features

- **TLS**: Standard TLS with custom certificates
- **Reality**: XTLS-Reality protocol support
- **UTLS**: Fingerprint spoofing
- **Custom Certificates**: User-provided certificate support
- **Advanced TLS Options**: SNI control, version control, cipher suites

### Advanced Features

- **TUN Interface**: System-wide traffic routing
- **Routing Rules**: Domain and IP-based routing
- **Load Balancing**: Multiple server support
- **Subscription Management**: Automatic config updates
- **Statistics**: Real-time traffic monitoring
- **Multiple Themes**: Dark/light mode support

---

## 🛠️ Development

### Building from Source

```bash
# Clone the repository
git clone https://github.com/creatorofuniverses/IRBox.git
cd IRBox

# Install dependencies (also provides the Tauri CLI via @tauri-apps/cli)
npm install

# Download core executables
# On Linux/macOS:
chmod +x cores.sh
./cores.sh

# On Windows:
./cores.bat

# Run in development mode (hot reload)
npm run tauri dev

# Build release installers for the current platform
npm run tauri build
```

Built installers land in `src-tauri/target/release/bundle/`. See the main [README](../README.md#-build-from-source) for per-OS install steps and how releases are produced.

### Prerequisites

- Rust and Cargo
- NodeJS and NPM (Node 18+)
- Tauri CLI — comes from the pinned `@tauri-apps/cli` dev-dependency (installed by `npm install`); run via `npm run tauri`. No separate `cargo install tauri-cli` needed.
- Tauri platform prerequisites — see <https://v2.tauri.app/start/prerequisites/>

### Project Structure

```
IRBox/
├── src/                 # Frontend React components
├── src-tauri/          # Tauri backend (Rust)
│   ├── pcores/         # Proxy core executables (sing-box, xray)
│   └── src/            # Rust source code
├── docs/               # Documentation
├── .github/workflows/  # CI/CD workflows
├── cores.sh            # Linux/macOS core downloader
├── cores.bat           # Windows core downloader
├── package.json        # Node.js dependencies
└── README.md          # Main documentation
```

---

## 🤝 Contributing

Contributions are welcome! Please feel free to submit pull requests, report bugs, or suggest new features.

### Development Setup

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Test thoroughly
5. Submit a pull request

### Reporting Issues

Please include the following information when reporting issues:

- Operating system and version
- IRBox version
- Steps to reproduce
- Expected vs actual behavior
- Relevant configuration (remove sensitive data)

---

## 📄 License

This project is licensed under the MIT License. See the [LICENSE](../LICENSE) file for details.

---

## 🆘 Support

For support, please:

- Check the [FAQ](#faq) section below
- Open an issue on [GitHub](https://github.com/frank-vpl/IRBox/issues)
- Refer to the build workflow in `.github/workflows/build.yaml` for troubleshooting

### FAQ

**Q: Why does TUN mode require administrator privileges?**

A: TUN mode creates a virtual network interface that requires system-level permissions to manage network traffic routing.

**Q: Can I use both Sing-box and Xray cores?**

A: Yes, you can switch between cores in the settings. Some protocols are only available in specific cores.

**Q: How do I update my configuration?**

A: IRBox supports subscription links that automatically update your configuration. You can also manually add individual proxy links.

**Q: What are the core executables in pcores/ directory?**

A: The `pcores/` directory contains the sing-box and xray-core executables that IRBox uses for proxy functionality. These are automatically downloaded during the build process via `cores.sh` or `cores.bat`.

**Q: How do I build IRBox from source?**

A: Follow the development setup instructions above. Make sure to run the core download scripts (`cores.sh` or `cores.bat`) before building to ensure all required executables are present.

**Q: Is my configuration secure?**

A: IRBox stores configuration locally and encrypts sensitive data. Always use trusted proxy sources and verify server certificates.

**Q: How do I import proxy servers?**

A: You can import servers in multiple ways:
- Paste individual proxy links (VLESS, VMess, Shadowsocks, etc.) in the import box
- Add subscription URLs that automatically fetch server lists
- Use deep links: `irbox://import/SUBSCRIPTION_URL`

**Q: What is the difference between Proxy mode and TUN mode?**

A: 
- **Proxy mode**: Routes traffic through SOCKS/HTTP proxy ports (10808/10809 by default). Applications must be configured to use these proxies.
- **TUN mode**: Creates a virtual network interface that routes all system traffic automatically. Requires administrator privileges.

**Q: How does auto-reconnect work?**

A: When enabled in Settings, IRBox will automatically attempt to reconnect to your selected server if the connection drops unexpectedly. This is useful for maintaining stable connectivity.

**Q: Can I customize the proxy ports?**

A: Yes, you can change the default SOCKS (10808) and HTTP (10809) ports in Settings. Make sure to use ports between 1-65535 and avoid conflicts with other applications.

**Q: How do I check server latency?**

A: Use the "Ping All" button in the server list to test connectivity and latency to all your servers. Results are displayed in milliseconds for each server.

**Q: What is the auto-select feature?**

A: The auto-select function automatically chooses the server with the lowest latency from your list, helping you connect to the fastest available server.

**Q: How does routing work?**

A: IRBox supports domain-based routing with three actions:
- **Proxy**: Route through your selected proxy server
- **Direct**: Connect directly without proxy
- **Block**: Block connections to specified domains

You can set a default route (proxy all or direct all) and create custom rules for specific domains.

**Q: What statistics are available?**

A: The Stats page shows:
- Connection history with duration and traffic data
- Total sessions, upload/download statistics
- Real-time speed graph
- Session duration and data transfer per connection

**Q: How do I manage subscriptions?**

A: In the Subscriptions tab, you can:
- Add new subscription URLs
- Update existing subscriptions to get latest server lists
- Delete subscriptions you no longer need
- View server count and last update time

**Q: What information is in the logs?**

A: The Logs page displays real-time application logs including:
- Connection status changes
- Error messages
- Core operations (Sing-box/Xray)
- System events

You can filter logs, enable auto-scroll, and copy logs for troubleshooting.

**Q: How does HWID (Hardware ID) work?**

A: HWID is a unique identifier for your device that can be enabled in Settings. When enabled, it shows:
- Hardware ID for device identification
- Platform and OS version information
- Device model information

This can be useful for subscription management or device-specific configurations.

**Q: Can I change the application theme?**

A: Yes, IRBox supports multiple themes and customization options:
- Dark/Light theme switching
- Different UI styles (Default, Minimal)
- Animation preferences (None, Smooth, Energetic)
- Language settings (currently English)

**Q: What deep link functionality is supported?**

A: IRBox supports the `irbox://import/SUBSCRIPTION_URL` deep link format, allowing you to automatically add subscriptions by clicking links in browsers or other applications.

**Q: How do I export/import my configuration?**

A: Use the export/import functions to backup and restore your configuration:
- Export: Creates a backup of your current configuration
- Import: Restores configuration from a previously exported file

**Q: What is the onboarding process?**

A: First-time users go through an onboarding overlay that guides them through:
- Initial setup steps
- Basic feature introduction
- Configuration recommendations

**Q: How do I troubleshoot connection issues?**

A: Check these steps:
1. Verify server configuration and credentials
2. Check logs for error messages
3. Test server connectivity with ping
4. Ensure required ports are available
5. For TUN mode, verify administrator privileges
6. Try switching between Sing-box and Xray cores

**Q: What are the system resource requirements?**

A: IRBox is lightweight:
- RAM: 512MB minimum, 1GB recommended
- Storage: ~100MB for application and cores
- CPU: Minimal usage during normal operation
- Network: Bandwidth depends on your proxy usage

**Q: Can I run multiple instances?**

A: IRBox implements single-instance behavior to prevent conflicts. If you try to launch a second instance, it will focus the existing window instead.

---

<p align="center">
  <strong>Happy Proxying! 🚀</strong>
</p>