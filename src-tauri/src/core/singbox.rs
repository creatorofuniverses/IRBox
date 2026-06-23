use anyhow::Result;
use serde_json::{json, Value};

use crate::proxy::models::*;

/// Generate a sing-box config for connecting to a single server
pub fn generate_config(server: &Server, socks_port: u16, http_port: u16, tun_mode: bool, routing_rules: &[RoutingRule], default_route: &str, bridge: &BridgeConfig) -> Result<Value> {
    let outbound = build_outbound(server)?;
    
    // Handle WireGuard endpoints separately
    let endpoints = if server.protocol == Protocol::WireGuard {
        if let Some(ref wg_settings) = server.wireguard_settings {
            let mut wg_endpoint = json!({
                "type": "wireguard",
                "tag": "wg-ep",
            });
            
            // Add WireGuard specific settings
            if let Some(system) = wg_settings.system {
                wg_endpoint["system"] = json!(system);
            }
            if let Some(ref name) = wg_settings.name {
                wg_endpoint["name"] = json!(name);
            }
            if let Some(mtu) = wg_settings.mtu {
                wg_endpoint["mtu"] = json!(mtu);
            }
            if !wg_settings.address.is_empty() {
                wg_endpoint["address"] = json!(wg_settings.address);
            }
            wg_endpoint["private_key"] = json!(wg_settings.private_key);
            
            if let Some(listen_port) = wg_settings.listen_port {
                wg_endpoint["listen_port"] = json!(listen_port);
            }
            
            // Peers settings - now including address and port as required by new format
            let mut peers = Vec::new();
            for peer in &wg_settings.peers {
                let mut peer_obj = json!({
                    "public_key": peer.public_key,
                    "allowed_ips": peer.allowed_ips,
                });
                
                // Add address and port to peer as required by new endpoint format
                if !peer.address.is_empty() {
                    peer_obj["address"] = json!(peer.address);
                }
                if let Some(port) = peer.port {
                    peer_obj["port"] = json!(port);
                }
                
                if let Some(ref psk) = peer.pre_shared_key {
                    peer_obj["pre_shared_key"] = json!(psk);
                }
                if let Some(interval) = peer.persistent_keepalive_interval {
                    peer_obj["persistent_keepalive_interval"] = json!(interval);
                }
                if let Some(ref reserved) = peer.reserved {
                    peer_obj["reserved"] = json!(reserved);
                }
                peers.push(peer_obj);
            }
            wg_endpoint["peers"] = json!(peers);
            
            // Optional settings
            if let Some(ref timeout) = wg_settings.udp_timeout {
                wg_endpoint["udp_timeout"] = json!(timeout);
            }
            if let Some(workers) = wg_settings.workers {
                wg_endpoint["workers"] = json!(workers);
            }
            
            Some(vec![wg_endpoint])
        } else {
            None
        }
    } else {
        None
    };

    // On Android, TUN requires VpnService — force disable
    let tun_mode = if cfg!(target_os = "android") { false } else { tun_mode };

    let mut inbounds = vec![
        json!({
            "type": "socks",
            "tag": "socks-in",
            "listen": "127.0.0.1",
            "listen_port": socks_port
        }),
        json!({
            "type": "http",
            "tag": "http-in",
            "listen": "127.0.0.1",
            "listen_port": http_port
        }),
    ];

    if tun_mode {
        inbounds.push(json!({
            "type": "tun",
            "tag": "tun-in",
            "address": [
                "172.19.0.1/30",
                "fdfe:dcba:9876::1/126"
            ],
            "auto_route": true,
            "strict_route": false,
            "stack": "mixed",
            "endpoint_independent_nat": true,
            "mtu": 1420
        }));
    }

    // DNS config: TUN mode needs DoH + proper resolver chain to avoid loops
    // sing-box 1.12+ new DNS format: use type/server instead of address
    let dns = if tun_mode {
        json!({
            "servers": [
                {
                    "tag": "dns-remote",
                    "type": "https",
                    "server": "dns.google",
                    "server_port": 443,
                    "domain_resolver": "dns-direct",
                    "detour": "proxy"
                },
                {
                    "tag": "dns-direct",
                    "type": "udp",
                    "server": "8.8.8.8",
                    "server_port": 53
                }
            ],
            "rules": [
                { "query_type": [28, 32, 33], "action": "reject" },
                { "domain_suffix": [".lan"], "action": "reject" }
            ],
            "final": "dns-remote",
            "independent_cache": true,
            "disable_cache": false,
            "disable_expire": false
        })
    } else {
        json!({
            "servers": [
                {
                    "tag": "dns-local",
                    "type": "local"
                },
                {
                    "tag": "dns-remote",
                    "type": "udp",
                    "server": "8.8.8.8"
                }
            ],
            "final": "dns-remote",
            "disable_cache": false,
            "disable_expire": false
        })
    };
    
    // Route rules (sniff + DNS hijack, same approach as NekoRay)
    let mut route_rules: Vec<Value> = vec![
        json!({ "action": "sniff" }),
        json!({ "protocol": "dns", "action": "hijack-dns" }),
    ];

    if tun_mode {
        // Block multicast, NetBIOS, mDNS — they shouldn't go through proxy
        route_rules.push(json!({
            "network": "udp",
            "port": [135, 137, 138, 139, 5353],
            "action": "reject"
        }));
        route_rules.push(json!({
            "ip_cidr": ["224.0.0.0/3", "ff00::/8"],
            "action": "reject"
        }));
        route_rules.push(json!({
            "source_ip_cidr": ["224.0.0.0/3", "ff00::/8"],
            "action": "reject"
        }));
    }

    // Anti-loop guard: keep the external tunnel's own endpoint traffic on
    // `direct` so its handshake/data is not captured back into sing-box (TUN).
    // Must precede user rules (first match wins).
    if bridge.interface.is_some() && !bridge.endpoints.is_empty() {
        route_rules.push(json!({ "ip_cidr": bridge.endpoints, "outbound": "direct" }));
    }

    // User-defined routing rules
    for rule in routing_rules.iter().filter(|r| r.enabled) {
        let domain = &rule.domain;
        match rule.action {
            RuleAction::Direct => {
                route_rules.push(json!({ "domain_suffix": [domain], "outbound": "direct" }));
            }
            RuleAction::Block => {
                route_rules.push(json!({ "domain_suffix": [domain], "action": "reject" }));
            }
            RuleAction::Proxy => {
                route_rules.push(json!({ "domain_suffix": [domain], "outbound": "proxy" }));
            }
            RuleAction::Bridge => {
                // Falls back to `proxy` if no bridge interface is configured.
                let outbound = if bridge.interface.is_some() { "bridge" } else { "proxy" };
                route_rules.push(json!({ "domain_suffix": [domain], "outbound": outbound }));
            }
        }
    }

    let final_route = if default_route == "direct" { "direct" } else { "proxy" };

    let mut outbounds = vec![
        outbound,
        json!({ "type": "direct", "tag": "direct" }),
    ];
    if let Some(ref iface) = bridge.interface {
        let mut bridge_out = json!({
            "type": "direct",
            "tag": "bridge",
            "bind_interface": iface,
        });
        if let Some(mark) = bridge.routing_mark {
            bridge_out["routing_mark"] = json!(mark);
        }
        outbounds.push(bridge_out);
    }

    let mut config = json!({
        "log": {
            "level": if cfg!(target_os = "android") { "info" } else { "warn" },
            "timestamp": true
        },
        "dns": dns,
        "inbounds": inbounds,
        "outbounds": outbounds,
        "route": {
            "rules": route_rules,
            "final": final_route,
            "auto_detect_interface": !cfg!(target_os = "android"),
            "default_domain_resolver": {
                "server": if cfg!(target_os = "android") { "dns-remote" } else if tun_mode { "dns-direct" } else { "dns-local" }
            }
        },
        "experimental": {
            "clash_api": {
                "external_controller": "127.0.0.1:9090"
            }
        }
    });
    
    // Add endpoints array if present (for WireGuard)
    if let Some(endpts) = endpoints {
        config["endpoints"] = json!(endpts);
    }

    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_server() -> Server {
        Server {
            id: "t".into(), name: "t".into(), address: "192.0.2.10".into(), port: 443,
            protocol: Protocol::Shadowsocks,
            uuid: None, password: Some("pw".into()), method: Some("aes-256-gcm".into()),
            flow: None, alter_id: None, ssh_settings: None, wireguard_settings: None,
            tun_settings: None, transport: Transport::Tcp, ws: None, grpc: None,
            xhttp: None, httpupgrade: None, kcp: None, quic: None,
            tls: TlsSettings::default(), subscription_id: None, latency_ms: None,
            json_config: None,
        }
    }

    fn outbound_tags(cfg: &Value) -> Vec<String> {
        cfg["outbounds"].as_array().unwrap().iter()
            .filter_map(|o| o["tag"].as_str().map(|s| s.to_string())).collect()
    }

    #[test]
    fn no_bridge_outbound_when_interface_unset() {
        let cfg = generate_config(&test_server(), 1080, 1081, false, &[], "proxy",
            &BridgeConfig::default()).unwrap();
        assert!(!outbound_tags(&cfg).contains(&"bridge".to_string()));
    }

    #[test]
    fn bridge_outbound_emitted_with_bind_interface_and_mark() {
        let bridge = BridgeConfig {
            interface: Some("awg0".into()), routing_mark: Some(51820), endpoints: vec![],
        };
        let cfg = generate_config(&test_server(), 1080, 1081, false, &[], "proxy", &bridge).unwrap();
        let out = cfg["outbounds"].as_array().unwrap().iter()
            .find(|o| o["tag"] == "bridge").expect("bridge outbound present");
        assert_eq!(out["type"], "direct");
        assert_eq!(out["bind_interface"], "awg0");
        assert_eq!(out["routing_mark"], 51820);
    }

    #[test]
    fn bridge_outbound_omits_mark_when_unset() {
        let bridge = BridgeConfig { interface: Some("awg0".into()), routing_mark: None, endpoints: vec![] };
        let cfg = generate_config(&test_server(), 1080, 1081, false, &[], "proxy", &bridge).unwrap();
        let out = cfg["outbounds"].as_array().unwrap().iter()
            .find(|o| o["tag"] == "bridge").unwrap();
        assert!(out.get("routing_mark").is_none());
    }

    #[test]
    fn antiloop_rule_precedes_user_rules() {
        let bridge = BridgeConfig {
            interface: Some("awg0".into()), routing_mark: None,
            endpoints: vec!["192.0.2.1/32".into(), "198.51.100.7".into()],
        };
        let rules = vec![RoutingRule {
            id: "r1".into(), domain: "example.com".into(),
            action: RuleAction::Direct, enabled: true,
        }];
        let cfg = generate_config(&test_server(), 1080, 1081, false, &rules, "proxy", &bridge).unwrap();
        let route_rules = cfg["route"]["rules"].as_array().unwrap();
        let antiloop_idx = route_rules.iter().position(|r| r["ip_cidr"][0] == "192.0.2.1/32");
        let user_idx = route_rules.iter().position(|r| r["domain_suffix"][0] == "example.com");
        assert!(antiloop_idx.is_some(), "anti-loop rule present");
        assert!(user_idx.is_some(), "user rule present");
        assert!(antiloop_idx.unwrap() < user_idx.unwrap(), "anti-loop must precede user rules");
    }

    #[test]
    fn no_antiloop_rule_when_endpoints_empty() {
        let bridge = BridgeConfig { interface: Some("awg0".into()), routing_mark: None, endpoints: vec![] };
        let cfg = generate_config(&test_server(), 1080, 1081, false, &[], "proxy", &bridge).unwrap();
        let route_rules = cfg["route"]["rules"].as_array().unwrap();
        assert!(
            !route_rules.iter().any(|r| r["ip_cidr"].is_array() && r["outbound"] == "direct"),
            "no anti-loop ip_cidr->direct rule should be emitted when endpoints is empty"
        );
    }

    #[test]
    fn bridge_rule_routes_to_bridge_when_iface_set() {
        let bridge = BridgeConfig { interface: Some("awg0".into()), routing_mark: None, endpoints: vec![] };
        let rules = vec![RoutingRule {
            id: "r".into(), domain: "example.com".into(), action: RuleAction::Bridge, enabled: true,
        }];
        let cfg = generate_config(&test_server(), 1080, 1081, false, &rules, "proxy", &bridge).unwrap();
        let rule = cfg["route"]["rules"].as_array().unwrap().iter()
            .find(|r| r["domain_suffix"][0] == "example.com").unwrap();
        assert_eq!(rule["outbound"], "bridge");
    }

    #[test]
    fn bridge_rule_falls_back_to_proxy_when_iface_unset() {
        let rules = vec![RoutingRule {
            id: "r".into(), domain: "example.com".into(), action: RuleAction::Bridge, enabled: true,
        }];
        let cfg = generate_config(&test_server(), 1080, 1081, false, &rules, "proxy",
            &BridgeConfig::default()).unwrap();
        let rule = cfg["route"]["rules"].as_array().unwrap().iter()
            .find(|r| r["domain_suffix"][0] == "example.com").unwrap();
        assert_eq!(rule["outbound"], "proxy");
    }
}

fn build_outbound(server: &Server) -> Result<Value> {
    // For WireGuard, we return a placeholder since it goes in endpoints, not outbounds
    if server.protocol == Protocol::WireGuard {
        return Ok(json!({
            "type": "selector",
            "tag": "proxy",
            "outbounds": ["wg-ep"], // Reference to the WireGuard endpoint
            "default": "wg-ep"
        }));
    }

    let mut out = json!({
        "tag": "proxy",
        "server": server.address,
        "server_port": server.port,
        "udp_fragment": true,
    });

    match server.protocol {
        Protocol::Vless => {
            out["type"] = json!("vless");
            out["tag"] = json!("proxy");
            out["server"] = json!(server.address);
            out["server_port"] = json!(server.port);
            out["uuid"] = json!(server.uuid.as_deref().unwrap_or(""));
            if let Some(flow) = &server.flow {
                if !flow.is_empty() {
                    out["flow"] = json!(flow);
                }
            }
            out["network"] = json!("tcp");
            out["packet_encoding"] = json!("");
            out["multiplex"] = json!({});
        }
        Protocol::Vmess => {
            out["type"] = json!("vmess");
            out["tag"] = json!("proxy");
            out["uuid"] = json!(server.uuid.as_deref().unwrap_or(""));
            out["alter_id"] = json!(server.alter_id.unwrap_or(0));
            out["security"] = json!("auto");
        }
        Protocol::Shadowsocks => {
            out["type"] = json!("shadowsocks");
            out["tag"] = json!("proxy");
            out["method"] = json!(server.method.as_deref().unwrap_or("aes-256-gcm"));
            out["password"] = json!(server.password.as_deref().unwrap_or(""));
        }
        Protocol::Trojan => {
            out["type"] = json!("trojan");
            out["tag"] = json!("proxy");
            out["password"] = json!(server.password.as_deref().unwrap_or(""));
        }
        Protocol::Hysteria2 => {
            out["type"] = json!("hysteria2");
            out["tag"] = json!("proxy");
            out["password"] = json!(server.password.as_deref().unwrap_or(""));
            
            // Add Hysteria2 specific settings from server parameters
            if let Some(ref method) = server.method {
                if method != "none" {
                    out["obfs"] = json!({
                        "type": method,
                        "password": server.password.as_deref().unwrap_or("")
                    });
                }
            }
            
            // Extract bandwidth info from flow field if available
            if let Some(ref flow_info) = server.flow {
                if flow_info.contains("upmbps") && flow_info.contains("downmbps") {
                    // Parse bandwidth info from format: "upmbps:XX;downmbps:YY"
                    let parts: Vec<&str> = flow_info.split(';').collect();
                    for part in parts {
                        if part.starts_with("upmbps:") {
                            if let Ok(up_mbps) = part.strip_prefix("upmbps:").unwrap_or("0").parse::<u32>() {
                                if up_mbps > 0 {
                                    out["up_mbps"] = json!(up_mbps);
                                }
                            }
                        } else if part.starts_with("downmbps:") {
                            if let Ok(down_mbps) = part.strip_prefix("downmbps:").unwrap_or("0").parse::<u32>() {
                                if down_mbps > 0 {
                                    out["down_mbps"] = json!(down_mbps);
                                }
                            }
                        }
                    }
                }
            }
        }
        Protocol::Tuic => {
            out["type"] = json!("tuic");
            out["tag"] = json!("proxy");
            out["server"] = json!(server.address);
            out["server_port"] = json!(server.port);
            out["uuid"] = json!(server.uuid.as_deref().unwrap_or(""));
            
            // Password is optional in TUIC
            if let Some(ref password) = server.password {
                if !password.is_empty() {
                    out["password"] = json!(password);
                }
            }
            
            // Add TUIC specific settings from server parameters
            if let Some(ref method) = server.method {
                if method != "cubic" {
                    out["congestion_control"] = json!(method);
                }
            }
            
            // Parse additional TUIC settings from flow field
            if let Some(ref flow_info) = server.flow {
                // Try to parse the JSON string stored in flow field
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(flow_info) {
                    if let Some(cc) = parsed.get("congestion_control").and_then(|v| v.as_str()) {
                        if cc != "cubic" {
                            out["congestion_control"] = json!(cc);
                        }
                    }
                    if let Some(urm) = parsed.get("udp_relay_mode").and_then(|v| v.as_str()) {
                        if urm != "native" {
                            out["udp_relay_mode"] = json!(urm);
                        }
                    }
                    if let Some(uos) = parsed.get("udp_over_stream").and_then(|v| v.as_bool()) {
                        out["udp_over_stream"] = json!(uos);
                    }
                    if let Some(zrth) = parsed.get("zero_rtt_handshake").and_then(|v| v.as_bool()) {
                        out["zero_rtt_handshake"] = json!(zrth);
                    }
                    if let Some(hb) = parsed.get("heartbeat").and_then(|v| v.as_str()) {
                        if !hb.is_empty() {
                            out["heartbeat"] = json!(hb);
                        }
                    }
                    if let Some(net) = parsed.get("network").and_then(|v| v.as_str()) {
                        if net != "tcp,udp" {
                            out["network"] = json!(net);
                        }
                    }
                }
            }
        }
        Protocol::Ssh => {
            out["type"] = json!("ssh");
            out["tag"] = json!("proxy");
            out["server"] = json!(server.address);
            out["server_port"] = json!(server.port);
            
            // Add SSH specific settings from server parameters
            if let Some(ref ssh_settings) = server.ssh_settings {
                if let Some(ref user) = ssh_settings.user {
                    out["user"] = json!(user);
                }
                if let Some(ref password) = ssh_settings.password {
                    out["password"] = json!(password);
                }
                if let Some(ref private_key) = ssh_settings.private_key {
                    out["private_key"] = json!(private_key);
                }
                if let Some(ref private_key_path) = ssh_settings.private_key_path {
                    out["private_key_path"] = json!(private_key_path);
                }
                if let Some(ref private_key_passphrase) = ssh_settings.private_key_passphrase {
                    out["private_key_passphrase"] = json!(private_key_passphrase);
                }
                if let Some(ref host_key) = ssh_settings.host_key {
                    out["host_key"] = json!(host_key);
                }
                if !ssh_settings.host_key_algorithms.is_empty() {
                    out["host_key_algorithms"] = json!(ssh_settings.host_key_algorithms);
                }
                if let Some(ref client_version) = ssh_settings.client_version {
                    out["client_version"] = json!(client_version);
                }
            }
        }
        Protocol::Tun => {
            out["type"] = json!("tun");
            out["tag"] = json!("proxy");
            
            // Add TUN specific settings from server parameters
            if let Some(ref tun_settings) = server.tun_settings {
                // Basic settings
                if let Some(ref interface_name) = tun_settings.interface_name {
                    out["interface_name"] = json!(interface_name);
                }
                
                // Use new format address structure (combines inet4 and inet6 addresses)
                if !tun_settings.address.is_empty() {
                    out["address"] = json!(tun_settings.address);
                } else {
                    // Fallback to deprecated fields if new format address is empty
                    let mut combined_addresses = Vec::new();
                    combined_addresses.extend_from_slice(&tun_settings.inet4_address);
                    combined_addresses.extend_from_slice(&tun_settings.inet6_address);
                    if !combined_addresses.is_empty() {
                        out["address"] = json!(combined_addresses);
                    }
                }
                
                if let Some(mtu) = tun_settings.mtu {
                    out["mtu"] = json!(mtu);
                }
                if let Some(auto_route) = tun_settings.auto_route {
                    out["auto_route"] = json!(auto_route);
                }
                if let Some(iproute2_table_index) = tun_settings.iproute2_table_index {
                    out["iproute2_table_index"] = json!(iproute2_table_index);
                }
                if let Some(iproute2_rule_index) = tun_settings.iproute2_rule_index {
                    out["iproute2_rule_index"] = json!(iproute2_rule_index);
                }
                if let Some(auto_redirect) = tun_settings.auto_redirect {
                    out["auto_redirect"] = json!(auto_redirect);
                }
                if let Some(ref input_mark) = tun_settings.auto_redirect_input_mark {
                    out["auto_redirect_input_mark"] = json!(input_mark);
                }
                if let Some(ref output_mark) = tun_settings.auto_redirect_output_mark {
                    out["auto_redirect_output_mark"] = json!(output_mark);
                }
                if let Some(ref reset_mark) = tun_settings.auto_redirect_reset_mark {
                    out["auto_redirect_reset_mark"] = json!(reset_mark);
                }
                if let Some(auto_redirect_nfqueue) = tun_settings.auto_redirect_nfqueue {
                    out["auto_redirect_nfqueue"] = json!(auto_redirect_nfqueue);
                }
                if let Some(fallback_rule_index) = tun_settings.auto_redirect_iproute2_fallback_rule_index {
                    out["auto_redirect_iproute2_fallback_rule_index"] = json!(fallback_rule_index);
                }
                if let Some(exclude_mptcp) = tun_settings.exclude_mptcp {
                    out["exclude_mptcp"] = json!(exclude_mptcp);
                }
                if !tun_settings.loopback_address.is_empty() {
                    out["loopback_address"] = json!(tun_settings.loopback_address);
                }
                if let Some(strict_route) = tun_settings.strict_route {
                    out["strict_route"] = json!(strict_route);
                }
                
                // Use new format route_address (combines deprecated route fields)
                if !tun_settings.route_address.is_empty() {
                    out["route_address"] = json!(tun_settings.route_address);
                } else {
                    // Fallback to deprecated route fields if new format is empty
                    let mut combined_routes = Vec::new();
                    combined_routes.extend_from_slice(&tun_settings.inet4_route_address);
                    combined_routes.extend_from_slice(&tun_settings.inet6_route_address);
                    if !combined_routes.is_empty() {
                        out["route_address"] = json!(combined_routes);
                    }
                }
                
                // Use new format route_exclude_address (combines deprecated exclude fields)
                if !tun_settings.route_exclude_address.is_empty() {
                    out["route_exclude_address"] = json!(tun_settings.route_exclude_address);
                } else {
                    // Fallback to deprecated exclude fields if new format is empty
                    let mut combined_excludes = Vec::new();
                    combined_excludes.extend_from_slice(&tun_settings.inet4_route_exclude_address);
                    combined_excludes.extend_from_slice(&tun_settings.inet6_route_exclude_address);
                    if !combined_excludes.is_empty() {
                        out["route_exclude_address"] = json!(combined_excludes);
                    }
                }
                
                if !tun_settings.route_address_set.is_empty() {
                    out["route_address_set"] = json!(tun_settings.route_address_set);
                }
                if !tun_settings.route_exclude_address_set.is_empty() {
                    out["route_exclude_address_set"] = json!(tun_settings.route_exclude_address_set);
                }
                if let Some(endpoint_independent_nat) = tun_settings.endpoint_independent_nat {
                    out["endpoint_independent_nat"] = json!(endpoint_independent_nat);
                }
                if let Some(ref udp_timeout) = tun_settings.udp_timeout {
                    out["udp_timeout"] = json!(udp_timeout);
                }
                
                // Set mixed stack for better performance and compatibility
                if tun_settings.stack.is_none() {
                    out["stack"] = json!("mixed");
                } else {
                    out["stack"] = json!(tun_settings.stack);
                }
                
                if !tun_settings.include_interface.is_empty() {
                    out["include_interface"] = json!(tun_settings.include_interface);
                }
                if !tun_settings.exclude_interface.is_empty() {
                    out["exclude_interface"] = json!(tun_settings.exclude_interface);
                }
                if !tun_settings.include_uid.is_empty() {
                    out["include_uid"] = json!(tun_settings.include_uid);
                }
                if !tun_settings.include_uid_range.is_empty() {
                    out["include_uid_range"] = json!(tun_settings.include_uid_range);
                }
                if !tun_settings.exclude_uid.is_empty() {
                    out["exclude_uid"] = json!(tun_settings.exclude_uid);
                }
                if !tun_settings.exclude_uid_range.is_empty() {
                    out["exclude_uid_range"] = json!(tun_settings.exclude_uid_range);
                }
                if !tun_settings.include_android_user.is_empty() {
                    out["include_android_user"] = json!(tun_settings.include_android_user);
                }
                if !tun_settings.include_package.is_empty() {
                    out["include_package"] = json!(tun_settings.include_package);
                }
                if !tun_settings.exclude_package.is_empty() {
                    out["exclude_package"] = json!(tun_settings.exclude_package);
                }
                
                // Platform settings
                if let Some(ref platform) = tun_settings.platform {
                    if let Some(ref http_proxy) = platform.http_proxy {
                        let mut http_proxy_obj = serde_json::Map::new();
                        
                        if let Some(enabled) = http_proxy.enabled {
                            http_proxy_obj.insert("enabled".to_string(), serde_json::Value::Bool(enabled));
                        }
                        if let Some(ref server) = http_proxy.server {
                            http_proxy_obj.insert("server".to_string(), serde_json::Value::String(server.clone()));
                        }
                        if let Some(server_port) = http_proxy.server_port {
                            http_proxy_obj.insert("server_port".to_string(), serde_json::Value::Number(serde_json::Number::from(server_port)));
                        }
                        if let Some(ref bypass_domain) = http_proxy.bypass_domain {
                            http_proxy_obj.insert("bypass_domain".to_string(), serde_json::Value::Array(bypass_domain.iter().map(|s| serde_json::Value::String(s.clone())).collect()));
                        }
                        if let Some(ref match_domain) = http_proxy.match_domain {
                            http_proxy_obj.insert("match_domain".to_string(), serde_json::Value::Array(match_domain.iter().map(|s| serde_json::Value::String(s.clone())).collect()));
                        }
                        
                        let mut platform_map = serde_json::Map::new();
                        platform_map.insert("http_proxy".to_string(), serde_json::Value::Object(http_proxy_obj));
                        out["platform"] = serde_json::Value::Object(platform_map);
                    }
                }
            } else {
                // Default TUN settings for better performance
                out["address"] = json!(["172.19.0.1/30", "fdfe:dcba:9876::1/126"]);
                out["auto_route"] = json!(true);
                out["strict_route"] = json!(false);
                out["stack"] = json!("mixed");
                out["endpoint_independent_nat"] = json!(true);
            }
        }
        _ => {} // All other protocols handled above
    }

    // Transport
    match server.transport {
        Transport::Tcp => {
            // TCP is default, no transport settings needed
        }
        Transport::Kcp => {
            let kcp = server.kcp.as_ref();
            let mut transport = json!({
                "type": "kcp",
                "header_type": kcp.map(|k| k.header_type.as_str()).unwrap_or("none")
            });
            if let Some(seed) = kcp.and_then(|k| k.seed.as_deref()) {
                transport["seed"] = json!(seed);
            }
            out["transport"] = transport;
        }
        Transport::Ws => {
            let ws = server.ws.as_ref();
            let mut transport = json!({
                "type": "ws",
                "path": ws.map(|w| w.path.as_str()).unwrap_or("/")
            });
            if let Some(host) = ws.and_then(|w| w.host.as_deref()) {
                transport["headers"] = json!({"Host": host});
            }
            out["transport"] = transport;
        }
        Transport::Grpc => {
            let grpc = server.grpc.as_ref();
            out["transport"] = json!({
                "type": "grpc",
                "service_name": grpc.map(|g| g.service_name.as_str()).unwrap_or("")
            });
        }
        Transport::Http => {
            out["transport"] = json!({"type": "http"});
        }
        Transport::Quic => {
            let quic = server.quic.as_ref();
            let mut transport = json!({
                "type": "quic",
                "header_type": quic.map(|q| q.header_type.as_str()).unwrap_or("none"),
                "quic_encryption": quic.map(|q| q.quic_security.as_str()).unwrap_or("none"),
                "key": quic.map(|q| q.key.as_str()).unwrap_or("")
            });
            out["transport"] = transport;
        }
        Transport::XHttp => {
            let xhttp = server.xhttp.as_ref();
            let mut transport = json!({
                "type": "xhttp",
                "path": xhttp.map(|x| x.path.as_str()).unwrap_or("/"),
                "mode": xhttp.map(|x| x.mode.as_str()).unwrap_or("auto")
            });
            if let Some(host) = xhttp.and_then(|x| x.host.as_deref()) {
                transport["host"] = json!(host);
            }
            out["transport"] = transport;
        }
        Transport::HttpUpgrade => {
            let httpupgrade = server.httpupgrade.as_ref();
            let mut transport = json!({
                "type": "httpupgrade",
                "path": httpupgrade.map(|h| h.path.as_str()).unwrap_or("/")
            });
            if let Some(host) = httpupgrade.and_then(|h| h.host.as_deref()) {
                transport["host"] = json!(host);
            }
            out["transport"] = transport;
        }
    }

    // TLS
    if server.tls.enabled {
        let mut tls = json!({
            "enabled": true
        });
        if let Some(sni) = &server.tls.server_name {
            tls["server_name"] = json!(sni);
        }
        if server.tls.insecure {
            tls["insecure"] = json!(true);
        }
        if server.tls.disable_sni {
            tls["disable_sni"] = json!(true);
        }
        if !server.tls.alpn.is_empty() {
            tls["alpn"] = json!(server.tls.alpn);
        }
        if let Some(min_ver) = &server.tls.min_version {
            tls["min_version"] = json!(min_ver);
        }
        if let Some(max_ver) = &server.tls.max_version {
            tls["max_version"] = json!(max_ver);
        }
        if !server.tls.cipher_suites.is_empty() {
            tls["cipher_suites"] = json!(server.tls.cipher_suites);
        }
        if !server.tls.curve_preferences.is_empty() {
            tls["curve_preferences"] = json!(server.tls.curve_preferences);
        }
        if let Some(cert) = &server.tls.certificate {
            tls["certificate"] = json!(cert);
        }
        if let Some(cert_path) = &server.tls.certificate_path {
            tls["certificate_path"] = json!(cert_path);
        }
        if !server.tls.certificate_public_key_sha256.is_empty() {
            tls["certificate_public_key_sha256"] = json!(server.tls.certificate_public_key_sha256);
        }
        if let Some(client_cert) = &server.tls.client_certificate {
            tls["client_certificate"] = json!(client_cert);
        }
        if let Some(client_cert_path) = &server.tls.client_certificate_path {
            tls["client_certificate_path"] = json!(client_cert_path);
        }
        if let Some(client_key) = &server.tls.client_key {
            tls["client_key"] = json!(client_key);
        }
        if let Some(client_key_path) = &server.tls.client_key_path {
            tls["client_key_path"] = json!(client_key_path);
        }
        if server.tls.utls_enabled {
            if let Some(fp) = &server.tls.fingerprint {
                tls["utls"] = json!({"enabled": true, "fingerprint": fp});
            }
        } else if let Some(fp) = &server.tls.fingerprint {
            // For backward compatibility, set utls if fingerprint exists but utls_enabled is false
            tls["utls"] = json!({"enabled": true, "fingerprint": fp});
        }
        if let Some(reality) = &server.tls.reality {
            tls["reality"] = json!({
                "enabled": true,
                "public_key": reality.public_key,
                "short_id": reality.short_id
            });
        }
        out["tls"] = tls;
    }

    Ok(out)
}
