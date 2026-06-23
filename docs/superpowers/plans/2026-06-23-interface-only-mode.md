# Interface-Only Mode Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let the user "go live" with just an **active interface** and no proxy server selected — the core (sing-box) starts with no proxy outbound, default route `direct`, and `Interface`-action rules routing into the active interface.

**Architecture:** Thread `Option<&Server>` through the core-start path (`singbox::generate_config`, `CoreManager::start`) so the config builder can omit the proxy outbound; force sing-box and reject xray/custom when there's no server; widen the `connect` command to `Option<String>`; replace `reconnect_active` with a `reconcile_core` helper that stops the core when nothing is left to route (M4). The Connect button enables when a server is selected **or** an active interface exists.

**Tech Stack:** Rust + Tauri 2 (crate `irbox`), `serde_json`. React 18 + TypeScript (Vite), `useReducer` context.

## Global Constraints

- **Interface-only mode is sing-box only.** When no server is passed, `CoreManager::start` forces `CoreType::SingBox` regardless of `selected_core`; the xray generator requires a server (errors otherwise); the Custom-protocol path only runs when a server is present. (xray degrades `Bridge → "proxy"` and Custom owns its outbounds — neither can route into a bridge with no proxy.)
- **Interface-only sing-box config shape:** no proxy outbound; `final` route `direct`; `RuleAction::Proxy` rules degrade to `direct`; `RuleAction::Bridge` rules route to the `bridge` outbound (an active interface is the precondition); anti-loop endpoints still apply; DNS `detour` points at `direct` instead of `proxy`. **This is selected by `server == None`** — the existing proxy shape is unchanged when a server is present.
- **System proxy is still set in non-TUN mode** for interface-only too (so app traffic reaches sing-box, where matched domains go to the interface and the rest go direct). `connect`'s `if !tun_mode { set_system_proxy }` is unchanged.
- **M4 — stop, don't reconnect-into-empty:** deleting or deactivating the active interface while interface-only-live (no server) leaves nothing to route. `reconcile_core` must **stop the core** (and unset the system proxy), mirroring `disconnect`, not restart into an empty config.
- **Connect enablement:** the Connect button is enabled when a proxy server is selected **OR** an active interface exists. Default route in interface-only mode is **`direct`** (resolved in the spec).
- **Keep internal `bridge`/`Bridge` naming.** No JS test runner — frontend gate is `npm run build`; backend gate is `cd src-tauri && cargo test`. New UI i18n keys are **English-only**. `README_FA.md` is maintained by the user — do not touch it.
- **Out of scope (later):** interface liveness/status indicator (item C). Auto-reconnect stays proxy-only (its `selectedServerId` guard is left as-is). "Both" mode (server + active interface) displays as the proxy server; distinguishing it in the UI is not required.

---

## File Structure

**Backend:**
- `src-tauri/src/core/singbox.rs` — **modify.** `generate_config` takes `Option<&Server>`; omit proxy outbound + force-direct shape when `None`; update tests; add interface-only tests.
- `src-tauri/src/core/manager.rs` — **modify.** `CoreManager::start` takes `Option<&Server>`; force sing-box when `None`; guard the Custom path; xray requires a server; guard the log line.
- `src-tauri/src/commands.rs` — **modify.** `connect` takes `Option<String>` + interface-only branch; `reconnect_active` → `reconcile_core` (interface-only reconnect + M4 stop); widen the four call sites.

**Frontend:**
- `src/api/tauri.ts` — **modify.** `connect` accepts `string | null`.
- `src/components/home/StatusPanel.tsx` — **modify.** Button enablement, `handleToggle`, interface-only status label.
- `src/context/AppContext.tsx` — no change (already holds `interfaces`/`activeInterfaceId`).
- `src/i18n/translations.ts` — **modify.** `status.interfaceOnly`, `servers.selectFirstOrInterface`.

**Docs:**
- `README.md`, `docs/README.md` — **modify.** Note interface-only flow. `README_FA.md` untouched.

---

## Task 1: Backend — core-start path accepts no server (`Option<&Server>`)

**Files:**
- Modify: `src-tauri/src/core/singbox.rs` (sig line 7; internals lines 8-234; tests)
- Modify: `src-tauri/src/core/manager.rs` (sig line 116; core_type line 119; Custom guard line 136; config gen lines 342-345; log lines 351-355; 4 call sites widened — see Step 9)

**Interfaces:**
- Consumes: `InterfaceConfig`, `active_interface` (existing). `build_outbound(server: &Server)` (unchanged, called only when `Some`).
- Produces: `singbox::generate_config(server: Option<&Server>, socks_port, http_port, tun_mode, routing_rules, default_route, active_interface)`; `CoreManager::start(server: Option<&Server>, tun_mode, routing_rules, default_route, active_interface)`. Consumed by Task 2.

This task widens the types and adds the interface-only config shape, but **interface-only is not reachable yet** (all call sites still pass `Some(...)`), so existing behavior is preserved. Follow TDD: update the existing sing-box tests + add the interface-only tests first, watch them fail, then implement.

- [ ] **Step 1: Update existing sing-box test calls + add interface-only tests**

In `src-tauri/src/core/singbox.rs` `#[cfg(test)] mod tests`, every existing test calls `generate_config(&test_server(), ...)`. Change each of those calls to `generate_config(Some(&test_server()), ...)` (7 call sites — `no_bridge_outbound_when_no_active_interface`, `bridge_outbound_emitted_with_bind_interface_and_mark`, `bridge_outbound_omits_mark_when_unset`, `antiloop_rule_precedes_user_rules`, `no_antiloop_rule_when_endpoints_empty`, `bridge_rule_routes_to_bridge_when_active`, `bridge_rule_falls_back_to_proxy_when_no_active`).

Then add these new tests (they reuse the existing `iface()` and `outbound_tags()` helpers):

```rust
    #[test]
    fn interface_only_omits_proxy_outbound() {
        let i = iface("awg0", None, vec![]);
        let cfg = generate_config(None, 1080, 1081, false, &[], "proxy", Some(&i)).unwrap();
        let tags = outbound_tags(&cfg);
        assert!(!tags.contains(&"proxy".to_string()));
        assert!(tags.contains(&"direct".to_string()));
        assert!(tags.contains(&"bridge".to_string()));
    }

    #[test]
    fn interface_only_forces_direct_final_route() {
        let i = iface("awg0", None, vec![]);
        // default_route "proxy" must be overridden to "direct" when there's no server.
        let cfg = generate_config(None, 1080, 1081, false, &[], "proxy", Some(&i)).unwrap();
        assert_eq!(cfg["route"]["final"], "direct");
    }

    #[test]
    fn interface_only_proxy_rule_degrades_to_direct() {
        let i = iface("awg0", None, vec![]);
        let rules = vec![RoutingRule { id: "r".into(), domain: "example.com".into(), action: RuleAction::Proxy, enabled: true }];
        let cfg = generate_config(None, 1080, 1081, false, &rules, "proxy", Some(&i)).unwrap();
        let rule = cfg["route"]["rules"].as_array().unwrap().iter()
            .find(|r| r.get("domain_suffix").is_some()).unwrap();
        assert_eq!(rule["outbound"], "direct");
    }

    #[test]
    fn interface_only_bridge_rule_routes_to_bridge() {
        let i = iface("awg0", None, vec![]);
        let rules = vec![RoutingRule { id: "r".into(), domain: "example.com".into(), action: RuleAction::Bridge, enabled: true }];
        let cfg = generate_config(None, 1080, 1081, false, &rules, "proxy", Some(&i)).unwrap();
        let rule = cfg["route"]["rules"].as_array().unwrap().iter()
            .find(|r| r.get("domain_suffix").is_some()).unwrap();
        assert_eq!(rule["outbound"], "bridge");
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd src-tauri && cargo test core::singbox`
Expected: FAIL to compile — `generate_config` still expects `&Server`, so `Some(&test_server())` and `None` don't type-check.

- [ ] **Step 3: Change the `generate_config` signature**

`singbox.rs` line 7:

```rust
pub fn generate_config(server: Option<&Server>, socks_port: u16, http_port: u16, tun_mode: bool, routing_rules: &[RoutingRule], default_route: &str, active_interface: Option<&InterfaceConfig>) -> Result<Value> {
```

- [ ] **Step 4: Make the proxy outbound optional (line 8)**

Replace:

```rust
    let outbound = build_outbound(server)?;
```

with:

```rust
    // Proxy outbound only exists when a server is selected; interface-only mode omits it.
    let outbound: Option<Value> = match server {
        Some(s) => Some(build_outbound(s)?),
        None => None,
    };
```

- [ ] **Step 5: Guard the WireGuard endpoints block (lines 11 and 78-80)**

Change line 11 from:

```rust
    let endpoints = if server.protocol == Protocol::WireGuard {
```

to:

```rust
    let endpoints = if let Some(server) = server {
        if server.protocol == Protocol::WireGuard {
```

The inner body (lines 12-77) is unchanged. Then change the closing tail (currently lines 78-80):

```rust
    } else {
        None
    };
```

to (add one extra `} else { None }` to close the new outer `if let`):

```rust
        } else {
            None
        }
    } else {
        None
    };
```

- [ ] **Step 6: Point the TUN DNS detour at direct when there's no server**

Just before the `let dns = if tun_mode {` line (line 118), add:

```rust
    // In interface-only mode there is no "proxy" outbound for DNS to detour through.
    let dns_detour = if server.is_some() { "proxy" } else { "direct" };
```

Then in the TUN DNS block, change the `dns-remote` server's `"detour": "proxy"` (line 127) to:

```rust
                    "detour": dns_detour
```

- [ ] **Step 7: Degrade Proxy/Bridge rules and force the final route**

In the user-rules loop, replace the `RuleAction::Proxy` arm (lines 206-208):

```rust
            RuleAction::Proxy => {
                // No proxy outbound in interface-only mode — fall through to direct.
                let outbound = if server.is_some() { "proxy" } else { "direct" };
                route_rules.push(json!({ "domain_suffix": [domain], "outbound": outbound }));
            }
```

Replace the `RuleAction::Bridge` arm (lines 209-214):

```rust
            RuleAction::Bridge => {
                // Routes into the active interface's bridge outbound; without an active
                // interface, falls back to proxy (or direct in interface-only mode).
                let outbound = if active_interface.is_some() {
                    "bridge"
                } else if server.is_some() {
                    "proxy"
                } else {
                    "direct"
                };
                route_rules.push(json!({ "domain_suffix": [domain], "outbound": outbound }));
            }
```

Replace the `final_route` line (line 218):

```rust
    // Interface-only mode (no server) always finals to direct — nothing to proxy into.
    let final_route = if server.is_none() {
        "direct"
    } else if default_route == "direct" {
        "direct"
    } else {
        "proxy"
    };
```

- [ ] **Step 8: Omit the proxy outbound from the outbounds vec (lines 220-223)**

Replace:

```rust
    let mut outbounds = vec![
        outbound,
        json!({ "type": "direct", "tag": "direct" }),
    ];
```

with:

```rust
    let mut outbounds = vec![
        json!({ "type": "direct", "tag": "direct" }),
    ];
    if let Some(out) = outbound {
        outbounds.insert(0, out);
    }
```

(Proxy outbound stays first when present — preserving the existing `[proxy, direct, bridge?]` order; interface-only yields `[direct, bridge?]`.)

- [ ] **Step 9: Widen `CoreManager::start` and force sing-box for no-server**

In `src-tauri/src/core/manager.rs`, change the signature (line 116):

```rust
    pub async fn start(&self, server: Option<&Server>, tun_mode: bool, routing_rules: &[RoutingRule], default_route: &str, active_interface: Option<&InterfaceConfig>) -> Result<()> {
```

Replace the `core_type` read (line 119):

```rust
        // Interface-only mode (no server) always runs sing-box, regardless of the
        // user's selected core — xray/custom can't route into a bridge outbound.
        let core_type = if server.is_none() {
            CoreType::SingBox
        } else {
            self.core_type.lock().await.clone()
        };
```

Guard the Custom-protocol branch (line 136). Change:

```rust
        if server.protocol == Protocol::Custom {
```

to:

```rust
        if matches!(server, Some(s) if s.protocol == Protocol::Custom) {
            let server = server.expect("custom path requires a server");
```

(The `let server = ...expect(...)` rebinds `server` to `&Server` inside this branch, which is only entered when `server` is `Some`, so the rest of the Custom block is unchanged.)

Change the generator selection (lines 342-345) — `singbox::generate_config` now takes `Option`, so `server` passes through; xray requires a server:

```rust
            let config = match core_type {
                CoreType::SingBox => singbox::generate_config(server, socks_port, http_port, tun_mode, routing_rules, default_route, active_interface)?,
                CoreType::Xray => xray::generate_config(
                    server.ok_or_else(|| anyhow!("xray core requires a proxy server"))?,
                    socks_port, http_port, routing_rules, default_route,
                )?,
            };
```

Guard the log line (lines 351-355):

```rust
        let target = match server {
            Some(s) => format!("'{}' ({}:{})", s.name, s.address, s.port),
            None => "interface-only".to_string(),
        };
        log::info!(
            "Starting {:?}{} for {}",
            core_type, if tun_mode { " [TUN]" } else { "" }, target
        );
```

- [ ] **Step 10: Widen the four `start(...)` call sites to `Some(...)` (behavior preserved)**

In `src-tauri/src/commands.rs`, the existing call sites still resolve a concrete server — wrap each in `Some(&...)` so the crate compiles with the new signature. Interface-only is wired up in Task 2.

- `connect` (line 159): `ctx.core.start(&server, ...)` → `ctx.core.start(Some(&server), ...)`
- `save_settings` reconnect (line 550): `ctx.core.start(&s, ...)` → `ctx.core.start(Some(&s), ...)`
- `set_core_type` reconnect (line 294): `ctx.core.start(&s, ...)` → `ctx.core.start(Some(&s), ...)`
- `reconnect_active` (line 905): `ctx.core.start(&server, ...)` → `ctx.core.start(Some(&server), ...)`

(`xray.rs` is unchanged — it still takes `&Server`, and is only reached when a server is present.)

- [ ] **Step 11: Run all backend tests**

Run: `cd src-tauri && cargo test`
Expected: PASS — the updated + new `core::singbox` tests pass, `proxy::models` tests pass, whole crate compiles.

- [ ] **Step 12: Commit**

```bash
git add src-tauri/src/core/singbox.rs src-tauri/src/core/manager.rs src-tauri/src/commands.rs
git commit -m "feat(interface-only): core-start path accepts no server

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_011btTtMPPUr9HEUYTvV6cUZ"
```

---

## Task 2: Backend — interface-only connect + reconcile/stop (M4)

**Files:**
- Modify: `src-tauri/src/commands.rs` (`connect` 143-196; `reconnect_active` → `reconcile_core` 891-908; rename 4 call sites 693/725/740/754)

**Interfaces:**
- Consumes (Task 1): `CoreManager::start(server: Option<&Server>, ...)`.
- Produces: `connect(ctx, server_id: Option<String>)` (None ⇒ interface-only); `reconcile_core(ctx)` replacing `reconnect_active`. End of backend.

This is command-level wiring with no unit-test harness (commands need a live `CoreManager`); correctness of the emitted config is already covered by Task 1's tests. Verify by `cargo test` (compile + existing tests) and the manual checks in Task 3.

- [ ] **Step 1: Rework `connect` to accept `Option<String>`**

Replace the whole `connect` command (commands.rs lines 143-196) with:

```rust
/// Connect to a proxy server, an active interface, or both.
#[tauri::command]
pub async fn connect(ctx: State<'_, AppContext>, server_id: Option<String>) -> Result<StatusResponse, String> {
    let mut state = ctx.state.lock().await;

    // Resolve the proxy server when one was requested.
    let server = match &server_id {
        Some(id) => Some(
            state.servers.iter().find(|s| &s.id == id).cloned()
                .ok_or("Server not found")?,
        ),
        None => None,
    };
    let active_iface = state.active_interface().cloned();

    // Need at least one of: a proxy server, or an active interface.
    if server.is_none() && active_iface.is_none() {
        return Err("Select a proxy server or activate an interface first".into());
    }

    let tun_mode = state.settings.vpn_mode == "tun";
    let routing_rules = state.routing_rules.clone();
    let default_route = state.default_route.clone();

    ctx.core
        .start(server.as_ref(), tun_mode, &routing_rules, &default_route, active_iface.as_ref())
        .await
        .map_err(|e| format!("Failed to start core: {}", e))?;

    let http_port = ctx.core.http_port().await;

    // System proxy applies whenever we're not in TUN mode (so app traffic reaches
    // sing-box) — in proxy, interface-only, and both modes alike.
    if !tun_mode {
        proxy_setter::set_system_proxy("127.0.0.1", http_port)
            .map_err(|e| format!("Failed to set system proxy: {}", e))?;
    }

    // Record a session only when a proxy server is in use.
    if let Some(ref server) = server {
        let record = ConnectionRecord {
            server_name: server.name.clone(),
            server_address: format!("{}:{}", server.address, server.port),
            protocol: format!("{:?}", server.protocol).to_lowercase(),
            core_type: format!("{:?}", ctx.core.get_core_type().await),
            vpn_mode: if tun_mode { "tun".to_string() } else { "proxy".to_string() },
            connected_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
            disconnected_at: None,
            upload_bytes: 0,
            download_bytes: 0,
        };
        state.sessions.push(record);
    }

    state.active_server_id = server_id;
    save_state(&state);

    Ok(StatusResponse {
        connected: true,
        server_name: server.as_ref().map(|s| s.name.clone()),
        core_type: format!("{:?}", ctx.core.get_core_type().await),
        socks_port: ctx.core.socks_port().await,
        http_port,
    })
}
```

- [ ] **Step 2: Replace `reconnect_active` with `reconcile_core` (interface-only + M4 stop)**

Replace the whole `reconnect_active` helper (commands.rs lines 891-908) with:

```rust
/// Reconcile the running core with current state after a config change.
/// Restarts when there's still something to route (proxy / interface-only /
/// both); stops the core when nothing is left (no server, no active interface —
/// the M4 case), mirroring `disconnect`'s cleanup. No-op when not running.
async fn reconcile_core(ctx: &AppContext) {
    if !ctx.core.is_running().await {
        return;
    }
    let state = ctx.state.lock().await;
    let server = state.active_server_id.as_ref()
        .and_then(|id| state.servers.iter().find(|s| &s.id == id))
        .cloned();
    let active_iface = state.active_interface().cloned();
    let tun_mode = state.settings.vpn_mode == "tun";
    let rules = state.routing_rules.clone();
    let dr = state.default_route.clone();
    drop(state);

    // Nothing to route into any more — stop the core and clean up like disconnect.
    if server.is_none() && active_iface.is_none() {
        if let Err(e) = proxy_setter::unset_system_proxy() {
            log::error!("Failed to unset system proxy on stop: {}", e);
        }
        if let Err(e) = ctx.core.stop().await {
            log::error!("Failed to stop core: {}", e);
        }
        return;
    }

    if let Err(e) = ctx.core.start(server.as_ref(), tun_mode, &rules, &dr, active_iface.as_ref()).await {
        log::error!("Failed to reconcile core after config change: {}", e);
    }
}
```

- [ ] **Step 3: Rename the four `reconnect_active` call sites**

Change `reconnect_active(&ctx).await;` → `reconcile_core(&ctx).await;` at the four call sites (commands.rs lines 693 in `save_routing_rules`, 725 in `save_interface`, 740 in `delete_interface`, 754 in `set_active_interface`). The surrounding conditions are unchanged: `save_interface`/`delete_interface` still call it only when `touched_active`/`was_active`; `set_active_interface` and `save_routing_rules` still call it unconditionally.

- [ ] **Step 4: Build and run backend tests**

Run: `cd src-tauri && cargo test`
Expected: PASS — whole crate compiles; all `proxy::models` and `core::singbox` tests pass. (No new unit tests here — this is command wiring; the config shape is covered by Task 1 and the flow by Task 3's manual checks.)

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/commands.rs
git commit -m "feat(interface-only): connect with no server; reconcile/stop core on empty (M4)

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_011btTtMPPUr9HEUYTvV6cUZ"
```

---

## Task 3: Frontend — Connect enablement + interface-only status

**Files:**
- Modify: `src/api/tauri.ts` (`connect` wrapper, line 115)
- Modify: `src/components/home/StatusPanel.tsx` (button `disabled` ~69; `handleToggle` 36-58; status display 81-104)
- Modify: `src/i18n/translations.ts` (add two keys)

**Interfaces:**
- Consumes (Task 2): `connect` accepting a null server id; `state.interfaces`/`state.activeInterfaceId` (already in context).
- Produces: a Connect button usable with no server when an interface is active, and an interface-only status label.

- [ ] **Step 1: Widen the `connect` API wrapper**

In `src/api/tauri.ts`, change the `connect` wrapper (line 115):

```ts
  connect: (serverId: string | null) =>
    invoke<StatusResponse>("connect", { serverId }),
```

(Tauri maps `serverId` → the Rust `server_id: Option<String>`; `null` becomes `None`.)

- [ ] **Step 2: Add i18n keys**

In `src/i18n/translations.ts`, add (English-only), next to the existing `status.*` / `servers.*` keys:

```ts
  "status.interfaceOnly": { en: "Interface-only" },
  "servers.selectFirstOrInterface": { en: "Select a server or activate an interface first" },
```

- [ ] **Step 3: Enable the Connect button when a server OR an active interface exists**

In `src/components/home/StatusPanel.tsx`, add a derived flag near the top of the component (after the existing `selectedServer` lookup ~line 62):

```tsx
  const canConnect = !!state.selectedServerId || !!state.activeInterfaceId;
```

Change the button's `disabled` prop (line 69) from `disabled={loading}` to:

```tsx
  disabled={loading || (!state.connected && !canConnect)}
```

- [ ] **Step 4: Update `handleToggle`'s connect branch**

Replace the connect branch of `handleToggle` (StatusPanel.tsx lines 44-52) — currently:

```tsx
      if (!state.selectedServerId) {
        toast(t("servers.selectFirst"), "error");
        setLoading(false);
        return;
      }
      const status = await api.connect(state.selectedServerId);
      dispatch({ type: "SET_STATUS", status });
      toast(`${t("toast.connectedTo")} ${status.server_name}`, "success");
```

with:

```tsx
      if (!state.selectedServerId && !state.activeInterfaceId) {
        toast(t("servers.selectFirstOrInterface"), "error");
        setLoading(false);
        return;
      }
      const status = await api.connect(state.selectedServerId);
      dispatch({ type: "SET_STATUS", status });
      const label =
        status.server_name ??
        state.interfaces.find((i) => i.id === state.activeInterfaceId)?.label ??
        t("status.interfaceOnly");
      toast(`${t("toast.connectedTo")} ${label}`, "success");
```

(`state.selectedServerId` is `string | null`; passing `null` triggers interface-only mode in the backend.)

- [ ] **Step 5: Show an interface-only label in the status display**

In the status display (StatusPanel.tsx lines 83-87), the connected branch currently shows the server name. Add an interface-only branch after it:

```tsx
      {state.connected && state.serverName && (
        <div className="status-server">{state.serverName}</div>
      )}
      {state.connected && !state.serverName && state.activeInterfaceId && (
        <div className="status-server">
          {t("status.interfaceOnly")} ·{" "}
          {state.interfaces.find((i) => i.id === state.activeInterfaceId)?.label ?? ""}
        </div>
      )}
```

(In interface-only mode the backend returns `server_name: null`, so the first branch is skipped and this one shows the active interface's label. "Both" mode returns the server name and shows as proxy — acceptable per the constraints.)

- [ ] **Step 6: Type-check and build**

Run: `npm run build`
Expected: PASS — `tsc` accepts the widened `connect` type and the new keys/branches.

- [ ] **Step 7: Manual verification (run the app)**

Run: `npm run tauri dev`

Verify:
1. **Interface-only connect:** with no server selected but an interface active, the Connect button is **enabled**; pressing it connects (no server). The status shows "Interface-only · <label>". Matching `Interface`-action domains egress via the interface; the rest go direct.
2. **No server, no interface:** Connect is **disabled** (nothing to connect to).
3. **Proxy still works:** select a server (no active interface) → connects as before, status shows the server name.
4. **Both:** server selected + interface active → connects; `Interface` rules route into the interface, other traffic via proxy/default; status shows the server.
5. **M4 stop:** while interface-only-live, deactivate (Use → off) or delete the active interface → the core **stops** within ~5s (status flips to Disconnected), system proxy is cleared, internet still works. It does **not** hang in a broken connected state.
6. **Disconnect:** the Disconnect button stops an interface-only session cleanly.

- [ ] **Step 8: Commit**

```bash
git add src/api/tauri.ts src/components/home/StatusPanel.tsx src/i18n/translations.ts
git commit -m "feat(interface-only): connect with no server from the UI; interface-only status

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_011btTtMPPUr9HEUYTvV6cUZ"
```

---

## Task 4: Docs — interface-only flow

**Files:**
- Modify: `README.md` (`## 🔀 Custom Interface Routing`), `docs/README.md` (`## Custom Interface Routing`)
- **Do not touch** `README_FA.md`.

**Interfaces:** Consumes the final behavior. No code consumers.

- [ ] **Step 1: Note interface-only in the README how-to**

In `README.md`, in the `## 🔀 Custom Interface Routing` section, add a paragraph after the existing steps:

```markdown
**Interface-only mode (no proxy server):** you don't need a proxy server to use
an interface. With an interface marked **active** and **no server selected**,
press **Connect** — IRBox runs in interface-only mode: matching `Interface`
rules route into the active interface and everything else goes direct. The
status shows "Interface-only · <label>". Deactivating or deleting the active
interface while live stops the tunnel.
```

- [ ] **Step 2: Note interface-only in the docs hub**

In `docs/README.md`, in the `## Custom Interface Routing` section, add:

```markdown
**Interface-only mode:** with an active interface and no proxy server selected,
Connect starts sing-box with no proxy outbound and a `direct` default route —
only `Interface`-action rules are sent into the interface, everything else stays
direct. This mode always uses the sing-box core (xray/custom cannot route into a
bridge outbound). Removing or deactivating the active interface while connected
stops the core.
```

- [ ] **Step 3: Sanity build + commit**

Run: `npm run build` (Expected: PASS — confirms nothing else regressed.)

```bash
git add README.md docs/README.md
git commit -m "docs: document interface-only mode

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_011btTtMPPUr9HEUYTvV6cUZ"
```

---

## Self-Review

**Spec coverage (item B):**
- "Core starts in interface-only mode (no proxy outbound, default route direct, bridge bound to active interface, Interface rules route into it, anti-loop endpoints apply)" → Task 1 Steps 4-8 + tests.
- "Gate interface-only to sing-box; xray/custom dead-end" → Task 1 Step 9 (force sing-box when `None`; xray requires a server; Custom guarded).
- "Connect button enabled if a server is selected OR an active interface exists; status reflects the mode" → Task 3 Steps 3-5.
- "`connect` accepts no server + active interface" → Task 2 Step 1.
- "M4: deleting/deactivating the active interface while interface-only-live stops the core" → Task 2 Step 2 (`reconcile_core` stop branch) + Task 3 Step 7 #5.
- "Default route interface-only = direct" → Task 1 Step 7 (`final_route` forced direct when no server).
- Out of scope: liveness (C) — not implemented, per Global Constraints.

**Placeholder scan:** No "TBD"/"handle errors"/"similar to". Backend has TDD tests for the config shape (Task 1); Task 2 is command wiring with documented manual verification (no command-level unit harness exists, consistent with the repo).

**Type consistency:** `generate_config(server: Option<&Server>, ...)` (Task 1) ↔ `CoreManager::start(server: Option<&Server>, ...)` (Task 1) ↔ `start(server.as_ref(), ...)` / `start(Some(&server), ...)` call sites (Tasks 1-2). `connect(server_id: Option<String>)` (Task 2) ↔ `connect: (serverId: string | null)` wrapper (Task 3) ↔ `api.connect(state.selectedServerId)` where `selectedServerId: string | null`. `reconcile_core` replaces `reconnect_active` at all four sites (Task 2). Interface-only status keys `status.interfaceOnly` / `servers.selectFirstOrInterface` defined (Task 3 Step 2) and used (Steps 4-5).
