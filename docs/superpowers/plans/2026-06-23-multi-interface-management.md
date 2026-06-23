# Multi-Interface Management Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the single global "bridge" interface with a list of named custom interfaces and one **active** selection, managed on a dedicated **Interfaces** page, while the on-wire sing-box config stays unchanged (still at most one `bridge` outbound, now sourced from the active entry).

**Architecture:** Backend keeps a `Vec<InterfaceConfig>` + `active_interface_id` on `AppState` (the legacy `bridge` field stays as a dual-write downgrade artifact). `generate_config`/`CoreManager::start` take the resolved active interface (`Option<&InterfaceConfig>`). New Tauri CRUD commands persist and reconnect only when the change touches the active interface (M3). Frontend gains an Interfaces page (list + modal, modeled on Subscriptions) and an `interfaces` sidebar entry; the Routing page loses its bridge fields.

**Tech Stack:** Rust + Tauri 2 (`src-tauri/`, crate `irbox`), `uuid` v4 (already a dependency), `serde`/`serde_json`. React 18 + TypeScript (Vite), `useReducer` context, flat i18n key map.

## Global Constraints

- **On-wire sing-box config is unchanged** — at most one `bridge` `direct` outbound (tag `"bridge"`, `bind_interface`, optional `routing_mark`), emitted only when an active interface exists; one anti-loop `{ ip_cidr: <active.endpoints>, outbound: "direct" }` rule before user rules when the active interface has endpoints; `RuleAction::Bridge` → `"bridge"` if active else `"proxy"`. xray and the Custom-protocol `manager.rs` path keep degrading `Bridge → "proxy"` (unchanged).
- **Keep internal `bridge`/`Bridge` naming** — the sing-box outbound tag `"bridge"`, `RuleAction::Bridge`, and the legacy `bridge` config field stay. Do not rename (minimal upstream-clean diff).
- **`id` ownership = backend.** `save_interface` mints a uuid when `config.id` is empty (add); a non-empty id is an update. The frontend submits `id: ""` for adds. Migration mints backend-side.
- **`label` defaults to `interface`** — resolved backend-side in the save path, so every consumer sees a filled label.
- **Dual-write downgrade safety** — `AppState` has no version field; keep the legacy `bridge` field populated from the active interface on every persist (`sync_legacy_bridge`) so a rollback to v1.1.0 still finds a usable config.
- **Reconnect only on active change (M3)** — a running core restarts only when the mutation touches the resolved active interface (`set_active_interface` always; `save_interface`/`delete_interface` only when the affected id is the active one). Non-active mutations persist without reconnecting.
- **No JS test runner** — frontend gate is `npm run build` (`tsc && vite build`); behavior verified manually. Do NOT add a JS test framework. Backend gate is `cd src-tauri && cargo test` (pure-function unit tests; no sidecar binaries needed).
- **New UI i18n keys are English-only** — `src/i18n/translations.ts` `Lang` type is `"en"` only; there are no Persian UI keys (Persian lives in README_FA docs, not the UI).
- **Out of scope (later plans):** interface-only mode / "connect with no server" (item B, M2, M4 stop-core), liveness/status indicator (item C). This plan keeps the existing behavior that the core only runs with a proxy server.

### Transition window (read before sequencing)

Backend Tasks 2–3 change the Tauri contract (routing commands drop `bridge`; new interface commands appear). The frontend is not cut over until Tasks 4–7. **Do not launch the full app between Task 2 and Task 7** — it will be mid-migration. Each task still passes its *own* gate (`cargo test` for backend, `npm run build` for frontend) independently. Full manual verification happens after Task 7; docs in Task 8.

---

## File Structure

**Backend (`src-tauri/src/`):**
- `proxy/models.rs` — **modify.** Add `InterfaceConfig`; add `interfaces` + `active_interface_id` to `AppState`; add `impl AppState` methods (`active_interface`, `sync_legacy_bridge`, `migrate_bridge_to_interfaces`, `upsert_interface`, `delete_interface`, `set_active`); unit tests.
- `core/singbox.rs` — **modify.** `generate_config` takes `Option<&InterfaceConfig>` instead of `&BridgeConfig`; update internals + the 7 bridge tests.
- `core/manager.rs` — **modify.** `CoreManager::start` takes `Option<&InterfaceConfig>`; thread it into `singbox::generate_config`.
- `commands.rs` — **modify.** Update `connect`, `save_settings` (ports reconnect), and `save_routing_rules` call sites; drop `bridge` from `RoutingRulesResponse`/`get_routing_rules`/`save_routing_rules`; add `reconnect_active` helper; add the four interface commands + `InterfacesResponse`; `load_state` runs migration + sync.
- `lib.rs` — **modify.** Register the four new commands in `generate_handler!`.

**Frontend (`src/`):**
- `api/tauri.ts` — **modify.** Add `InterfaceConfig`/`InterfacesResponse` + four wrappers; (Task 7) drop `bridge` from `RoutingRulesResponse`/`saveRoutingRules`.
- `context/AppContext.tsx` — **modify.** Add `interfaces`/`activeInterfaceId` + `SET_INTERFACES` + bootstrap; extend `Page`; (Task 7) drop `bridge`.
- `components/ui/Icons.tsx` — **modify.** Add `NetworkIcon`.
- `components/layout/Sidebar.tsx` — **modify.** Add the `interfaces` nav item.
- `App.tsx` — **modify.** Route `case "interfaces"`.
- `components/interfaces/InterfacesPage.tsx` — **create.** List of interface cards + Add button.
- `components/interfaces/InterfaceModal.tsx` — **create.** Add/edit modal with validation.
- `components/routing/RoutingPage.tsx` — **modify (Task 7).** Remove the bridge block + `setBridge` + `parseEndpoints`; simplify `save()`; add the required "no active interface" hint.
- `i18n/translations.ts` — **modify.** Add `nav.interfaces` + `interfaces.*` keys.

**Docs:**
- `README.md`, `docs/README.md` — **modify (Task 8).** Multi-interface how-to (English). `README_FA.md` is maintained by the user — leave it.

---

## Task 1: Backend — InterfaceConfig model, AppState fields, migration

**Files:**
- Modify: `src-tauri/src/proxy/models.rs` (add struct after `BridgeConfig` ~line 349; add fields to `AppState` ~line 367; add `impl AppState`; add tests in `mod tests`)
- Test: same file's `#[cfg(test)] mod tests`

**Interfaces:**
- Consumes: nothing (additive). `BridgeConfig` (models.rs:337), `RuleAction` (models.rs:299), `uuid` crate.
- Produces: `pub struct InterfaceConfig { id, label, interface, routing_mark: Option<u32>, endpoints: Vec<String> }`; `AppState.interfaces: Vec<InterfaceConfig>`, `AppState.active_interface_id: Option<String>`; `AppState::active_interface(&self) -> Option<&InterfaceConfig>`, `AppState::sync_legacy_bridge(&mut self)`, `AppState::migrate_bridge_to_interfaces(&mut self)`. Consumed by Tasks 2 and 3.

- [ ] **Step 1: Write the failing tests**

In `src-tauri/src/proxy/models.rs`, inside the existing `mod tests { use super::*; ... }`, add these tests (after the existing bridge tests):

```rust
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
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cd src-tauri && cargo test proxy::models`
Expected: FAIL to compile — `InterfaceConfig` not found, `AppState` has no `interfaces`/`active_interface_id` fields, methods missing.

- [ ] **Step 3: Add the `InterfaceConfig` struct**

In `src-tauri/src/proxy/models.rs`, immediately after the `BridgeConfig` struct (ends ~line 349), add:

```rust
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
```

- [ ] **Step 4: Add the `AppState` fields**

In the `AppState` struct, add two fields immediately after `pub bridge: BridgeConfig,` (line 367) and before `pub onboarding_completed: bool,`:

```rust
    #[serde(default)]
    pub bridge: BridgeConfig,
    #[serde(default)]
    pub interfaces: Vec<InterfaceConfig>,
    #[serde(default)]
    pub active_interface_id: Option<String>,
    #[serde(default)]
    pub onboarding_completed: bool,
```

(Only the two middle lines are new; the surrounding lines show placement.)

- [ ] **Step 5: Add the `impl AppState` methods**

Add a new `impl AppState { ... }` block after the `AppState` struct definition (before `mod tests`):

```rust
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
```

- [ ] **Step 6: Run the tests to verify they pass**

Run: `cd src-tauri && cargo test proxy::models`
Expected: PASS — all new tests plus the existing `bridge_config_*` / `appstate_default_has_empty_bridge` tests pass.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/proxy/models.rs
git commit -m "feat(interfaces): add InterfaceConfig model + AppState migration

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_011btTtMPPUr9HEUYTvV6cUZ"
```

---

## Task 2: Backend — thread active interface through core-start; drop bridge from routing

**Files:**
- Modify: `src-tauri/src/core/singbox.rs` (signature line 7; internals lines 190-192, 207-211, 221-231; tests lines ~264-368)
- Modify: `src-tauri/src/core/manager.rs` (signature line 116; call at lines 342-345)
- Modify: `src-tauri/src/commands.rs` (`connect` 153-161; `save_settings` ports reconnect 540-552; `RoutingRulesResponse` 665-680; `get_routing_rules`; `save_routing_rules`; `load_state` ~822)

**Interfaces:**
- Consumes (from Task 1): `InterfaceConfig`, `AppState::active_interface`, `AppState::migrate_bridge_to_interfaces`, `AppState::sync_legacy_bridge`.
- Produces: `singbox::generate_config(server, socks_port, http_port, tun_mode, routing_rules, default_route, active_interface: Option<&InterfaceConfig>)`; `CoreManager::start(server, tun_mode, routing_rules, default_route, active_interface: Option<&InterfaceConfig>)`; `RoutingRulesResponse { rules, default_route }` (no `bridge`); `save_routing_rules(rules, default_route, ctx)` (no `bridge`). Consumed by Task 3 (reconnect path) and Tasks 4/7 (frontend contract).

- [ ] **Step 1: Update the sing-box generator tests (write the new expected shape first)**

In `src-tauri/src/core/singbox.rs`, replace the entire `#[cfg(test)] mod tests { ... }` block with the version below. It keeps `test_server()` and `outbound_tags()`, adds an `iface()` helper, and converts each test from `&BridgeConfig` to `Option<&InterfaceConfig>`:

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

    fn iface(name: &str, mark: Option<u32>, endpoints: Vec<String>) -> InterfaceConfig {
        InterfaceConfig {
            id: "i".into(), label: name.into(), interface: name.into(),
            routing_mark: mark, endpoints,
        }
    }

    #[test]
    fn no_bridge_outbound_when_no_active_interface() {
        let cfg = generate_config(&test_server(), 1080, 1081, false, &[], "proxy", None).unwrap();
        assert!(!outbound_tags(&cfg).contains(&"bridge".to_string()));
    }

    #[test]
    fn bridge_outbound_emitted_with_bind_interface_and_mark() {
        let i = iface("awg0", Some(51820), vec![]);
        let cfg = generate_config(&test_server(), 1080, 1081, false, &[], "proxy", Some(&i)).unwrap();
        let out = cfg["outbounds"].as_array().unwrap().iter()
            .find(|o| o["tag"] == "bridge").expect("bridge outbound present");
        assert_eq!(out["type"], "direct");
        assert_eq!(out["bind_interface"], "awg0");
        assert_eq!(out["routing_mark"], 51820);
    }

    #[test]
    fn bridge_outbound_omits_mark_when_unset() {
        let i = iface("awg0", None, vec![]);
        let cfg = generate_config(&test_server(), 1080, 1081, false, &[], "proxy", Some(&i)).unwrap();
        let out = cfg["outbounds"].as_array().unwrap().iter()
            .find(|o| o["tag"] == "bridge").expect("bridge outbound present");
        assert!(out.get("routing_mark").is_none());
    }

    #[test]
    fn antiloop_rule_precedes_user_rules() {
        let i = iface("awg0", None, vec!["192.0.2.1/32".into(), "198.51.100.7".into()]);
        let rules = vec![RoutingRule {
            id: "r".into(), domain: "example.com".into(), action: RuleAction::Bridge, enabled: true,
        }];
        let cfg = generate_config(&test_server(), 1080, 1081, false, &rules, "proxy", Some(&i)).unwrap();
        let route_rules = cfg["route"]["rules"].as_array().unwrap();
        let antiloop_idx = route_rules.iter().position(|r| r.get("ip_cidr").is_some()).unwrap();
        let user_idx = route_rules.iter().position(|r| r.get("domain_suffix").is_some()).unwrap();
        assert!(antiloop_idx < user_idx);
    }

    #[test]
    fn no_antiloop_rule_when_endpoints_empty() {
        let i = iface("awg0", None, vec![]);
        let cfg = generate_config(&test_server(), 1080, 1081, false, &[], "proxy", Some(&i)).unwrap();
        let route_rules = cfg["route"]["rules"].as_array().unwrap();
        assert!(!route_rules.iter().any(|r| r.get("ip_cidr").is_some() && r["outbound"] == "direct"));
    }

    #[test]
    fn bridge_rule_routes_to_bridge_when_active() {
        let i = iface("awg0", None, vec![]);
        let rules = vec![RoutingRule {
            id: "r".into(), domain: "example.com".into(), action: RuleAction::Bridge, enabled: true,
        }];
        let cfg = generate_config(&test_server(), 1080, 1081, false, &rules, "proxy", Some(&i)).unwrap();
        let rule = cfg["route"]["rules"].as_array().unwrap().iter()
            .find(|r| r.get("domain_suffix").is_some()).unwrap();
        assert_eq!(rule["outbound"], "bridge");
    }

    #[test]
    fn bridge_rule_falls_back_to_proxy_when_no_active() {
        let rules = vec![RoutingRule {
            id: "r".into(), domain: "example.com".into(), action: RuleAction::Bridge, enabled: true,
        }];
        let cfg = generate_config(&test_server(), 1080, 1081, false, &rules, "proxy", None).unwrap();
        let rule = cfg["route"]["rules"].as_array().unwrap().iter()
            .find(|r| r.get("domain_suffix").is_some()).unwrap();
        assert_eq!(rule["outbound"], "proxy");
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cd src-tauri && cargo test core::singbox`
Expected: FAIL to compile — `generate_config` still expects `&BridgeConfig`, so `None` / `Some(&i)` arguments don't type-check.

- [ ] **Step 3: Change the `generate_config` signature and internals**

In `src-tauri/src/core/singbox.rs`, change the signature (line 7):

```rust
pub fn generate_config(server: &Server, socks_port: u16, http_port: u16, tun_mode: bool, routing_rules: &[RoutingRule], default_route: &str, active_interface: Option<&InterfaceConfig>) -> Result<Value> {
```

Replace the anti-loop block (lines 190-192):

```rust
    if let Some(iface) = active_interface {
        if !iface.endpoints.is_empty() {
            route_rules.push(json!({ "ip_cidr": iface.endpoints, "outbound": "direct" }));
        }
    }
```

Replace the `RuleAction::Bridge` mapping (lines 207-211):

```rust
            RuleAction::Bridge => {
                // Routes into the active interface's bridge outbound; falls back
                // to `proxy` when no interface is active.
                let outbound = if active_interface.is_some() { "bridge" } else { "proxy" };
                route_rules.push(json!({ "domain_suffix": [domain], "outbound": outbound }));
            }
```

Replace the bridge-outbound emission block (lines 225-231, the `if let Some(ref iface) = bridge.interface { ... }`):

```rust
    if let Some(iface) = active_interface {
        let mut bridge_out = json!({
            "type": "direct",
            "tag": "bridge",
            "bind_interface": iface.interface,
        });
        if let Some(mark) = iface.routing_mark {
            bridge_out["routing_mark"] = json!(mark);
        }
        outbounds.push(bridge_out);
    }
```

(`InterfaceConfig` is in scope via the existing `use crate::proxy::models::*;` at line 4 — no import change needed.)

- [ ] **Step 4: Change `CoreManager::start` and its sing-box call**

In `src-tauri/src/core/manager.rs`, change the signature (line 116):

```rust
    pub async fn start(&self, server: &Server, tun_mode: bool, routing_rules: &[RoutingRule], default_route: &str, active_interface: Option<&InterfaceConfig>) -> Result<()> {
```

Change the sing-box generate call (line 343) to pass `active_interface`; leave the xray arm unchanged:

```rust
        let config = match core_type {
            CoreType::SingBox => singbox::generate_config(server, socks_port, http_port, tun_mode, routing_rules, default_route, active_interface)?,
            CoreType::Xray => xray::generate_config(server, socks_port, http_port, routing_rules, default_route)?,
        };
```

(The Custom-protocol `RuleAction::Bridge => "proxy"` degrade at line 304 is unchanged.)

- [ ] **Step 5: Update the `connect` and `save_settings` call sites in commands.rs**

In `connect` (commands.rs), replace `let bridge = state.bridge.clone();` (line 156) and the `start` call (lines 158-161):

```rust
    let active_iface = state.active_interface().cloned();

    ctx.core
        .start(&server, tun_mode, &routing_rules, &default_route, active_iface.as_ref())
        .await
        .map_err(|e| format!("Failed to start core: {}", e))?;
```

In `save_settings`, inside the ports-reconnect block, replace `let bridge = state.bridge.clone();` (line 547) and the `start` call (line 550):

```rust
                    let active_iface = state.active_interface().cloned();
                    drop(state);
                    // Reconnect with new ports
                    if let Err(e) = ctx.core.start(&s, tun_mode, &rules, &dr, active_iface.as_ref()).await {
                        log::error!("Failed to reconnect with new ports: {}", e);
                    }
```

- [ ] **Step 6: Drop `bridge` from the routing response and commands**

In `commands.rs`, change `RoutingRulesResponse` (remove the `bridge` field):

```rust
#[derive(Serialize)]
pub struct RoutingRulesResponse {
    pub rules: Vec<RoutingRule>,
    pub default_route: String,
}
```

Change `get_routing_rules` (drop the `bridge` line):

```rust
#[tauri::command]
pub async fn get_routing_rules(ctx: State<'_, AppContext>) -> Result<RoutingRulesResponse, String> {
    let state = ctx.state.lock().await;
    Ok(RoutingRulesResponse {
        rules: state.routing_rules.clone(),
        default_route: state.default_route.clone(),
    })
}
```

Replace `save_routing_rules` entirely (drop the `bridge` param, stop writing `state.bridge`, reconnect via the active interface):

```rust
#[tauri::command]
pub async fn save_routing_rules(
    rules: Vec<RoutingRule>,
    default_route: String,
    ctx: State<'_, AppContext>,
) -> Result<(), String> {
    let mut state = ctx.state.lock().await;
    state.routing_rules = rules;
    state.default_route = default_route;
    save_state(&state);
    drop(state);

    // Routing rules always affect the running config — reconnect if live.
    reconnect_active(&ctx).await;
    Ok(())
}
```

(`reconnect_active` is added in Task 3. Until then this won't compile — Task 3 follows immediately. If you must keep Task 2 independently green, see the note below.)

> **Sequencing note:** `save_routing_rules` now calls `reconnect_active`, which is introduced in Task 3. To keep Task 2 compiling on its own, add the `reconnect_active` helper (Task 3 Step 3) in **this** task instead, and move its commit boundary — OR implement Tasks 2 and 3 back-to-back and commit once after Task 3's tests pass. The recommended approach: add `reconnect_active` here (copy it from Task 3 Step 3) so `cargo test` passes at the end of Task 2.

- [ ] **Step 7: Add `reconnect_active` (so this task compiles) and run migration on load**

Add the helper to `commands.rs` (near the other private helpers, e.g. after `save_state`):

```rust
/// Reconnect the running core so config changes take effect. No-op when the
/// core isn't running or there's no active server. Resolves the active
/// interface from current state.
async fn reconnect_active(ctx: &AppContext) {
    if !ctx.core.is_running().await {
        return;
    }
    let state = ctx.state.lock().await;
    let Some(id) = state.active_server_id.clone() else { return };
    let Some(server) = state.servers.iter().find(|s| s.id == id).cloned() else { return };
    let tun_mode = state.settings.vpn_mode == "tun";
    let rules = state.routing_rules.clone();
    let dr = state.default_route.clone();
    let active_iface = state.active_interface().cloned();
    drop(state);
    if let Err(e) = ctx.core.start(&server, tun_mode, &rules, &dr, active_iface.as_ref()).await {
        log::error!("Failed to reconnect after config change: {}", e);
    }
}
```

Update `load_state` to run migration + dual-write after deserialize:

```rust
pub fn load_state() -> AppState {
    let path = state_path();
    let mut state = if path.exists() {
        match std::fs::read_to_string(&path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
            Err(_) => AppState::default(),
        }
    } else {
        AppState::default()
    };
    state.migrate_bridge_to_interfaces();
    state.sync_legacy_bridge();
    state
}
```

- [ ] **Step 8: Run all backend tests**

Run: `cd src-tauri && cargo test`
Expected: PASS — the rewritten `core::singbox` tests pass, `proxy::models` tests (Task 1) pass, and the whole crate compiles (all `start` call sites updated).

- [ ] **Step 9: Commit**

```bash
git add src-tauri/src/core/singbox.rs src-tauri/src/core/manager.rs src-tauri/src/commands.rs
git commit -m "feat(interfaces): generate config from active interface; drop bridge from routing

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_011btTtMPPUr9HEUYTvV6cUZ"
```

---

## Task 3: Backend — interface CRUD commands + mutation methods

**Files:**
- Modify: `src-tauri/src/proxy/models.rs` (add a second `impl AppState` with `upsert_interface`/`delete_interface`/`set_active`; tests)
- Modify: `src-tauri/src/commands.rs` (add `InterfacesResponse` + four commands)
- Modify: `src-tauri/src/lib.rs` (register the four commands)

**Interfaces:**
- Consumes (from Tasks 1-2): `InterfaceConfig`, `AppState::sync_legacy_bridge`, `AppState::active_interface`, `reconnect_active`.
- Produces (Tauri commands consumed by frontend Task 4): `get_interfaces() -> InterfacesResponse { interfaces, active_interface_id }`; `save_interface(config: InterfaceConfig)`; `delete_interface(id: String)`; `set_active_interface(id: Option<String>)`. Plus `AppState::upsert_interface(&mut self, InterfaceConfig) -> bool` (true = touched active), `AppState::delete_interface(&mut self, &str) -> bool` (true = was active), `AppState::set_active(&mut self, Option<String>)`.

- [ ] **Step 1: Write the failing tests for the mutation methods**

In `src-tauri/src/proxy/models.rs` `mod tests`, add:

```rust
    fn ic(id: &str, name: &str) -> InterfaceConfig {
        InterfaceConfig { id: id.into(), label: name.into(), interface: name.into(), routing_mark: None, endpoints: vec![] }
    }

    #[test]
    fn upsert_new_mints_id_and_is_not_active() {
        let mut s = AppState::default();
        let touched = s.upsert_interface(InterfaceConfig {
            id: "".into(), label: "".into(), interface: "awg0".into(), routing_mark: None, endpoints: vec![],
        });
        assert!(!touched);
        assert_eq!(s.interfaces.len(), 1);
        assert!(!s.interfaces[0].id.is_empty());
        assert_eq!(s.interfaces[0].label, "awg0"); // label filled from interface
    }

    #[test]
    fn upsert_edit_of_active_returns_true() {
        let mut s = AppState { interfaces: vec![ic("a", "awg0")], active_interface_id: Some("a".into()), ..Default::default() };
        let touched = s.upsert_interface(InterfaceConfig {
            id: "a".into(), label: "Renamed".into(), interface: "awg1".into(), routing_mark: None, endpoints: vec![],
        });
        assert!(touched);
        assert_eq!(s.interfaces[0].interface, "awg1");
        assert_eq!(s.interfaces[0].label, "Renamed");
    }

    #[test]
    fn upsert_edit_of_nonactive_returns_false() {
        let mut s = AppState { interfaces: vec![ic("a", "awg0"), ic("b", "wg1")], active_interface_id: Some("a".into()), ..Default::default() };
        let touched = s.upsert_interface(ic("b", "wg2"));
        assert!(!touched);
        assert_eq!(s.interfaces[1].interface, "wg2");
    }

    #[test]
    fn delete_active_clears_and_returns_true() {
        let mut s = AppState { interfaces: vec![ic("a", "awg0")], active_interface_id: Some("a".into()), ..Default::default() };
        let was_active = s.delete_interface("a");
        assert!(was_active);
        assert!(s.interfaces.is_empty());
        assert!(s.active_interface_id.is_none());
    }

    #[test]
    fn delete_nonactive_keeps_active_returns_false() {
        let mut s = AppState { interfaces: vec![ic("a", "awg0"), ic("b", "wg1")], active_interface_id: Some("a".into()), ..Default::default() };
        let was_active = s.delete_interface("b");
        assert!(!was_active);
        assert_eq!(s.active_interface_id.as_deref(), Some("a"));
        assert_eq!(s.interfaces.len(), 1);
    }

    #[test]
    fn set_active_unknown_id_clears() {
        let mut s = AppState { interfaces: vec![ic("a", "awg0")], active_interface_id: Some("a".into()), ..Default::default() };
        s.set_active(Some("ghost".into()));
        assert!(s.active_interface_id.is_none());
        s.set_active(Some("a".into()));
        assert_eq!(s.active_interface_id.as_deref(), Some("a"));
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cd src-tauri && cargo test proxy::models`
Expected: FAIL to compile — `upsert_interface`/`delete_interface`/`set_active` not found.

- [ ] **Step 3: Add the mutation methods**

In `src-tauri/src/proxy/models.rs`, add a second `impl AppState` block (after the one from Task 1):

```rust
impl AppState {
    /// Insert or update an interface. Empty id => new (mint uuid). Fills `label`
    /// from `interface` when blank. Returns true if the change touched the
    /// currently-active interface (the caller reconnects only then — M3).
    pub fn upsert_interface(&mut self, mut config: InterfaceConfig) -> bool {
        if config.label.trim().is_empty() {
            config.label = config.interface.clone();
        }
        if config.id.is_empty() {
            config.id = uuid::Uuid::new_v4().to_string();
            self.interfaces.push(config);
            false
        } else {
            let touched_active = self.active_interface_id.as_deref() == Some(config.id.as_str());
            if let Some(slot) = self.interfaces.iter_mut().find(|i| i.id == config.id) {
                *slot = config;
            } else {
                self.interfaces.push(config);
            }
            touched_active
        }
    }

    /// Remove an interface by id. Clears the active selection if it pointed
    /// here. Returns true if the deleted interface was active.
    pub fn delete_interface(&mut self, id: &str) -> bool {
        let was_active = self.active_interface_id.as_deref() == Some(id);
        self.interfaces.retain(|i| i.id != id);
        if was_active {
            self.active_interface_id = None;
        }
        was_active
    }

    /// Set the active interface, ignoring an unknown id (clears instead).
    pub fn set_active(&mut self, id: Option<String>) {
        self.active_interface_id = match id {
            Some(id) if self.interfaces.iter().any(|i| i.id == id) => Some(id),
            _ => None,
        };
    }
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cd src-tauri && cargo test proxy::models`
Expected: PASS — all mutation-method tests plus Task 1's tests.

- [ ] **Step 5: Add the Tauri commands**

In `src-tauri/src/commands.rs`, add the response struct and four commands (near `get_routing_rules`):

```rust
#[derive(Serialize)]
pub struct InterfacesResponse {
    pub interfaces: Vec<InterfaceConfig>,
    pub active_interface_id: Option<String>,
}

#[tauri::command]
pub async fn get_interfaces(ctx: State<'_, AppContext>) -> Result<InterfacesResponse, String> {
    let state = ctx.state.lock().await;
    Ok(InterfacesResponse {
        interfaces: state.interfaces.clone(),
        active_interface_id: state.active_interface_id.clone(),
    })
}

#[tauri::command]
pub async fn save_interface(ctx: State<'_, AppContext>, config: InterfaceConfig) -> Result<(), String> {
    if config.interface.trim().is_empty() {
        return Err("Interface name is required".into());
    }
    let touched_active;
    {
        let mut state = ctx.state.lock().await;
        touched_active = state.upsert_interface(config);
        state.sync_legacy_bridge();
        save_state(&state);
    }
    if touched_active {
        reconnect_active(&ctx).await;
    }
    Ok(())
}

#[tauri::command]
pub async fn delete_interface(ctx: State<'_, AppContext>, id: String) -> Result<(), String> {
    let was_active;
    {
        let mut state = ctx.state.lock().await;
        was_active = state.delete_interface(&id);
        state.sync_legacy_bridge();
        save_state(&state);
    }
    if was_active {
        reconnect_active(&ctx).await;
    }
    Ok(())
}

#[tauri::command]
pub async fn set_active_interface(ctx: State<'_, AppContext>, id: Option<String>) -> Result<(), String> {
    {
        let mut state = ctx.state.lock().await;
        state.set_active(id);
        state.sync_legacy_bridge();
        save_state(&state);
    }
    // The active selection changed — always reconnect a live core.
    reconnect_active(&ctx).await;
    Ok(())
}
```

- [ ] **Step 6: Register the commands in lib.rs**

In `src-tauri/src/lib.rs`, add to the `tauri::generate_handler!` list (after `commands::save_routing_rules,`):

```rust
        commands::save_routing_rules,
        commands::get_interfaces,
        commands::save_interface,
        commands::delete_interface,
        commands::set_active_interface,
```

- [ ] **Step 7: Build and run all backend tests**

Run: `cd src-tauri && cargo test`
Expected: PASS — whole crate compiles, all `proxy::models` and `core::singbox` tests pass.

- [ ] **Step 8: Commit**

```bash
git add src-tauri/src/proxy/models.rs src-tauri/src/commands.rs src-tauri/src/lib.rs
git commit -m "feat(interfaces): add interface CRUD commands with reconnect-on-active

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_011btTtMPPUr9HEUYTvV6cUZ"
```

---

## Task 4: Frontend — API types and command wrappers

**Files:**
- Modify: `src/api/tauri.ts` (add types near `RoutingRulesResponse` line 84; add wrappers near `saveRoutingRules` line 158)

**Interfaces:**
- Consumes: the Tauri commands from Task 3.
- Produces: `InterfaceConfig` type, `InterfacesResponse` type, and `api.getInterfaces`/`api.saveInterface`/`api.deleteInterface`/`api.setActiveInterface`. Consumed by Tasks 5, 6, 7.

- [ ] **Step 1: Add the types**

In `src/api/tauri.ts`, after the `RoutingRulesResponse` interface (line 88), add:

```ts
export interface InterfaceConfig {
  id: string;
  label: string;
  interface: string;
  routing_mark: number | null;
  endpoints: string[];
}

export interface InterfacesResponse {
  interfaces: InterfaceConfig[];
  active_interface_id: string | null;
}
```

- [ ] **Step 2: Add the command wrappers**

In the `api` object, after the `saveRoutingRules` wrapper (line 158-159), add:

```ts
  getInterfaces: () => invoke<InterfacesResponse>("get_interfaces"),

  saveInterface: (config: InterfaceConfig) =>
    invoke<void>("save_interface", { config }),

  deleteInterface: (id: string) =>
    invoke<void>("delete_interface", { id }),

  setActiveInterface: (id: string | null) =>
    invoke<void>("set_active_interface", { id }),
```

- [ ] **Step 3: Type-check**

Run: `npm run build`
Expected: PASS — additive types/wrappers, no consumers broken.

- [ ] **Step 4: Commit**

```bash
git add src/api/tauri.ts
git commit -m "feat(interfaces): add interface API types and command wrappers

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_011btTtMPPUr9HEUYTvV6cUZ"
```

---

## Task 5: Frontend — AppContext state, action, bootstrap

**Files:**
- Modify: `src/context/AppContext.tsx` (import line 8; `AppState` lines 17-36; `initialState` ~68; `Action` union ~94; reducer ~169; bootstrap ~212-228)

**Interfaces:**
- Consumes (from Task 4): `InterfaceConfig`, `InterfacesResponse`, `api.getInterfaces`.
- Produces: `state.interfaces: InterfaceConfig[]`, `state.activeInterfaceId: string | null`, and the `SET_INTERFACES` action. Consumed by Tasks 6, 7. (The `Page` union gains `"interfaces"` in Task 6, not here.)

- [ ] **Step 1: Import the type**

In `src/context/AppContext.tsx`, add `InterfaceConfig` to the existing import from `../api/tauri` (the import that already brings in `RoutingRule`, line ~8). For example:

```ts
import {
  // ...existing imports...
  RoutingRule,
  InterfaceConfig,
  // ...
} from "../api/tauri";
```

- [ ] **Step 2: Add the state fields**

In the `AppState` interface, after `bridge: BridgeConfig;` (line 30), add:

```ts
  bridge: BridgeConfig;
  interfaces: InterfaceConfig[];
  activeInterfaceId: string | null;
```

(Only the two new lines are added; `bridge` stays for now — removed in Task 7.)

- [ ] **Step 3: Add to initialState**

In `initialState`, after the `bridge: { interface: null, routing_mark: null, endpoints: [] },` line (~70), add:

```ts
  bridge: { interface: null, routing_mark: null, endpoints: [] },
  interfaces: [],
  activeInterfaceId: null,
```

- [ ] **Step 4: Add the action variant**

In the `Action` union, after the `SET_ROUTING_RULES` line (94), add:

```ts
  | { type: "SET_INTERFACES"; interfaces: InterfaceConfig[]; activeInterfaceId: string | null }
```

- [ ] **Step 5: Add the reducer case**

In the reducer `switch`, after the `SET_ROUTING_RULES` case (lines 169-170), add:

```ts
    case "SET_INTERFACES":
      return { ...state, interfaces: action.interfaces, activeInterfaceId: action.activeInterfaceId };
```

- [ ] **Step 6: Load interfaces during bootstrap**

In the bootstrap `useEffect` `Promise.all` (lines 217-224), add `api.getInterfaces()` and a matching destructure entry, then dispatch:

```ts
      const [servers, status, settings, subs, routing, interfaces, onboardingDone] = await Promise.all([
        api.getServers(),
        api.getStatus(),
        api.getSettings(),
        api.getSubscriptions(),
        api.getRoutingRules(),
        api.getInterfaces(),
        api.getOnboardingCompleted(),
      ]);
```

And after the `SET_ROUTING_RULES` dispatch (line 227), add:

```ts
      dispatch({ type: "SET_INTERFACES", interfaces: interfaces.interfaces, activeInterfaceId: interfaces.active_interface_id });
```

> Note: `dispatch({ type: "SET_ROUTING_RULES", ... bridge: routing.bridge })` still references `routing.bridge`. The backend (Task 2) no longer returns `bridge`, so at runtime this is `undefined` — harmless for `tsc` (the frontend `RoutingRulesResponse` type still has `bridge` until Task 7). This is the transition window; do not launch the app yet.

- [ ] **Step 7: Type-check**

Run: `npm run build`
Expected: PASS.

- [ ] **Step 8: Commit**

```bash
git add src/context/AppContext.tsx
git commit -m "feat(interfaces): add interfaces state + bootstrap to AppContext

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_011btTtMPPUr9HEUYTvV6cUZ"
```

---

## Task 6: Frontend — Interfaces page, modal, sidebar, route, i18n

**Files:**
- Create: `src/components/interfaces/InterfacesPage.tsx`
- Create: `src/components/interfaces/InterfaceModal.tsx`
- Modify: `src/components/ui/Icons.tsx` (add `NetworkIcon`)
- Modify: `src/context/AppContext.tsx` (`Page` union line 15)
- Modify: `src/components/layout/Sidebar.tsx` (import; `navIcons` 13-20; `navItems` 27-34)
- Modify: `src/App.tsx` (import; `renderPage` switch 117-138)
- Modify: `src/i18n/translations.ts` (add `nav.interfaces` + `interfaces.*`)

**Interfaces:**
- Consumes (Tasks 4, 5): `api.getInterfaces/saveInterface/deleteInterface/setActiveInterface`, `InterfaceConfig`, `state.interfaces`, `state.activeInterfaceId`, `SET_INTERFACES`.
- Produces: a working Interfaces page reachable from the sidebar. No exports consumed by later tasks.

- [ ] **Step 1: Add the i18n keys**

In `src/i18n/translations.ts`, add a `nav.interfaces` key next to the other `nav.*` keys, and an `interfaces.*` block (English-only). Place the block near the `routing.bridge*` keys:

```ts
  "nav.interfaces": { en: "Interfaces" },
  "interfaces.title": { en: "Custom Interfaces" },
  "interfaces.add": { en: "Add interface" },
  "interfaces.addTitle": { en: "Add interface" },
  "interfaces.editTitle": { en: "Edit interface" },
  "interfaces.empty": { en: "No custom interfaces yet. Add one to route the \"Interface\" action into an external tunnel." },
  "interfaces.use": { en: "Use" },
  "interfaces.active": { en: "Active" },
  "interfaces.labelField": { en: "Label" },
  "interfaces.labelPlaceholder": { en: "e.g. Work AWG (defaults to the interface name)" },
  "interfaces.interfaceField": { en: "Interface name" },
  "interfaces.interfacePlaceholder": { en: "e.g. awg0" },
  "interfaces.endpointsField": { en: "Endpoint IPs to exclude (anti-loop)" },
  "interfaces.endpointsPlaceholder": { en: "comma-separated, e.g. 192.0.2.1, 198.51.100.7" },
  "interfaces.markField": { en: "Firewall mark (fwmark)" },
  "interfaces.markPlaceholder": { en: "optional, e.g. 51820" },
  "interfaces.errInterfaceRequired": { en: "Interface name is required" },
  "interfaces.errInterfaceFormat": { en: "Interface name must have no spaces" },
  "interfaces.errEndpoint": { en: "Invalid endpoint (use IP or CIDR)" },
  "interfaces.noActiveHint": { en: "A rule uses the \"Interface\" action but no interface is active. Open Interfaces to activate one." },
```

- [ ] **Step 2: Add the `NetworkIcon`**

In `src/components/ui/Icons.tsx`, add (following the existing icon pattern, e.g. `RouteIcon`):

```tsx
export function NetworkIcon({ size, color, className }: IconProps = defaults) {
  const s = size ?? 20;
  const c = color ?? "currentColor";
  return (
    <svg
      width={s}
      height={s}
      viewBox="0 0 24 24"
      fill="none"
      stroke={c}
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      className={className}
    >
      <rect x="9" y="2" width="6" height="6" rx="1" />
      <rect x="2" y="16" width="6" height="6" rx="1" />
      <rect x="16" y="16" width="6" height="6" rx="1" />
      <path d="M12 8v4M12 12H5v4M12 12h7v4" />
    </svg>
  );
}
```

- [ ] **Step 3: Extend the `Page` union**

In `src/context/AppContext.tsx` (line 15), add `"interfaces"`:

```ts
export type Page = "home" | "subscriptions" | "interfaces" | "settings" | "logs" | "stats" | "routing";
```

(This makes `navIcons: Record<Page, ReactNode>` in Sidebar and the `renderPage` switch in App.tsx require the new case — handled in Steps 4-5; until both are done, `tsc` will error, so do Steps 3-5 together before building.)

- [ ] **Step 4: Add the sidebar nav item**

In `src/components/layout/Sidebar.tsx`, add `NetworkIcon` to the icon import (line 4-6), add it to `navIcons` (between `subscriptions` and `routing`), and add the `navItems` entry:

```tsx
import {
  ZapIcon, FolderIcon, BarChartIcon, FileTextIcon, SettingsIcon, RouteIcon, NetworkIcon,
} from "../ui/Icons";
```

```tsx
const navIcons: Record<Page, ReactNode> = {
  home: <ZapIcon size={18} />,
  subscriptions: <FolderIcon size={18} />,
  interfaces: <NetworkIcon size={18} />,
  routing: <RouteIcon size={18} />,
  stats: <BarChartIcon size={18} />,
  logs: <FileTextIcon size={18} />,
  settings: <SettingsIcon size={18} />,
};
```

```tsx
const navItems: { id: Page; label: string }[] = [
  { id: "home", label: t("nav.home") },
  { id: "subscriptions", label: t("nav.subscriptions") },
  { id: "interfaces", label: t("nav.interfaces") },
  { id: "routing", label: t("nav.routing") },
  { id: "stats", label: t("nav.stats") },
  { id: "logs", label: t("nav.logs") },
  { id: "settings", label: t("nav.settings") },
];
```

- [ ] **Step 5: Route the page in App.tsx**

In `src/App.tsx`, add the import (with the other page imports near the top):

```tsx
import { InterfacesPage } from "./components/interfaces/InterfacesPage";
```

And add the case to the `renderPage` switch (after `case "subscriptions":`):

```tsx
    case "subscriptions":
      return <SubList />;
    case "interfaces":
      return <InterfacesPage />;
```

- [ ] **Step 6: Create the InterfaceModal**

Create `src/components/interfaces/InterfaceModal.tsx`:

```tsx
import { useState, useEffect } from "react";
import { Modal } from "../ui/Modal";
import { Button } from "../ui/Button";
import { Spinner } from "../ui/Spinner";
import { useApp } from "../../context/AppContext";
import { api, InterfaceConfig } from "../../api/tauri";
import { t } from "../../i18n/translations";

interface Props {
  open: boolean;
  onClose: () => void;
  /** The interface to edit, or null to add a new one. */
  editing: InterfaceConfig | null;
}

const parseEndpoints = (raw: string): string[] =>
  raw.split(/[\s,]+/).map((s) => s.trim()).filter((s) => s.length > 0);

// Loose IP / CIDR check: dotted-quad or IPv6-ish, optional /prefix.
const isEndpoint = (s: string): boolean =>
  /^[0-9a-fA-F:.]+(\/\d{1,3})?$/.test(s);

export function InterfaceModal({ open, onClose, editing }: Props) {
  const { dispatch, toast } = useApp();
  const [label, setLabel] = useState("");
  const [iface, setIface] = useState("");
  const [endpoints, setEndpoints] = useState("");
  const [mark, setMark] = useState("");
  const [loading, setLoading] = useState(false);

  // Re-seed the form whenever the modal opens (add vs edit).
  useEffect(() => {
    if (open) {
      setLabel(editing?.label ?? "");
      setIface(editing?.interface ?? "");
      setEndpoints(editing?.endpoints.join(", ") ?? "");
      setMark(editing?.routing_mark != null ? String(editing.routing_mark) : "");
    }
  }, [open, editing]);

  const handleSave = async () => {
    const interfaceName = iface.trim();
    if (!interfaceName) {
      toast(t("interfaces.errInterfaceRequired"), "error");
      return;
    }
    if (/\s/.test(interfaceName)) {
      toast(t("interfaces.errInterfaceFormat"), "error");
      return;
    }
    const eps = parseEndpoints(endpoints);
    const bad = eps.find((e) => !isEndpoint(e));
    if (bad) {
      toast(`${t("interfaces.errEndpoint")}: ${bad}`, "error");
      return;
    }
    const config: InterfaceConfig = {
      id: editing?.id ?? "",
      label: label.trim(),
      interface: interfaceName,
      routing_mark: mark.trim() === "" ? null : Number(mark),
      endpoints: eps,
    };
    setLoading(true);
    try {
      await api.saveInterface(config);
      const res = await api.getInterfaces();
      dispatch({ type: "SET_INTERFACES", interfaces: res.interfaces, activeInterfaceId: res.active_interface_id });
      onClose();
    } catch (e) {
      toast(`${e}`, "error");
    }
    setLoading(false);
  };

  return (
    <Modal open={open} onClose={onClose} title={editing ? t("interfaces.editTitle") : t("interfaces.addTitle")}>
      <div className="form-group">
        <label className="form-label">{t("interfaces.interfaceField")}</label>
        <input
          className="form-input"
          type="text"
          placeholder={t("interfaces.interfacePlaceholder")}
          value={iface}
          onChange={(e) => setIface(e.target.value)}
        />
      </div>
      <div className="form-group">
        <label className="form-label">{t("interfaces.labelField")}</label>
        <input
          className="form-input"
          type="text"
          placeholder={t("interfaces.labelPlaceholder")}
          value={label}
          onChange={(e) => setLabel(e.target.value)}
        />
      </div>
      <div className="form-group">
        <label className="form-label">{t("interfaces.endpointsField")}</label>
        <input
          className="form-input"
          type="text"
          placeholder={t("interfaces.endpointsPlaceholder")}
          value={endpoints}
          onChange={(e) => setEndpoints(e.target.value)}
        />
      </div>
      <div className="form-group">
        <label className="form-label">{t("interfaces.markField")}</label>
        <input
          className="form-input"
          type="number"
          placeholder={t("interfaces.markPlaceholder")}
          value={mark}
          onChange={(e) => setMark(e.target.value)}
        />
      </div>
      <div className="form-actions">
        <Button onClick={handleSave} disabled={loading}>
          {loading ? <Spinner size={14} /> : t("common.save")}
        </Button>
        <Button variant="ghost" onClick={onClose}>
          {t("common.cancel")}
        </Button>
      </div>
    </Modal>
  );
}
```

- [ ] **Step 7: Create the InterfacesPage**

Create `src/components/interfaces/InterfacesPage.tsx`:

```tsx
import { useState } from "react";
import { useApp } from "../../context/AppContext";
import { api, InterfaceConfig } from "../../api/tauri";
import { t } from "../../i18n/translations";
import { Button } from "../ui/Button";
import { InterfaceModal } from "./InterfaceModal";

export function InterfacesPage() {
  const { state, dispatch, toast } = useApp();
  void state.langTick;

  const [modalOpen, setModalOpen] = useState(false);
  const [editing, setEditing] = useState<InterfaceConfig | null>(null);

  const refresh = async () => {
    const res = await api.getInterfaces();
    dispatch({ type: "SET_INTERFACES", interfaces: res.interfaces, activeInterfaceId: res.active_interface_id });
  };

  const openAdd = () => {
    setEditing(null);
    setModalOpen(true);
  };

  const openEdit = (iface: InterfaceConfig) => {
    setEditing(iface);
    setModalOpen(true);
  };

  const toggleActive = async (id: string) => {
    const next = state.activeInterfaceId === id ? null : id;
    try {
      await api.setActiveInterface(next);
      await refresh();
    } catch (e) {
      toast(`${e}`, "error");
    }
  };

  const remove = async (id: string) => {
    try {
      await api.deleteInterface(id);
      await refresh();
    } catch (e) {
      toast(`${e}`, "error");
    }
  };

  return (
    <div className="sub-page">
      <div className="sub-header">
        <h2>{t("interfaces.title")}</h2>
        <Button onClick={openAdd}>{t("interfaces.add")}</Button>
      </div>

      {state.interfaces.length === 0 ? (
        <div className="empty-list">{t("interfaces.empty")}</div>
      ) : (
        <div className="sub-list">
          {state.interfaces.map((iface) => {
            const active = state.activeInterfaceId === iface.id;
            return (
              <div key={iface.id} className={`sub-card ${active ? "sub-featured" : ""}`}>
                <div className="sub-info">
                  <span className="sub-name">
                    {iface.label}
                    {active && <span className="sub-featured-badge">{t("interfaces.active")}</span>}
                  </span>
                  <span className="sub-url">{iface.interface}</span>
                  <span className="sub-meta">
                    {iface.endpoints.length} endpoints
                    {iface.routing_mark != null ? ` · fwmark ${iface.routing_mark}` : ""}
                  </span>
                </div>
                <div className="sub-actions">
                  <Button
                    variant={active ? "primary" : "secondary"}
                    size="sm"
                    onClick={() => toggleActive(iface.id)}
                  >
                    {t("interfaces.use")}
                  </Button>
                  <Button variant="secondary" size="sm" onClick={() => openEdit(iface)}>
                    {t("common.edit") /* see note below */}
                  </Button>
                  <Button variant="danger" size="sm" onClick={() => remove(iface.id)}>
                    {t("common.delete") /* see note below */}
                  </Button>
                </div>
              </div>
            );
          })}
        </div>
      )}

      <InterfaceModal open={modalOpen} onClose={() => setModalOpen(false)} editing={editing} />
    </div>
  );
}
```

> **Note on `common.edit` / `common.delete`:** verify these keys exist in `translations.ts`. If `common.edit` is absent, add `"common.edit": { en: "Edit" }`; if `common.delete` is absent, add `"common.delete": { en: "Delete" }` (alongside the existing `common.add`/`common.cancel`/`common.save`). Confirm the `Button` component accepts `size="sm"` and `variant` values `primary|secondary|danger|ghost` — the reference shows `.btn-sm`/`.btn-primary`/`.btn-secondary`/`.btn-danger`/`.btn-ghost` CSS classes exist; match the `Button` prop names to its actual signature in `src/components/ui/Button.tsx` (adjust `size`/`variant` prop usage if the component differs).

- [ ] **Step 8: Type-check and build**

Run: `npm run build`
Expected: PASS — new page compiles, `Page` exhaustiveness satisfied in Sidebar + App, all `t("interfaces.*")` keys resolve.

- [ ] **Step 9: Commit**

```bash
git add src/components/interfaces/ src/components/ui/Icons.tsx src/components/layout/Sidebar.tsx src/App.tsx src/context/AppContext.tsx src/i18n/translations.ts
git commit -m "feat(interfaces): add Interfaces page, modal, and sidebar entry

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_011btTtMPPUr9HEUYTvV6cUZ"
```

---

## Task 7: Frontend — remove bridge from RoutingPage/context/api; add no-active hint

**Files:**
- Modify: `src/api/tauri.ts` (`RoutingRulesResponse` 84-88; `saveRoutingRules` 158)
- Modify: `src/context/AppContext.tsx` (`AppState` `bridge`; `initialState`; `SET_ROUTING_RULES` action + reducer; bootstrap dispatch)
- Modify: `src/components/routing/RoutingPage.tsx` (import 3; `save` 65-82 and all callers; `parseEndpoints` 181-182; `setBridge` 184-185; bridge JSX block 230-266; dead `getLang`/`lang` 5/187)

**Interfaces:**
- Consumes (Task 5): `state.activeInterfaceId`.
- Produces: bridge fully removed from the routing path; RoutingPage shows a required "no active interface" hint. End of the feature's frontend.

- [ ] **Step 1: Remove `bridge` from the API contract**

In `src/api/tauri.ts`, change `RoutingRulesResponse` to drop `bridge`:

```ts
export interface RoutingRulesResponse {
  rules: RoutingRule[];
  default_route: string;
}
```

Change `saveRoutingRules` to drop the `bridge` parameter:

```ts
  saveRoutingRules: (rules: RoutingRule[], defaultRoute: string) =>
    invoke<void>("save_routing_rules", { rules, defaultRoute }),
```

(`BridgeConfig` may now be unused in `tauri.ts`. Leave the `BridgeConfig` type definition itself — `AppContext` still imports it until the next step removes that usage; remove the type only if nothing references it after Step 2. `tsc` flags unused imports, not unused exported types, so the exported `BridgeConfig` type can stay harmlessly.)

- [ ] **Step 2: Remove `bridge` from AppContext**

In `src/context/AppContext.tsx`:

- Remove `bridge: BridgeConfig;` from the `AppState` interface.
- Remove the `bridge: { interface: null, routing_mark: null, endpoints: [] },` line from `initialState`.
- Change the `SET_ROUTING_RULES` action variant to drop `bridge`:

```ts
  | { type: "SET_ROUTING_RULES"; rules: RoutingRule[]; defaultRoute: string }
```

- Change the reducer case:

```ts
    case "SET_ROUTING_RULES":
      return { ...state, routingRules: action.rules, defaultRoute: action.defaultRoute };
```

- Change the bootstrap dispatch to drop `bridge`:

```ts
      dispatch({ type: "SET_ROUTING_RULES", rules: routing.rules, defaultRoute: routing.default_route });
```

- Remove the now-unused `BridgeConfig` import from `../api/tauri` (keep `InterfaceConfig`).

- [ ] **Step 3: Remove the bridge block and rewire `save` in RoutingPage**

In `src/components/routing/RoutingPage.tsx`:

- Line 3 import: remove `BridgeConfig` (keep `api, RoutingRule, RuleAction`).
- Line 5 / line 187: remove the dead `getLang` import and `const lang = getLang();` line.
- Lines 181-182: remove the `parseEndpoints` function (it now lives in `InterfaceModal`).
- Lines 184-185: remove the `setBridge` helper.
- Lines 230-266: remove the entire `{/* Bridge / Custom interface routing settings */}` `<div className="settings-section">` block.
- Change the `save` callback to drop the `bridge` parameter:

```tsx
  const save = useCallback(
    (rules: RoutingRule[], defaultRoute: string) => {
      dispatch({ type: "SET_ROUTING_RULES", rules, defaultRoute });
      clearTimeout(saveTimer.current);
      saveTimer.current = setTimeout(async () => {
        try {
          await api.saveRoutingRules(rules, defaultRoute);
          toast(t("routing.saved"), "success");
        } catch (e) {
          toast(`${e}`, "error");
        }
      }, 800);
    },
    [dispatch, toast]
  );
```

- Update every `save(...)` caller to drop the third (`state.bridge`) argument. These are in `setDefaultRoute` (line ~85), `addRule` (~98/101), `removeRule` (~103-109), `toggleRule` (~110-118), `saveEdit` (from the editable-rules feature), and `addPreset` (~134/165). Each becomes `save(<rules>, <defaultRoute>)`. For example:

```tsx
  const setDefaultRoute = (route: string) => {
    save(state.routingRules, route);
  };
```

```tsx
  const removeRule = (id: string) => {
    save(
      state.routingRules.filter((r) => r.id !== id),
      state.defaultRoute
    );
  };
```

(Apply the same two-argument change to `addRule`, `toggleRule`, `saveEdit`, and `addPreset`.)

- [ ] **Step 4: Add the required "no active interface" hint**

In `RoutingPage.tsx`, compute whether any enabled rule uses the `Interface` action while nothing is active, and render a hint above the rules list (uses `state.activeInterfaceId` and `dispatch` to navigate). Add near the top of the component body:

```tsx
  const needsActiveIface =
    state.activeInterfaceId === null &&
    state.routingRules.some((r) => r.enabled && r.action === "bridge");
```

And render the hint inside the rules `settings-section`, just before the rules list (reuse the `vpn-mode-desc` muted style; make it a button that navigates to the Interfaces page):

```tsx
      {needsActiveIface && (
        <div
          className="vpn-mode-desc"
          style={{ cursor: "pointer", color: "var(--warning)" }}
          onClick={() => dispatch({ type: "SET_PAGE", page: "interfaces" })}
        >
          {t("interfaces.noActiveHint")}
        </div>
      )}
```

- [ ] **Step 5: Type-check and build**

Run: `npm run build`
Expected: PASS — no remaining references to `state.bridge`, `setBridge`, `BridgeConfig` (in routing/context), or `saveRoutingRules(..., bridge)`.

- [ ] **Step 6: Manual verification (run the full app — first time since Task 1)**

Run: `npm run tauri dev`

Verify:
1. **Migration:** if you have a v1.1.0 `state.json` with a `bridge.interface`, the Interfaces page shows one interface, marked **Active**, and the Routing page no longer shows interface fields.
2. **Add/edit/delete:** add an interface (blank label → label shows the interface name), edit it, delete it — list updates and persists across app restarts.
3. **Active toggle:** click **Use** to activate; click again to deactivate (`set_active_interface(null)`). The active card is visually marked.
4. **Routing follows active:** with an `Interface`-action rule and an active interface, traffic to that domain egresses via the interface; with no active interface, the hint appears and the rule falls back to proxy.
5. **Reconnect scope (M3):** while connected to a proxy server, editing a **non-active** interface does not drop the tunnel; switching the active interface (or editing the active one) reconnects.
6. **No regressions:** routing add/remove/toggle/edit and default-route still work.

- [ ] **Step 7: Commit**

```bash
git add src/api/tauri.ts src/context/AppContext.tsx src/components/routing/RoutingPage.tsx
git commit -m "feat(interfaces): remove bridge fields from routing; add no-active hint

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_011btTtMPPUr9HEUYTvV6cUZ"
```

---

## Task 8: Docs — multi-interface how-to (English)

**Files:**
- Modify: `README.md` (the `## 🔀 Custom Interface Routing` section)
- Modify: `docs/README.md` (add a "Custom Interface Routing" section to the hub)
- **Do not modify** `README_FA.md` — the user maintains the Persian README themselves.

**Interfaces:**
- Consumes: the final UX from Tasks 1-7.
- Produces: updated docs. No code consumers.

- [ ] **Step 1: Rewrite the README how-to for the multi-interface flow**

In `README.md`, replace the body of the `## 🔀 Custom Interface Routing` section with the new flow:

```markdown
## 🔀 Custom Interface Routing

Route selected domains into an externally-managed network interface (e.g. a
WireGuard/AmneziaWG tunnel brought up with `table = off`). IRBox never creates
or tears down the interface — it only routes into it.

1. Open the **Interfaces** page (sidebar) and **Add interface**:
   - **Interface name** — the bind target, e.g. `awg0` (required).
   - **Label** — a friendly name (defaults to the interface name).
   - **Endpoint IPs to exclude** — the tunnel server's IP/CIDRs, kept on
     `direct` to avoid a routing loop in TUN mode.
   - **Firewall mark (fwmark)** — optional SO_MARK for the interface egress.
2. Click **Use** on the interface to mark it **active**. Only the active
   interface receives traffic.
3. On the **Routing** page, add rules with the **Interface** action for the
   domains you want to send into that interface.
4. Connect. Matching domains egress via the active interface; everything else
   follows your default route. If a rule uses **Interface** but nothing is
   active, the Routing page shows a hint and the rule falls back to proxy.
```

- [ ] **Step 2: Add a Custom Interface Routing section to the docs hub**

In `docs/README.md`, add a "Custom Interface Routing" section covering the same workflow plus the OS-side setup and rationale:

```markdown
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
```

- [ ] **Step 3: Verify the docs build/links**

Run: `npm run build`
Expected: PASS (docs are markdown; this just confirms nothing else regressed). Manually confirm the README renders and the new sections read correctly.

- [ ] **Step 4: Commit**

```bash
git add README.md docs/README.md
git commit -m "docs: rewrite Custom Interface Routing for multi-interface flow

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_011btTtMPPUr9HEUYTvV6cUZ"
```

---

## Self-Review

**Spec coverage:**
- Data model (list + active id) → Task 1. `interface` required / `label` default → Task 1 + Task 3 (`upsert_interface`) + command validation (Task 3). `id` backend-minted → Task 1 (migration) + Task 3 (`upsert_interface`). Dangling active → `None` → Task 1 (`active_interface`).
- Migration (M1), zero-interface edge, dual-write → Task 1 (methods + tests) + Task 2 (`load_state`).
- sing-box generation (bridge outbound / anti-loop / Bridge mapping unchanged shape) → Task 2 + tests.
- Commands (get/save/delete/set-active, drop bridge from routing) → Task 2 (routing) + Task 3 (interfaces). Reconnect-only-on-active (M3) → Task 3 (`upsert`/`delete` return flags; `set_active` always) + tests.
- Frontend (sidebar, page, modal, context, api, RoutingPage cleanup, required no-active hint) → Tasks 4-7. Validation (D) → Task 6 modal. i18n English-only → Task 6.
- Docs (item A; README_FA left to user) → Task 8.
- Out of scope confirmed deferred: interface-only mode (B/M2/M4), liveness (C) — stated in Global Constraints, not implemented here.

**Placeholder scan:** No "TBD"/"handle errors"/"similar to". Two explicit verification notes (Task 2 `reconnect_active` ordering; Task 6 `common.edit`/`common.delete` keys + `Button` prop names) instruct the implementer to confirm-and-adjust against real signatures rather than guess — these are deliberate integration checks, not placeholders.

**Type consistency:** `InterfaceConfig` fields (`id, label, interface, routing_mark, endpoints`) identical across Rust (Task 1) and TS (Task 4). `active_interface: Option<&InterfaceConfig>` used consistently in `generate_config` (Task 2) and `CoreManager::start` (Task 2) and `reconnect_active` (Task 2). `SET_INTERFACES` payload shape (`interfaces`, `activeInterfaceId`) matches between action (Task 5), reducer (Task 5), and dispatch sites (Tasks 5/6). `InterfacesResponse`/`get_interfaces` returns `{ interfaces, active_interface_id }` (Task 3) consumed as `res.interfaces`/`res.active_interface_id` (Tasks 5/6). `save_interface` takes `config` (Task 3) ↔ wrapper sends `{ config }` (Task 4). `set_active_interface(id: Option<String>)` ↔ `setActiveInterface(id: string | null)` ↔ `{ id }` (Tasks 3/4).
