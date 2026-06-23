use serde::{Deserialize, Serialize};

/// Supported proxy protocols
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Protocol {
    Vless,
    Vmess,
    Shadowsocks,
    Trojan,
    Hysteria2,
    Tuic,
    Ssh,
    WireGuard,
    Tun,
    Custom,
}

/// Transport type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Transport {
    Tcp,
    Kcp,
    Ws,
    Grpc,
    Http,
    Quic,
    XHttp,
    HttpUpgrade,
}

/// TLS settings
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TlsSettings {
    pub enabled: bool,
    pub server_name: Option<String>,
    pub insecure: bool,
    pub alpn: Vec<String>,
    pub fingerprint: Option<String>,
    pub reality: Option<RealitySettings>,
    // Additional TLS fields
    pub disable_sni: bool,
    pub min_version: Option<String>,
    pub max_version: Option<String>,
    pub cipher_suites: Vec<String>,
    pub curve_preferences: Vec<String>,
    pub certificate: Option<String>,
    pub certificate_path: Option<String>,
    pub certificate_public_key_sha256: Vec<String>,
    pub client_certificate: Option<String>,
    pub client_certificate_path: Option<String>,
    pub client_key: Option<String>,
    pub client_key_path: Option<String>,
    pub utls_enabled: bool,
}

/// SSH settings
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SshSettings {
    pub user: Option<String>,
    pub password: Option<String>,
    pub private_key: Option<String>,
    pub private_key_path: Option<String>,
    pub private_key_passphrase: Option<String>,
    pub host_key: Option<Vec<String>>,
    pub host_key_algorithms: Vec<String>,
    pub client_version: Option<String>,
}

/// WireGuard Peer settings
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WireGuardPeer {
    pub address: String,
    pub port: Option<u16>,
    pub public_key: String,
    pub pre_shared_key: Option<String>,
    pub allowed_ips: Vec<String>,
    pub persistent_keepalive_interval: Option<u32>,
    pub reserved: Option<Vec<u8>>,
}

/// WireGuard settings
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WireGuardSettings {
    pub system: Option<bool>,
    pub name: Option<String>,
    pub mtu: Option<u32>,
    pub address: Vec<String>,
    pub private_key: String,
    pub listen_port: Option<u16>,
    pub peers: Vec<WireGuardPeer>,
    pub udp_timeout: Option<String>,
    pub workers: Option<u32>,
}

/// TUN platform HTTP proxy settings
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TunHttpProxy {
    pub enabled: Option<bool>,
    pub server: Option<String>,
    pub server_port: Option<u16>,
    pub bypass_domain: Option<Vec<String>>,
    pub match_domain: Option<Vec<String>>,
}

/// TUN platform settings
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TunPlatform {
    pub http_proxy: Option<TunHttpProxy>,
}

/// TUN settings
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TunSettings {
    pub interface_name: Option<String>,
    pub address: Vec<String>,
    pub mtu: Option<u32>,
    pub auto_route: Option<bool>,
    pub iproute2_table_index: Option<u32>,
    pub iproute2_rule_index: Option<u32>,
    pub auto_redirect: Option<bool>,
    pub auto_redirect_input_mark: Option<String>,
    pub auto_redirect_output_mark: Option<String>,
    pub auto_redirect_reset_mark: Option<String>,
    pub auto_redirect_nfqueue: Option<u32>,
    pub auto_redirect_iproute2_fallback_rule_index: Option<u32>,
    pub exclude_mptcp: Option<bool>,
    pub loopback_address: Vec<String>,
    pub strict_route: Option<bool>,
    pub route_address: Vec<String>,
    pub route_exclude_address: Vec<String>,
    pub route_address_set: Vec<String>,
    pub route_exclude_address_set: Vec<String>,
    pub endpoint_independent_nat: Option<bool>,
    pub udp_timeout: Option<String>,
    pub stack: Option<String>,
    pub include_interface: Vec<String>,
    pub exclude_interface: Vec<String>,
    pub include_uid: Vec<u32>,
    pub include_uid_range: Vec<String>,
    pub exclude_uid: Vec<u32>,
    pub exclude_uid_range: Vec<String>,
    pub include_android_user: Vec<i32>,
    pub include_package: Vec<String>,
    pub exclude_package: Vec<String>,
    pub platform: Option<TunPlatform>,
    // Deprecated fields (included for completeness but typically not used in new format)
    pub inet4_address: Vec<String>,
    pub inet6_address: Vec<String>,
    pub inet4_route_address: Vec<String>,
    pub inet6_route_address: Vec<String>,
    pub inet4_route_exclude_address: Vec<String>,
    pub inet6_route_exclude_address: Vec<String>,
}

/// VLESS Reality settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealitySettings {
    pub public_key: String,
    pub short_id: String,
}

/// WebSocket settings
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WsSettings {
    pub path: String,
    pub host: Option<String>,
}

/// gRPC settings
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GrpcSettings {
    pub service_name: String,
}

/// XHTTP settings
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct XHttpSettings {
    pub host: Option<String>,
    pub path: String,
    pub mode: String,
}

/// HTTP Upgrade settings
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HttpUpgradeSettings {
    pub host: Option<String>,
    pub path: String,
}

/// KCP settings
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct KcpSettings {
    pub header_type: String,
    pub seed: Option<String>,
}

/// QUIC settings
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QuicSettings {
    pub header_type: String,
    pub quic_security: String,
    pub key: String,
}

/// A single proxy server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Server {
    pub id: String,
    pub name: String,
    pub address: String,
    pub port: u16,
    pub protocol: Protocol,

    // Protocol-specific
    pub uuid: Option<String>,       // vless, vmess
    pub password: Option<String>,   // ss, trojan
    pub method: Option<String>,     // ss encryption method
    pub flow: Option<String>,       // vless flow (xtls-rprx-vision)
    pub alter_id: Option<u32>,      // vmess
    pub ssh_settings: Option<SshSettings>, // ssh
    pub wireguard_settings: Option<WireGuardSettings>, // wireguard
    pub tun_settings: Option<TunSettings>, // tun

    // Transport
    pub transport: Transport,
    pub ws: Option<WsSettings>,
    pub grpc: Option<GrpcSettings>,
    pub xhttp: Option<XHttpSettings>,
    pub httpupgrade: Option<HttpUpgradeSettings>,
    pub kcp: Option<KcpSettings>,
    pub quic: Option<QuicSettings>,

    // TLS
    pub tls: TlsSettings,

    // Metadata
    pub subscription_id: Option<String>,
    pub latency_ms: Option<u32>,
    
    // For Custom protocol (JSON configs)
    pub json_config: Option<String>,
}

/// A subscription source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subscription {
    pub id: String,
    pub name: String,
    pub url: String,
    pub servers: Vec<String>, // server IDs
    pub updated_at: Option<u64>,
}

/// User settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub theme: String,
    #[serde(default = "default_style")]
    pub style: String,
    pub socks_port: u16,
    pub http_port: u16,
    pub auto_connect: bool,
    pub language: String,
    #[serde(default = "default_vpn_mode")]
    pub vpn_mode: String, // "proxy" or "tun"
    #[serde(default)]
    pub auto_reconnect: bool,
    #[serde(default = "default_true")]
    pub hwid_enabled: bool,
    #[serde(default = "default_animation")]
    pub animation: String,
}

fn default_style() -> String { "default".to_string() }
fn default_vpn_mode() -> String { "proxy".to_string() }
fn default_true() -> bool { true }
fn default_animation() -> String { "smooth".to_string() }

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: "dark".to_string(),
            style: "default".to_string(),
            socks_port: 10808,
            http_port: 10809,
            auto_connect: false,
            language: "en".to_string(),
            vpn_mode: "proxy".to_string(),
            auto_reconnect: false,
            hwid_enabled: true,
            animation: "smooth".to_string(),
        }
    }
}

/// Action for a routing rule
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum RuleAction {
    #[default]
    Proxy,
    Direct,
    Block,
    /// Route matching traffic into an externally-managed interface (e.g. an
    /// AmneziaWG tunnel brought up with `table = off`). sing-box only.
    Bridge,
}

/// A user-defined routing rule (domain → action)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingRule {
    pub id: String,
    pub domain: String,
    pub action: RuleAction,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

/// A recorded connection session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionRecord {
    pub server_name: String,
    pub server_address: String,
    pub protocol: String,
    pub core_type: String,
    pub vpn_mode: String,
    pub connected_at: u64,
    pub disconnected_at: Option<u64>,
    pub upload_bytes: u64,
    pub download_bytes: u64,
}

/// Configuration for routing into an externally-managed network interface (the
/// "bridge" outbound). The interface itself is created/owned outside IRBox.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BridgeConfig {
    /// Network interface to bind the bridge outbound to, e.g. "awg0".
    #[serde(default)]
    pub interface: Option<String>,
    /// Optional SO_MARK / fwmark to tag bridge egress (Linux).
    #[serde(default)]
    pub routing_mark: Option<u32>,
    /// Interface server endpoint IP/CIDRs kept on `direct` to avoid a routing
    /// loop in TUN mode. A list — supports multi-peer tunnels.
    #[serde(default)]
    pub endpoints: Vec<String>,
}

/// A named, externally-managed network interface IRBox can route into.
/// The interface itself is created/owned outside IRBox.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct InterfaceConfig {
    /// Stable uuid (minted backend-side; empty on a new-add request).
    pub id: String,
    /// User-facing name, e.g. "Work AWG". Defaults to `interface` if blank.
    pub label: String,
    /// Bind target, e.g. "awg0". Required, non-empty.
    pub interface: String,
    /// Optional SO_MARK / fwmark to tag bridge egress (Linux).
    #[serde(default)]
    pub routing_mark: Option<u32>,
    /// Interface server endpoint IP/CIDRs kept on `direct` (anti-loop in TUN).
    #[serde(default)]
    pub endpoints: Vec<String>,
}

/// App state persisted to disk
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppState {
    pub servers: Vec<Server>,
    pub subscriptions: Vec<Subscription>,
    pub active_server_id: Option<String>,
    pub selected_core: CoreType,
    #[serde(default)]
    pub settings: Settings,
    #[serde(default)]
    pub sessions: Vec<ConnectionRecord>,
    #[serde(default)]
    pub routing_rules: Vec<RoutingRule>,
    #[serde(default = "default_route")]
    pub default_route: String,
    #[serde(default)]
    pub bridge: BridgeConfig,
    #[serde(default)]
    pub interfaces: Vec<InterfaceConfig>,
    #[serde(default)]
    pub active_interface_id: Option<String>,
    #[serde(default)]
    pub onboarding_completed: bool,
}

fn default_route() -> String { "proxy".to_string() }

impl AppState {
    /// Resolve the active interface, treating a dangling id as none.
    pub fn active_interface(&self) -> Option<&InterfaceConfig> {
        let id = self.active_interface_id.as_ref()?;
        self.interfaces.iter().find(|i| &i.id == id)
    }

    /// Dual-write: keep the legacy `bridge` field populated from the active
    /// interface so a downgrade to v1.1.0 still finds a usable config.
    pub fn sync_legacy_bridge(&mut self) {
        self.bridge = match self.active_interface() {
            Some(i) => BridgeConfig {
                interface: Some(i.interface.clone()),
                routing_mark: i.routing_mark,
                endpoints: i.endpoints.clone(),
            },
            None => BridgeConfig::default(),
        };
    }

    /// One-shot migration from the v1.1.0 single `bridge` field to the
    /// interfaces list. Runs only when no interfaces exist yet and the legacy
    /// bridge has a non-empty interface.
    pub fn migrate_bridge_to_interfaces(&mut self) {
        if !self.interfaces.is_empty() {
            return;
        }
        if let Some(iface) = self.bridge.interface.clone().filter(|s| !s.is_empty()) {
            let entry = InterfaceConfig {
                id: uuid::Uuid::new_v4().to_string(),
                label: iface.clone(),
                interface: iface,
                routing_mark: self.bridge.routing_mark,
                endpoints: self.bridge.endpoints.clone(),
            };
            self.active_interface_id = Some(entry.id.clone());
            self.interfaces.push(entry);
        }
    }
}

/// Which proxy core to use
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum CoreType {
    #[default]
    SingBox,
    Xray,
}

/// Traffic statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TrafficStats {
    pub upload: u64,
    pub download: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bridge_config_defaults_are_empty() {
        let b = BridgeConfig::default();
        assert!(b.interface.is_none());
        assert!(b.routing_mark.is_none());
        assert!(b.endpoints.is_empty());
    }

    #[test]
    fn bridge_config_serde_roundtrip() {
        let b = BridgeConfig {
            interface: Some("awg0".to_string()),
            routing_mark: Some(51820),
            endpoints: vec!["192.0.2.1/32".to_string(), "198.51.100.7".to_string()],
        };
        let json = serde_json::to_string(&b).unwrap();
        let back: BridgeConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back.interface.as_deref(), Some("awg0"));
        assert_eq!(back.routing_mark, Some(51820));
        assert_eq!(back.endpoints, vec!["192.0.2.1/32", "198.51.100.7"]);
    }

    #[test]
    fn appstate_default_has_empty_bridge() {
        let s = AppState::default();
        assert!(s.bridge.interface.is_none());
    }

    #[test]
    fn migrate_v110_bridge_creates_one_active_interface() {
        let mut s = AppState {
            bridge: BridgeConfig {
                interface: Some("awg0".into()),
                routing_mark: Some(51820),
                endpoints: vec!["192.0.2.1/32".into()],
            },
            ..Default::default()
        };
        s.migrate_bridge_to_interfaces();
        assert_eq!(s.interfaces.len(), 1);
        let i = &s.interfaces[0];
        assert_eq!(i.interface, "awg0");
        assert_eq!(i.label, "awg0");
        assert_eq!(i.routing_mark, Some(51820));
        assert_eq!(i.endpoints, vec!["192.0.2.1/32".to_string()]);
        assert_eq!(s.active_interface_id.as_deref(), Some(i.id.as_str()));
        assert!(!i.id.is_empty());
    }

    #[test]
    fn migrate_bridge_without_interface_creates_zero_interfaces() {
        let mut s = AppState {
            bridge: BridgeConfig {
                interface: None,
                routing_mark: Some(7),
                endpoints: vec!["198.51.100.7".into()],
            },
            ..Default::default()
        };
        s.migrate_bridge_to_interfaces();
        assert!(s.interfaces.is_empty());
        assert!(s.active_interface_id.is_none());
    }

    #[test]
    fn migrate_is_noop_when_interfaces_already_present() {
        let mut s = AppState {
            interfaces: vec![InterfaceConfig {
                id: "keep".into(), label: "keep".into(), interface: "wg9".into(),
                routing_mark: None, endpoints: vec![],
            }],
            bridge: BridgeConfig { interface: Some("awg0".into()), ..Default::default() },
            ..Default::default()
        };
        s.migrate_bridge_to_interfaces();
        assert_eq!(s.interfaces.len(), 1);
        assert_eq!(s.interfaces[0].id, "keep");
    }

    #[test]
    fn active_interface_resolves_and_dangling_is_none() {
        let mut s = AppState {
            interfaces: vec![InterfaceConfig {
                id: "a".into(), label: "A".into(), interface: "awg0".into(),
                routing_mark: None, endpoints: vec![],
            }],
            active_interface_id: Some("a".into()),
            ..Default::default()
        };
        assert_eq!(s.active_interface().map(|i| i.id.as_str()), Some("a"));
        s.active_interface_id = Some("ghost".into());
        assert!(s.active_interface().is_none());
    }

    #[test]
    fn sync_legacy_bridge_mirrors_active_else_default() {
        let mut s = AppState {
            interfaces: vec![InterfaceConfig {
                id: "a".into(), label: "A".into(), interface: "awg0".into(),
                routing_mark: Some(51820), endpoints: vec!["192.0.2.1/32".into()],
            }],
            active_interface_id: Some("a".into()),
            ..Default::default()
        };
        s.sync_legacy_bridge();
        assert_eq!(s.bridge.interface.as_deref(), Some("awg0"));
        assert_eq!(s.bridge.routing_mark, Some(51820));
        assert_eq!(s.bridge.endpoints, vec!["192.0.2.1/32".to_string()]);
        s.active_interface_id = None;
        s.sync_legacy_bridge();
        assert!(s.bridge.interface.is_none());
        assert!(s.bridge.endpoints.is_empty());
    }
}
