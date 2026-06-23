# Spec: Route selected traffic into an external network interface ("bridge")

> **Status:** approved design, not yet implemented.
> **Supersedes:** `docs/bridge-routing-draft.md` (the original AWG-specific draft).
> **Verified against codebase:** 2026-06-23 — all referenced symbols and line
> ranges confirmed present with no drift.

## Goal

Let IRBox split-route selected traffic *into* a network interface that is
brought up and owned **outside** IRBox (e.g. an AmneziaWG/WireGuard tunnel
created with `table = off`, but any interface works). IRBox never creates,
owns, or tears down the interface — it only routes into it via a sing-box
`direct` outbound bound to that interface.

**General-purpose:** the feature is interface-agnostic. AWG is the motivating
example, but it does not matter what kind of tunnel the interface is or how it
was created. The internal code uses the name `bridge`; the user-facing label is
**"Interface" / "Custom interface routing"**.

## Why this approach

IRBox does not implement WireGuard itself. WireGuard runs inside the bundled
upstream sing-box binary (SagerNet, downloaded per-target by `cores.sh`); IRBox
only generates the sing-box config (`core/singbox.rs`). xray-core refuses
WireGuard outright (`core/xray.rs`).

sing-box is a rule→outbound router, and a `direct` outbound supports
`bind_interface` / `routing_mark`. So we can route domains to a `direct`
outbound pinned to the external interface — split routing, done by sing-box,
into an interface it never created. This **avoids forking/patching sing-box**.

**Scope:** solid on **Linux**; `bind_interface` works on Windows/macOS too, but
managing an external `table = off` interface there is the user's responsibility
— treat those platforms as best-effort.

**Out of scope (possible future phase, own spec):** IRBox launching/managing the
external tunnel lifecycle (would require a process/interface module sibling to
`singbox.rs`/`xray.rs`, plus privilege escalation). This spec keeps the feature
a pure config feature — the interface is managed externally.

## Naming convention

| Layer | Term |
|-------|------|
| Rust enum / struct / outbound tag | `RuleAction::Bridge`, `BridgeConfig`, outbound tag `"bridge"` |
| Serde / TS action value | `"bridge"` |
| User-facing label (UI + docs) | **"Interface"** (action), **"Custom interface routing"** (settings block) |

---

## Design summary

- New global config `BridgeConfig { interface, routing_mark, endpoints }`, stored
  in `AppState` next to `default_route`, threaded through `CoreManager::start`
  → `singbox::generate_config` exactly like `default_route`.
- New `RuleAction::Bridge` — routes a domain rule to the `bridge` outbound
  (falls back to `proxy` if no interface is configured).
- A `bridge` `direct` outbound emitted only when `interface` is set, with
  `bind_interface` (+ optional `routing_mark`).
- **Anti-loop guard:** if `endpoints` is non-empty, emit a high-priority
  `{ ip_cidr: endpoints, outbound: "direct" }` rule *before* user rules, so the
  tunnel's own handshake/data packets to its server(s) are not captured back
  into sing-box in TUN mode. `endpoints` is a **list** (supports multi-peer
  tunnels).

---

## Patch (per file)

### 1. `src-tauri/src/proxy/models.rs`

`RuleAction` (currently lines 298–306) — add the `Bridge` variant:

```rust
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
```

New struct (place before `AppState`):

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
    /// loop in TUN mode (the tunnel's own handshake must not re-enter sing-box).
    /// A list — supports multi-peer tunnels.
    #[serde(default)]
    pub endpoints: Vec<String>,
}
```

`AppState` (currently lines 333–349) — add `bridge` next to `default_route`:

```rust
    #[serde(default = "default_route")]
    pub default_route: String,
    #[serde(default)]
    pub bridge: BridgeConfig,
    #[serde(default)]
    pub onboarding_completed: bool,
```

### 2. `src-tauri/src/core/singbox.rs`

Signature (currently line 7) — add the `bridge` param:

```rust
pub fn generate_config(
    server: &Server, socks_port: u16, http_port: u16, tun_mode: bool,
    routing_rules: &[RoutingRule], default_route: &str,
    bridge: &BridgeConfig,                       // ← new
) -> Result<Value> {
```

Anti-loop guard — before the user-defined routing-rules loop (currently
lines 188–201). Must precede user rules (first match wins):

```rust
if bridge.interface.is_some() && !bridge.endpoints.is_empty() {
    route_rules.push(json!({ "ip_cidr": bridge.endpoints, "outbound": "direct" }));
}
```

Inside the rule match — add the `Bridge` arm:

```rust
            RuleAction::Bridge => {
                // Falls back to `proxy` if no bridge interface is configured.
                let outbound = if bridge.interface.is_some() { "bridge" } else { "proxy" };
                route_rules.push(json!({ "domain_suffix": [domain], "outbound": outbound }));
            }
```

Outbounds — replace the inline `outbounds` array literal (currently
lines 212–218) with a dynamically-built vec so `"bridge"` only appears when
configured:

```rust
    let mut outbounds = vec![
        outbound,
        json!({ "type": "direct", "tag": "direct" }),
    ];
    if let Some(ref iface) = bridge.interface {
        let mut bridge_out = json!({
            "type": "direct",
            "tag": "bridge",
            "bind_interface": iface,   // egress pinned to the external interface
        });
        if let Some(mark) = bridge.routing_mark {
            bridge_out["routing_mark"] = json!(mark);   // matches tunnel fwmark (Linux)
        }
        outbounds.push(bridge_out);
    }
    // ...then in the config json!{...}:  "outbounds": outbounds,
```

### 3. `src-tauri/src/core/manager.rs`

`CoreManager::start` (currently line 116) — add the `bridge` param:

```rust
    pub async fn start(&self, server: &Server, tun_mode: bool,
        routing_rules: &[RoutingRule], default_route: &str,
        bridge: &BridgeConfig,                   // ← new
    ) -> Result<()> {
```

`generate_config` call sites (currently lines 339–340):

```rust
        CoreType::SingBox => singbox::generate_config(
            server, socks_port, http_port, tun_mode,
            routing_rules, default_route, bridge)?,
        CoreType::Xray => xray::generate_config(
            server, socks_port, http_port, routing_rules, default_route)?,
```

`BridgeConfig` is in scope via the existing `use crate::proxy::models::*;`.

### 4. `src-tauri/src/core/xray.rs` (keep the match exhaustive)

Match block currently at lines 327–331:

```rust
        let tag = match rule.action {
            RuleAction::Direct => "direct",
            RuleAction::Block  => "block",
            RuleAction::Proxy  => "proxy",
            // Bridge (external-interface routing) is sing-box only; degrade to proxy.
            RuleAction::Bridge => "proxy",
        };
```

### 5. `src-tauri/src/commands.rs`

Thread bridge config at **all four** `ctx.core.start(...)` call sites
(connect ~158, set_core_type reconnect ~292, save_settings reconnect ~547,
save_routing_rules reconnect ~697):

```rust
    let bridge = state.bridge.clone();
    // ...
        .start(&server, tun_mode, &routing_rules, &default_route, &bridge)
```

Extend the get/save commands so the UI can persist it. `RoutingRulesResponse`
currently at lines 662–666:

```rust
#[derive(Serialize)]
pub struct RoutingRulesResponse {
    pub rules: Vec<RoutingRule>,
    pub default_route: String,
    pub bridge: BridgeConfig,                     // ← new
}

// get_routing_rules: add `bridge: state.bridge.clone()` to the response.

pub async fn save_routing_rules(
    rules: Vec<RoutingRule>,
    default_route: String,
    bridge: BridgeConfig,                         // ← new
    ctx: State<'_, AppContext>,
) -> Result<(), String> {
    // ...
    state.bridge = bridge;
```

`BridgeConfig` is in scope via the existing `use crate::proxy::{... models::* ...};`.

### 6. `src/api/tauri.ts`

`RuleAction` currently at line 69; `RoutingRulesResponse` at lines 78–81;
`saveRoutingRules` at lines 151–152.

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

saveRoutingRules: (rules: RoutingRule[], defaultRoute: string, bridge: BridgeConfig) =>
  invoke<void>("save_routing_rules", { rules, defaultRoute, bridge }),
```

### 7. `src/context/AppContext.tsx`

- Import `BridgeConfig` from `../api/tauri`.
- Add `bridge: BridgeConfig` to `AppState`.
- Initial state: `bridge: { interface: null, routing_mark: null, endpoints: [] }`.
- `SET_ROUTING_RULES` action: add `bridge: BridgeConfig`.
- Reducer case: spread `bridge: action.bridge`.
- Initial load `dispatch`: pass `bridge: routing.bridge`.

### 8. `src/components/routing/RoutingPage.tsx`

- Import `BridgeConfig`.
- `save` callback (currently lines 65–79): take a third `bridge: BridgeConfig`
  arg; pass to both the dispatch and `api.saveRoutingRules`.
- Update every `save(...)` caller (`setDefaultRoute`, `addRule`, `removeRule`,
  `toggleRule`, `addPreset`) to pass `state.bridge`.
- Add a `setBridge(patch: Partial<BridgeConfig>)` helper:
  `save(state.routingRules, state.defaultRoute, { ...state.bridge, ...patch })`.
- Add `<option value="bridge">{t("routing.bridge")}</option>` to the action
  `<select>`, and a `case "bridge"` color to `actionColor()`.
- Add a **"Custom interface routing"** settings block with inputs for:
  interface name (text → `interface`), fwmark (number → `routing_mark`), and
  endpoints (single text field, comma/whitespace-separated, parsed to a trimmed
  `string[]` with empties dropped → `endpoints`), all wired to `setBridge`.

### 9. `src/i18n/translations.ts`

Flat dot-notation keys, English only currently. Add under the `routing.*`
namespace: `routing.bridge` ("Interface") plus labels/helper text for the
settings block (interface name, endpoints, fwmark).

---

## The one correctness gotcha — anti-loop in TUN mode

In **TUN mode** sing-box swallows *all* egress, including the handshake/data
packets the external tunnel sends to its **server endpoint(s)**. If those match
`proxy` or `bridge`, you get a routing loop and the tunnel never establishes.
Two complementary defenses:

1. **OS side (preferred, matches `table = off`):** the interface uses its own
   fwmark + `ip rule`, and you exclude the endpoint exactly like `wg-quick`
   (`SO_MARK` + suppress-prefixlength). The `routing_mark` field lets sing-box's
   bridge traffic carry that mark.
2. **Config side (belt-and-suspenders, included above):** the `endpoints` list
   auto-inserts a high-priority `direct` rule for the endpoint IPs before user
   rules.

In **system-proxy mode** the loop risk is much lower (only proxied apps are
affected), so the bridge works with just the interface bind.

---

## Testing & validation

**Rust unit tests on `generate_config`:**
- No `bridge` outbound when `interface` is unset.
- `bridge` outbound present with correct `bind_interface` (and `routing_mark`
  when set) when `interface` is set.
- Anti-loop `direct` rule (covering all `endpoints`) precedes user rules when
  `interface` set and `endpoints` non-empty; absent when `endpoints` empty.
- `RuleAction::Bridge` falls back to `proxy` when `interface` unset (singbox)
  and always degrades to `proxy` in xray.

**Frontend:**
- Endpoints parsing: comma/whitespace-separated input → trimmed array, empty
  entries dropped.
- Settings block renders and round-trips through save/load.

**Platform:** Linux solid; Windows/macOS best-effort (bind works; managing a
`table = off` interface is the user's responsibility).

---

## Follow-ups / open questions

- Future phase: IRBox optionally launching/managing the external tunnel
  (process-lifecycle module). Out of scope here.
- Windows/macOS UX for an externally-managed `table = off` interface.
- Validation of endpoint CIDR/IP strings in the UI (basic format check).
