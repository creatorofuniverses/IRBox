# External-Interface ("bridge") Routing Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let IRBox split-route selected domain rules into an externally-managed network interface (e.g. an AmneziaWG/WireGuard tunnel with `table = off`) via a sing-box `direct` outbound bound with `bind_interface`.

**Architecture:** Pure config feature. A new global `BridgeConfig { interface, routing_mark, endpoints }` is threaded from `AppState` through `CoreManager::start` into `singbox::generate_config`, which emits a `bridge` `direct` outbound (only when an interface is set) and an anti-loop `direct` rule for the tunnel's own endpoints. A new `RuleAction::Bridge` routes a domain to that outbound, falling back to `proxy` when no interface is configured. xray degrades `Bridge` to `proxy`. IRBox never creates or tears down the interface.

**Tech Stack:** Rust (Tauri backend, `serde_json`), React + TypeScript (Vite), no frontend test runner.

**Spec:** `docs/superpowers/specs/2026-06-23-bridge-interface-routing-design.md`

## Global Constraints

- Internal code name is `bridge` / `Bridge`; the sing-box outbound **tag is `"bridge"`**; the serde/TS action value is **`"bridge"`**.
- User-facing label is **"Interface"** (action) / **"Custom interface routing"** (settings block) — never "bridge" in UI copy.
- `endpoints` is a **list** (`Vec<String>` / `string[]`), `#[serde(default)]`, empty = none.
- The anti-loop `direct` rule MUST be pushed **before** the user-defined routing-rules loop (sing-box is first-match-wins).
- The `bridge` outbound is emitted **only** when `interface` is `Some`.
- `RuleAction::Bridge` falls back to `"proxy"` when no interface is configured (sing-box) and **always** degrades to `"proxy"` in xray.
- Each task must leave the project compiling (`cargo build`) / typechecking (`npm run build`).
- i18n is English-only currently; add keys under the existing flat `routing.*` namespace.

---

## File Structure

| File | Responsibility | Tasks |
|------|----------------|-------|
| `src-tauri/src/proxy/models.rs` | `BridgeConfig` struct, `AppState.bridge`, `RuleAction::Bridge` | 1, 3 |
| `src-tauri/src/core/singbox.rs` | bridge outbound + anti-loop + Bridge arm + tests | 2, 3 |
| `src-tauri/src/core/manager.rs` | `start` signature + call site | 2 |
| `src-tauri/src/core/xray.rs` | exhaustive match (`Bridge => "proxy"`) | 3 |
| `src-tauri/src/commands.rs` | thread bridge into 4 `start` sites; get/save plumbing | 2, 4 |
| `src/i18n/translations.ts` | English UI strings | 5 |
| `src/api/tauri.ts` | TS types + invoke wrapper | 6 |
| `src/context/AppContext.tsx` | `bridge` in state/action/reducer/load | 6 |
| `src/components/routing/RoutingPage.tsx` | UI: option, color, settings block, parseEndpoints | 6 |

---

## Task 1: Data model — `BridgeConfig` + `AppState.bridge`

**Files:**
- Modify: `src-tauri/src/proxy/models.rs` (add struct before `AppState` ~line 332; add field in `AppState` ~lines 345-348)
- Test: inline `#[cfg(test)]` in `src-tauri/src/proxy/models.rs`

**Interfaces:**
- Produces: `pub struct BridgeConfig { pub interface: Option<String>, pub routing_mark: Option<u32>, pub endpoints: Vec<String> }` (derives `Debug, Clone, Serialize, Deserialize, Default`); `AppState.bridge: BridgeConfig`.

> Note: do **not** add the `RuleAction::Bridge` variant in this task — that would break the exhaustive matches in `singbox.rs`/`xray.rs` and stop the crate compiling. It lands in Task 3.

- [ ] **Step 1: Write the failing test**

Add at the bottom of `src-tauri/src/proxy/models.rs`:

```rust
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
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd src-tauri && cargo test --lib bridge_config`
Expected: FAIL — `cannot find type BridgeConfig` / `no field bridge on AppState`.

- [ ] **Step 3: Write minimal implementation**

Add this struct immediately before `pub struct AppState` (~line 332):

```rust
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
```

In `AppState`, add the field next to `default_route` (after line 346):

```rust
    #[serde(default = "default_route")]
    pub default_route: String,
    #[serde(default)]
    pub bridge: BridgeConfig,
    #[serde(default)]
    pub onboarding_completed: bool,
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd src-tauri && cargo test --lib bridge_config && cargo test --lib appstate_default_has_empty_bridge`
Expected: PASS (3 tests).

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/proxy/models.rs
git commit -m "feat(bridge): add BridgeConfig model and AppState.bridge field"
```

---

## Task 2: Thread bridge through sing-box generation

**Files:**
- Modify: `src-tauri/src/core/singbox.rs:7` (signature), outbounds literal (~lines 212-218), and add anti-loop guard before the user-rules loop (~line 187)
- Modify: `src-tauri/src/core/manager.rs:116` (`start` signature) and call site (~line 339)
- Modify: `src-tauri/src/commands.rs` — the four `ctx.core.start(...)` sites (~158, ~292, ~547, ~697)
- Test: inline `#[cfg(test)]` in `src-tauri/src/core/singbox.rs`

**Interfaces:**
- Consumes: `BridgeConfig` (Task 1).
- Produces: `generate_config(server, socks_port, http_port, tun_mode, routing_rules, default_route, bridge: &BridgeConfig) -> Result<Value>`; `CoreManager::start(&self, server, tun_mode, routing_rules, default_route, bridge: &BridgeConfig)`. When `bridge.interface` is `Some`, `outbounds` contains an object `{ "type":"direct", "tag":"bridge", "bind_interface":<iface> [, "routing_mark":<mark>] }`. When `bridge.endpoints` is non-empty and an interface is set, `route.rules` contains `{ "ip_cidr": <endpoints>, "outbound":"direct" }` positioned before any user rule.

> This task does NOT add `RuleAction::Bridge`. It only threads the config and emits the outbound + anti-loop rule. The Bridge match arm lands in Task 3.

- [ ] **Step 1: Write the failing test**

Add at the bottom of `src-tauri/src/core/singbox.rs`:

```rust
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
        assert!(route_rules.iter().all(|r| r["ip_cidr"].is_null() || r["outbound"] != "direct"
            || r["ip_cidr"][0] == "192.0.2.1/32" == false));
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd src-tauri && cargo test --lib singbox::tests 2>&1 | head -30`
Expected: FAIL to compile — `generate_config` takes 6 args, not 7.

- [ ] **Step 3: Update `generate_config` signature**

`src-tauri/src/core/singbox.rs:7`:

```rust
pub fn generate_config(server: &Server, socks_port: u16, http_port: u16, tun_mode: bool, routing_rules: &[RoutingRule], default_route: &str, bridge: &BridgeConfig) -> Result<Value> {
```

- [ ] **Step 4: Add the anti-loop guard before the user-rules loop**

Immediately before the `// User-defined routing rules` loop (~line 187), insert:

```rust
    // Anti-loop guard: keep the external tunnel's own endpoint traffic on
    // `direct` so its handshake/data is not captured back into sing-box (TUN).
    // Must precede user rules (first match wins).
    if bridge.interface.is_some() && !bridge.endpoints.is_empty() {
        route_rules.push(json!({ "ip_cidr": bridge.endpoints, "outbound": "direct" }));
    }
```

- [ ] **Step 5: Build the outbounds vec with the optional bridge outbound**

Replace the inline `"outbounds": [ outbound, { "type":"direct", "tag":"direct" } ]` (~lines 212-218). First, before the `let mut config = json!({...})` block, build the vec:

```rust
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
```

Then in the `json!({...})` config literal, replace the `"outbounds": [ ... ]` line with:

```rust
        "outbounds": outbounds,
```

- [ ] **Step 6: Thread `bridge` through `CoreManager::start`**

`src-tauri/src/core/manager.rs:116`:

```rust
    pub async fn start(&self, server: &Server, tun_mode: bool, routing_rules: &[RoutingRule], default_route: &str, bridge: &BridgeConfig) -> Result<()> {
```

Update the sing-box call site (~line 339); leave the xray site unchanged:

```rust
        CoreType::SingBox => singbox::generate_config(server, socks_port, http_port, tun_mode, routing_rules, default_route, bridge)?,
        CoreType::Xray => xray::generate_config(server, socks_port, http_port, routing_rules, default_route)?,
```

(`BridgeConfig` is in scope via the existing `use crate::proxy::models::*;`.)

- [ ] **Step 7: Pass bridge at all four `commands.rs` start sites**

At each of the four sites (~158, ~292, ~547, ~697), next to where `default_route` is obtained from `state`, add a clone and pass it as the new last arg. Pattern at each site:

```rust
    let bridge = state.bridge.clone();
    // ...existing start call, with &bridge appended:
    ctx.core.start(&server, tun_mode, &routing_rules, &default_route, &bridge).await
```

(`BridgeConfig` is in scope via the existing `use crate::proxy::{... models::* ...};`. If a site holds the state lock only briefly, clone `bridge` in the same scope as `default_route`.)

- [ ] **Step 8: Build and run tests**

Run: `cd src-tauri && cargo build && cargo test --lib singbox::tests`
Expected: builds clean; 5 sing-box tests PASS.

- [ ] **Step 9: Commit**

```bash
git add src-tauri/src/core/singbox.rs src-tauri/src/core/manager.rs src-tauri/src/commands.rs
git commit -m "feat(bridge): emit bridge outbound + anti-loop rule, thread config to start"
```

---

## Task 3: `RuleAction::Bridge` variant + match arms

**Files:**
- Modify: `src-tauri/src/proxy/models.rs:301-306` (add variant)
- Modify: `src-tauri/src/core/singbox.rs` rule-match loop (~lines 190-200, add Bridge arm)
- Modify: `src-tauri/src/core/xray.rs:327-331` (add Bridge arm)
- Test: inline `#[cfg(test)]` in `src-tauri/src/core/singbox.rs` (extend existing mod)

**Interfaces:**
- Consumes: bridge outbound emission (Task 2).
- Produces: `RuleAction::Bridge` (serde `"bridge"`). A `Bridge` rule routes to outbound `"bridge"` when `bridge.interface` is set, else `"proxy"` (sing-box). xray maps `Bridge => "proxy"`.

- [ ] **Step 1: Write the failing test**

Add to the `mod tests` in `src-tauri/src/core/singbox.rs`:

```rust
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
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd src-tauri && cargo test --lib singbox::tests 2>&1 | head -20`
Expected: FAIL to compile — `no variant named Bridge`.

- [ ] **Step 3: Add the variant**

`src-tauri/src/proxy/models.rs` `RuleAction` (~line 305):

```rust
pub enum RuleAction {
    #[default]
    Proxy,
    Direct,
    Block,
    /// Route matching traffic into an externally-managed interface (e.g. an
    /// AmneziaWG tunnel brought up with `table = off`). sing-box only.
    Bridge,
}
```

- [ ] **Step 4: Add the sing-box Bridge arm**

In the rule-match loop in `src-tauri/src/core/singbox.rs` (after the `RuleAction::Proxy` arm, ~line 199):

```rust
            RuleAction::Bridge => {
                // Falls back to `proxy` if no bridge interface is configured.
                let outbound = if bridge.interface.is_some() { "bridge" } else { "proxy" };
                route_rules.push(json!({ "domain_suffix": [domain], "outbound": outbound }));
            }
```

- [ ] **Step 5: Add the xray Bridge arm (keep match exhaustive)**

`src-tauri/src/core/xray.rs:327-331`:

```rust
        let tag = match rule.action {
            RuleAction::Direct => "direct",
            RuleAction::Block  => "block",
            RuleAction::Proxy  => "proxy",
            // Bridge (external-interface routing) is sing-box only; degrade to proxy.
            RuleAction::Bridge => "proxy",
        };
```

- [ ] **Step 6: Build and run all backend tests**

Run: `cd src-tauri && cargo build && cargo test --lib`
Expected: builds clean; all tests PASS (including the 2 new ones).

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/proxy/models.rs src-tauri/src/core/singbox.rs src-tauri/src/core/xray.rs
git commit -m "feat(bridge): add RuleAction::Bridge and route it to the bridge outbound"
```

---

## Task 4: Persistence API — get/save `bridge`

**Files:**
- Modify: `src-tauri/src/commands.rs` — `RoutingRulesResponse` (~lines 662-666), `get_routing_rules` (~669-675), `save_routing_rules` (~677-705)

**Interfaces:**
- Consumes: `AppState.bridge` (Task 1).
- Produces: `RoutingRulesResponse.bridge: BridgeConfig`; `save_routing_rules(rules, default_route, bridge: BridgeConfig, ctx)` persists `state.bridge = bridge`. This is the JSON contract the frontend (Task 6) binds to: `save_routing_rules` is invoked with `{ rules, defaultRoute, bridge }`.

- [ ] **Step 1: Add `bridge` to the response struct**

`src-tauri/src/commands.rs` `RoutingRulesResponse` (~line 662):

```rust
#[derive(Serialize)]
pub struct RoutingRulesResponse {
    pub rules: Vec<RoutingRule>,
    pub default_route: String,
    pub bridge: BridgeConfig,
}
```

- [ ] **Step 2: Populate it in `get_routing_rules`**

In the body of `get_routing_rules` where the response is constructed, add the field:

```rust
    Ok(RoutingRulesResponse {
        rules: state.routing_rules.clone(),
        default_route: state.default_route.clone(),
        bridge: state.bridge.clone(),
    })
```

- [ ] **Step 3: Accept and persist `bridge` in `save_routing_rules`**

`src-tauri/src/commands.rs` `save_routing_rules` signature + body:

```rust
pub async fn save_routing_rules(
    rules: Vec<RoutingRule>,
    default_route: String,
    bridge: BridgeConfig,
    ctx: State<'_, AppContext>,
) -> Result<(), String> {
    // ...existing locking of state...
    state.routing_rules = rules;
    state.default_route = default_route;
    state.bridge = bridge;
    // ...existing persist + reconnect (the ~697 start site already passes &bridge from Task 2)...
```

(`BridgeConfig` is in scope via the existing `use crate::proxy::{... models::* ...};`.)

- [ ] **Step 4: Build**

Run: `cd src-tauri && cargo build`
Expected: builds clean.

- [ ] **Step 5: Verify the command signature is registered**

Run: `cd src-tauri && cargo build 2>&1 | grep -i "save_routing_rules" || echo "no errors mentioning save_routing_rules"`
Expected: no errors (Tauri's `#[tauri::command]` arg deserialization picks up the new `bridge` param automatically).

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/commands.rs
git commit -m "feat(bridge): persist and return bridge config via routing commands"
```

---

## Task 5: i18n strings

**Files:**
- Modify: `src/i18n/translations.ts` — add keys under the `routing.*` namespace (flat dot-notation, next to existing `routing.proxy` ~lines 119-136)

**Interfaces:**
- Produces: keys `routing.bridge`, `routing.bridgeSettings`, `routing.bridgeInterface`, `routing.bridgeInterfacePlaceholder`, `routing.bridgeEndpoints`, `routing.bridgeEndpointsPlaceholder`, `routing.bridgeMark`, `routing.bridgeMarkPlaceholder`, `routing.bridgeHelp` (consumed by Task 6).

- [ ] **Step 1: Add the keys**

Add to the `en` object alongside the existing `routing.*` keys:

```ts
  "routing.bridge": "Interface",
  "routing.bridgeSettings": "Custom interface routing",
  "routing.bridgeInterface": "Interface name",
  "routing.bridgeInterfacePlaceholder": "e.g. awg0",
  "routing.bridgeEndpoints": "Endpoint IPs to exclude",
  "routing.bridgeEndpointsPlaceholder": "comma-separated, e.g. 192.0.2.1, 198.51.100.7",
  "routing.bridgeMark": "Firewall mark (fwmark)",
  "routing.bridgeMarkPlaceholder": "optional, e.g. 51820",
  "routing.bridgeHelp": "Route the \"Interface\" action into an externally-managed network interface (e.g. a WireGuard/AmneziaWG tunnel). IRBox does not create the interface.",
```

- [ ] **Step 2: Typecheck**

Run: `npm run build`
Expected: tsc + vite build succeed.

- [ ] **Step 3: Commit**

```bash
git add src/i18n/translations.ts
git commit -m "feat(bridge): add i18n strings for interface routing"
```

---

## Task 6: Frontend types, context, and UI

**Files:**
- Modify: `src/api/tauri.ts` (RuleAction ~line 69, BridgeConfig + RoutingRulesResponse ~78-81, saveRoutingRules ~151-152)
- Modify: `src/context/AppContext.tsx` (AppState ~16-34, initial ~55-73, SET_ROUTING_RULES action ~91 + reducer ~166-167, initial-load dispatch)
- Modify: `src/components/routing/RoutingPage.tsx` (save ~65-79 + callers, actionColor ~135-144, action select ~206-208; add setBridge + parseEndpoints + settings block)

**Interfaces:**
- Consumes: `save_routing_rules` JSON contract (Task 4); i18n keys (Task 5).
- Produces: typed `BridgeConfig` round-tripped through context and persisted on every save.

> This is one task because the new required `bridge` parameter on `saveRoutingRules` ripples through every consumer; splitting it would leave `tsc` red between steps.

- [ ] **Step 1: Update `src/api/tauri.ts`**

```ts
export type RuleAction = "proxy" | "direct" | "block" | "bridge";

export interface BridgeConfig {
  interface: string | null;
  routing_mark: number | null;
  endpoints: string[];
}

export interface RoutingRulesResponse {
  rules: RoutingRule[];
  default_route: string;
  bridge: BridgeConfig;
}
```

And the invoke wrapper (~line 151):

```ts
  saveRoutingRules: (rules: RoutingRule[], defaultRoute: string, bridge: BridgeConfig) =>
    invoke<void>("save_routing_rules", { rules, defaultRoute, bridge }),
```

- [ ] **Step 2: Update `src/context/AppContext.tsx`**

Import and state:

```ts
import { /* existing */ BridgeConfig } from "../api/tauri";
```

Add to `AppState` (~line 29, after `defaultRoute`):

```ts
  bridge: BridgeConfig;
```

Initial state (~line 56, after `defaultRoute: "proxy"`):

```ts
  bridge: { interface: null, routing_mark: null, endpoints: [] },
```

Action type (~line 91):

```ts
  | { type: "SET_ROUTING_RULES"; rules: RoutingRule[]; defaultRoute: string; bridge: BridgeConfig }
```

Reducer case (~line 166):

```ts
    case "SET_ROUTING_RULES":
      return { ...state, routingRules: action.rules, defaultRoute: action.defaultRoute, bridge: action.bridge };
```

Initial load (wherever `getRoutingRules` result is dispatched): add `bridge: routing.bridge` to the dispatched `SET_ROUTING_RULES` payload.

- [ ] **Step 3: Update `src/components/routing/RoutingPage.tsx` — save + callers**

Import `BridgeConfig` from `../../api/tauri`. Change the `save` callback (~line 65) to take a third arg and pass it through:

```ts
  const save = useCallback(
    (rules: RoutingRule[], defaultRoute: string, bridge: BridgeConfig) => {
      dispatch({ type: "SET_ROUTING_RULES", rules, defaultRoute, bridge });
      clearTimeout(saveTimer.current);
      saveTimer.current = setTimeout(async () => {
        try {
          await api.saveRoutingRules(rules, defaultRoute, bridge);
          toast(t("routing.saved"), "success");
        } catch (e) {
          toast(`${e}`, "error");
        }
      }, 800);
    },
    [dispatch, toast]
  );
```

Update every existing `save(...)` caller to pass `state.bridge` as the third arg:
- `setDefaultRoute`: `save(state.routingRules, route, state.bridge)`
- `addRule`: `save([...state.routingRules, rule], state.defaultRoute, state.bridge)`
- `removeRule`: `save(filtered, state.defaultRoute, state.bridge)`
- `toggleRule`: `save(mapped, state.defaultRoute, state.bridge)`
- `addPreset`: `save([...state.routingRules, ...newRules], state.defaultRoute, state.bridge)`

- [ ] **Step 4: Add `parseEndpoints` + `setBridge` helpers**

Near the top of the component body:

```ts
  const parseEndpoints = (raw: string): string[] =>
    raw.split(/[\s,]+/).map((s) => s.trim()).filter((s) => s.length > 0);

  const setBridge = (patch: Partial<BridgeConfig>) =>
    save(state.routingRules, state.defaultRoute, { ...state.bridge, ...patch });
```

- [ ] **Step 5: Add the select option + color**

Action `<select>` (~line 206), add:

```tsx
            <option value="bridge">{t("routing.bridge")}</option>
```

`actionColor` (~line 135), add a case (use the warning/secondary token; `var(--warning)` if it exists, else `var(--accent)`):

```ts
    case "bridge":
      return "var(--warning)";
```

- [ ] **Step 6: Add the "Custom interface routing" settings block**

Place it near the default-route controls (above or below the rules list):

```tsx
      <section className="bridge-settings">
        <h3>{t("routing.bridgeSettings")}</h3>
        <p className="hint">{t("routing.bridgeHelp")}</p>
        <label>
          {t("routing.bridgeInterface")}
          <input
            type="text"
            placeholder={t("routing.bridgeInterfacePlaceholder")}
            value={state.bridge.interface ?? ""}
            onChange={(e) => setBridge({ interface: e.target.value.trim() || null })}
          />
        </label>
        <label>
          {t("routing.bridgeEndpoints")}
          <input
            type="text"
            placeholder={t("routing.bridgeEndpointsPlaceholder")}
            value={state.bridge.endpoints.join(", ")}
            onChange={(e) => setBridge({ endpoints: parseEndpoints(e.target.value) })}
          />
        </label>
        <label>
          {t("routing.bridgeMark")}
          <input
            type="number"
            placeholder={t("routing.bridgeMarkPlaceholder")}
            value={state.bridge.routing_mark ?? ""}
            onChange={(e) =>
              setBridge({ routing_mark: e.target.value === "" ? null : Number(e.target.value) })
            }
          />
        </label>
      </section>
```

- [ ] **Step 7: Typecheck / build**

Run: `npm run build`
Expected: tsc + vite build succeed with no type errors.

- [ ] **Step 8: Manual verification**

Run: `npm run tauri dev` (or the project's dev command). Confirm:
- The Routing page shows the "Custom interface routing" block with three inputs.
- Selecting "Interface" as a rule action is possible and shows the bridge color.
- Setting interface = `awg0`, endpoints = `192.0.2.1, 198.51.100.7`, and reloading the app preserves the values (round-trips through get/save).

- [ ] **Step 9: Commit**

```bash
git add src/api/tauri.ts src/context/AppContext.tsx src/components/routing/RoutingPage.tsx
git commit -m "feat(bridge): UI + state for custom interface routing"
```

---

## Final verification

- [ ] `cd src-tauri && cargo test --lib` — all backend tests pass.
- [ ] `cd src-tauri && cargo build` — clean.
- [ ] `npm run build` — clean.
- [ ] Manual: with an external `awg0` (`table = off`) up, set interface/endpoints, add an "Interface" rule for a test domain in TUN mode, and confirm the domain egresses via `awg0` while the tunnel handshake still reaches its endpoint (no loop).

---

## Self-review notes

- **Spec coverage:** §2 model → Task 1/3; §3 sing-box (anti-loop, Bridge arm, outbounds vec) → Task 2/3; §4 Rust wiring → Task 2; §5 frontend → Task 6; commands get/save → Task 4; §6 anti-loop → Task 2 (config side) + manual final verification (OS side); §7 tests → Tasks 1-3 (Rust) + Task 6 manual (no FE runner); i18n → Task 5. No gaps.
- **No placeholders:** every code step shows real code; commands have expected output.
- **Type consistency:** `BridgeConfig`/`endpoints`/`bridge` names and the `"bridge"` tag/action value are identical across Rust, the TS types, the invoke payload (`{ rules, defaultRoute, bridge }`), and the UI.
