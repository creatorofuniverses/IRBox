use anyhow::Result;
use serde_json::{json, Value};

use crate::proxy::models::*;

/// Generate a minimal Xray-core config for a single server
pub fn generate_config(server: &Server, socks_port: u16, http_port: u16, routing_rules: &[RoutingRule], default_route: &str) -> Result<Value> {
    let outbound = build_outbound(server)?;

    let config = json!({
        "log": {
            "loglevel": "warning"
        },
        "dns": {
            "servers": [
                "https+local://1.1.1.1/dns-query",
                "localhost"
            ]
        },
        "stats": {},
        "api": {
            "tag": "api",
            "services": ["StatsService"]
        },
        "policy": {
            "system": {
                "statsInboundUplink": true,
                "statsInboundDownlink": true,
                "statsOutboundUplink": true,
                "statsOutboundDownlink": true
            }
        },
        "inbounds": [
            {
                "tag": "socks-in",
                "port": socks_port,
                "listen": "127.0.0.1",
                "protocol": "socks",
                "settings": {
                    "udp": true
                },
                "sniffing": {
                    "enabled": true,
                    "destOverride": ["http", "tls"]
                }
            },
            {
                "tag": "http-in",
                "port": http_port,
                "listen": "127.0.0.1",
                "protocol": "http",
                "sniffing": {
                    "enabled": true,
                    "destOverride": ["http", "tls"]
                }
            },
            {
                "tag": "api-in",
                "port": 10813,
                "listen": "127.0.0.1",
                "protocol": "dokodemo-door",
                "settings": {
                    "address": "127.0.0.1"
                }
            }
        ],
        "outbounds": [
            outbound,
            {
                "tag": "direct",
                "protocol": "freedom"
            },
            {
                "tag": "block",
                "protocol": "blackhole"
            }
        ],
        "routing": {
            "domainStrategy": "AsIs",
            "rules": build_xray_routing_rules(routing_rules, default_route)
        }
    });

    Ok(config)
}

fn build_outbound(server: &Server) -> Result<Value> {
    let mut out = json!({"tag": "proxy"});

    match server.protocol {
        Protocol::Vless => {
            out["protocol"] = json!("vless");
            let mut user = json!({
                "id": server.uuid.as_deref().unwrap_or(""),
                "encryption": "none"
            });
            if let Some(flow) = &server.flow {
                if !flow.is_empty() {
                    user["flow"] = json!(flow);
                }
            }
            out["settings"] = json!({
                "vnext": [{
                    "address": server.address,
                    "port": server.port,
                    "users": [user]
                }]
            });
        }
        Protocol::Vmess => {
            out["protocol"] = json!("vmess");
            out["settings"] = json!({
                "vnext": [{
                    "address": server.address,
                    "port": server.port,
                    "users": [{
                        "id": server.uuid.as_deref().unwrap_or(""),
                        "alterId": server.alter_id.unwrap_or(0),
                        "security": "auto"
                    }]
                }]
            });
        }
        Protocol::Shadowsocks => {
            out["protocol"] = json!("shadowsocks");
            out["settings"] = json!({
                "servers": [{
                    "address": server.address,
                    "port": server.port,
                    "method": server.method.as_deref().unwrap_or("aes-256-gcm"),
                    "password": server.password.as_deref().unwrap_or("")
                }]
            });
        }
        Protocol::Trojan => {
            out["protocol"] = json!("trojan");
            out["settings"] = json!({
                "servers": [{
                    "address": server.address,
                    "port": server.port,
                    "password": server.password.as_deref().unwrap_or("")
                }]
            });
        }
        Protocol::Hysteria2 => {
            return Err(anyhow::anyhow!(
                "Hysteria2 protocol is not supported by Xray-core, use Sing-box instead",
            ));
        }
        Protocol::Tuic => {
            return Err(anyhow::anyhow!(
                "TUIC protocol is not supported by Xray-core, use Sing-box instead",
            ));
        }
        Protocol::Ssh => {
            return Err(anyhow::anyhow!(
                "SSH protocol is not supported by Xray-core, use Sing-box instead",
            ));
        }
        Protocol::WireGuard => {
            return Err(anyhow::anyhow!(
                "WireGuard protocol is not supported by Xray-core, use Sing-box instead",
            ));
        }
        Protocol::Tun => {
            return Err(anyhow::anyhow!(
                "TUN interface is not supported by Xray-core, use Sing-box instead",
            ));
        }
        _ => {
            return Err(anyhow::anyhow!(
                "Protocol {:?} not supported by Xray-core",
                server.protocol
            ));
        }
    }

    // Stream settings
    let mut stream = json!({});

    // Transport
    match server.transport {
        Transport::Tcp => {
            stream["network"] = json!("tcp");
        }
        Transport::Kcp => {
            stream["network"] = json!("kcp");
            let kcp = server.kcp.as_ref();
            let mut kcp_settings = json!({
                "header": {
                    "type": kcp.map(|k| k.header_type.as_str()).unwrap_or("none")
                }
            });
            if let Some(seed) = kcp.and_then(|k| k.seed.as_deref()) {
                kcp_settings["seed"] = json!(seed);
            }
            stream["kcpSettings"] = kcp_settings;
        }
        Transport::Ws => {
            stream["network"] = json!("ws");
            let ws = server.ws.as_ref();
            let mut ws_settings = json!({
                "path": ws.map(|w| w.path.as_str()).unwrap_or("/")
            });
            if let Some(host) = ws.and_then(|w| w.host.as_deref()) {
                ws_settings["headers"] = json!({"Host": host});
            }
            stream["wsSettings"] = ws_settings;
        }
        Transport::Grpc => {
            stream["network"] = json!("grpc");
            let grpc = server.grpc.as_ref();
            stream["grpcSettings"] = json!({
                "serviceName": grpc.map(|g| g.service_name.as_str()).unwrap_or("")
            });
        }
        Transport::Http => {
            stream["network"] = json!("h2");
        }
        Transport::Quic => {
            stream["network"] = json!("quic");
            let quic = server.quic.as_ref();
            let mut quic_settings = json!({
                "header": {
                    "type": quic.map(|q| q.header_type.as_str()).unwrap_or("none")
                },
                "security": quic.map(|q| q.quic_security.as_str()).unwrap_or("none"),
                "key": quic.map(|q| q.key.as_str()).unwrap_or("")
            });
            stream["quicSettings"] = quic_settings;
        }
        Transport::XHttp => {
            stream["network"] = json!("xhttp");
            let xhttp = server.xhttp.as_ref();
            let mut xhttp_settings = json!({
                "path": xhttp.map(|x| x.path.as_str()).unwrap_or("/"),
                "mode": xhttp.map(|x| x.mode.as_str()).unwrap_or("auto")
            });
            if let Some(host) = xhttp.and_then(|x| x.host.as_deref()) {
                xhttp_settings["host"] = json!(host);
            }
            stream["xHttpSettings"] = xhttp_settings;
        }
        Transport::HttpUpgrade => {
            stream["network"] = json!("httpupgrade");
            let httpupgrade = server.httpupgrade.as_ref();
            let mut httpupgrade_settings = json!({
                "path": httpupgrade.map(|h| h.path.as_str()).unwrap_or("/")
            });
            if let Some(host) = httpupgrade.and_then(|h| h.host.as_deref()) {
                httpupgrade_settings["host"] = json!(host);
            }
            stream["httpUpgradeSettings"] = httpupgrade_settings;
        }
    }

    // TLS
    if server.tls.enabled {
        if server.tls.reality.is_some() {
            stream["security"] = json!("reality");
            let reality = server.tls.reality.as_ref().unwrap();
            let mut rs = json!({
                "publicKey": reality.public_key,
                "shortId": reality.short_id,
                "fingerprint": server.tls.fingerprint.as_deref().unwrap_or("chrome")
            });
            if let Some(sni) = &server.tls.server_name {
                rs["serverName"] = json!(sni);
            }
            // Add additional TLS fields for reality settings
            if server.tls.insecure {
                rs["allowInsecure"] = json!(true);
            }
            if !server.tls.alpn.is_empty() {
                rs["alpn"] = json!(server.tls.alpn);
            }
            stream["realitySettings"] = rs;
        } else {
            stream["security"] = json!("tls");
            let mut tls = json!({});
            if let Some(sni) = &server.tls.server_name {
                tls["serverName"] = json!(sni);
            }
            if server.tls.insecure {
                tls["allowInsecure"] = json!(true);
            }
            if server.tls.disable_sni {
                tls["disableSNI"] = json!(true);  // XRay specific field
            }
            if !server.tls.alpn.is_empty() {
                tls["alpn"] = json!(server.tls.alpn);
            }
            if let Some(fp) = &server.tls.fingerprint {
                tls["fingerprint"] = json!(fp);
            }
            // Add additional TLS fields for XRay
            if let Some(min_ver) = &server.tls.min_version {
                tls["minVersion"] = json!(min_ver);
            }
            if let Some(max_ver) = &server.tls.max_version {
                tls["maxVersion"] = json!(max_ver);
            }
            if !server.tls.cipher_suites.is_empty() {
                tls["cipherSuites"] = json!(server.tls.cipher_suites);
            }
            stream["tlsSettings"] = tls;
        }
    } else {
        stream["security"] = json!("none");
    }

    out["streamSettings"] = stream;

    Ok(out)
}

fn build_xray_routing_rules(routing_rules: &[RoutingRule], default_route: &str) -> Value {
    let mut rules = vec![
        json!({
            "inboundTag": ["api-in"],
            "outboundTag": "api",
            "type": "field"
        }),
    ];

    for rule in routing_rules.iter().filter(|r| r.enabled) {
        let tag = match rule.action {
            RuleAction::Direct => "direct",
            RuleAction::Block  => "block",
            RuleAction::Proxy  => "proxy",
            // Bridge (external-interface routing) is sing-box only; degrade to proxy.
            RuleAction::Bridge => "proxy",
        };
        rules.push(json!({
            "type": "field",
            "domain": [format!("domain:{}", rule.domain)],
            "outboundTag": tag
        }));
    }

    // If default is "direct", add a catch-all direct rule (xray uses first outbound as default)
    if default_route == "direct" {
        rules.push(json!({
            "type": "field",
            "network": "tcp,udp",
            "outboundTag": "direct"
        }));
    }

    json!(rules)
}
