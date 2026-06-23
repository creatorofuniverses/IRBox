# Spec: Multi-interface management ("Interfaces" page)

> **Status:** approved design, NOT yet implemented. Future feature.
> **Builds on / revises:** `2026-06-23-bridge-interface-routing-design.md` (the
> shipped single-interface "bridge" feature, v1.1.0). That feature stays as-is
> and merges first; this spec redesigns its UX into a proper multi-interface
> manager in a later release.
> **Trigger:** the shipped UX exposes a single global interface as three loose
> fields on the Routing page — it neither supports several named interfaces nor
> reads as "manage interfaces". This spec fixes that.

## Goal

Manage **multiple named custom interfaces** the way the app manages servers and
subscriptions: a dedicated **"Interfaces"** Sidebar page listing saved
interfaces, an **Add/Edit interface** modal, and one interface marked **active
("Use")**. Routing rules with the **Interface** action route into whichever
interface is currently active. The Routing page goes back to being *just*
routing (rules + default route) — the interface fields move out of it.

This is a pure config + UX feature; like the current implementation, IRBox never
creates or tears down the interfaces.

## What changes vs. the shipped feature

| Aspect | Shipped (v1.1.0) | This redesign |
|--------|------------------|---------------|
| Data | one global `BridgeConfig { interface, routing_mark, endpoints }` | a **list** of named interfaces + an **active** selection |
| UI | 3 fields inline on the Routing page | dedicated **Interfaces** page (list + modal), active toggle |
| Routing action | `Interface` → the single global interface | `Interface` → the **active** interface (else proxy) |
| Routing page | rules + default route + interface fields | rules + default route only |

## Non-goals (explicit)

- **Per-rule interface selection.** Every `Interface`-action rule uses the
  single active interface. (Routing different domains to different interfaces
  simultaneously is a possible future extension; see Open questions.)
- **Routing to multiple interfaces at once.** Only the active interface gets a
  bridge outbound emitted.
- **IRBox launching/managing the tunnel lifecycle.** Still external.
- **A literal second OS window.** The app has no multi-window pattern; the
  manager is an in-app page + modal, consistent with Subscriptions.

---

## Design

### Data model

Replace the single `BridgeConfig` on `AppState` with a list plus an active id.

```rust
/// A named, externally-managed network interface IRBox can route into.
pub struct InterfaceConfig {
    pub id: String,                 // stable uuid
    pub label: String,              // user-facing name, e.g. "Work AWG"
    pub interface: String,          // bind target, e.g. "awg0" (required, non-empty)
    pub routing_mark: Option<u32>,  // optional SO_MARK / fwmark (Linux)
    pub endpoints: Vec<String>,     // server IP/CIDRs kept on direct (anti-loop)
}

// AppState (replacing `bridge: BridgeConfig`):
pub interfaces: Vec<InterfaceConfig>,        // #[serde(default)]
pub active_interface_id: Option<String>,     // #[serde(default)]
```

Notes:
- `interface` is required per entry (an interface with no bind target is
  meaningless); `label` defaults to the interface name if left blank.
- `active_interface_id` points at one entry, or `None` (no active interface).
  If it references a deleted entry, treat as `None`.

### Migration (backward compatibility)

Persisted state from v1.1.0 carries `bridge: BridgeConfig`. On load:
- If `bridge.interface` is set, create one `InterfaceConfig` from it
  (label = the interface name) and set it active.
- Keep deserializing the old `bridge` field (`#[serde(default)]`, may become a
  legacy/transitional field) so existing config files don't error, then drop it
  after migration writes the new shape. A one-shot migration on first load is
  sufficient; document it in the implementation plan.

### Backend (sing-box generation)

`generate_config` takes the resolved **active** interface (the controller
resolves `active_interface_id` → `Option<&InterfaceConfig>` before calling, or
passes the whole list + id; implementation detail for the plan):

- Emit the `bridge` `direct` outbound (tag `"bridge"`, `bind_interface`,
  optional `routing_mark`) **only when there is an active interface**.
- Emit the anti-loop `{ ip_cidr: <active.endpoints>, outbound: "direct" }` rule
  before user rules when the active interface has endpoints.
- `RuleAction::Bridge` (the `Interface` action) → outbound `"bridge"` if an
  active interface exists, else `"proxy"` (unchanged fallback semantics).
- xray and the Custom-protocol `manager.rs` path keep degrading `Bridge` →
  `"proxy"` (unchanged).

The on-wire sing-box config is unchanged in shape from the shipped feature — it
still emits at most one `bridge` outbound. Only the *source* of that interface
changes (active selection instead of the single global field).

### Commands (Tauri)

Interface CRUD + active selection, plus the existing routing get/save:

- `get_interfaces() -> { interfaces: Vec<InterfaceConfig>, active_interface_id: Option<String> }`
- `save_interface(config: InterfaceConfig)` — add or update by id.
- `delete_interface(id: String)` — also clears active if it pointed here.
- `set_active_interface(id: Option<String>)` — mark active (or none).
- Each mutation persists state and triggers the same reconnect path the routing
  commands use, so a running core picks up the change (mirrors how
  `save_routing_rules` reconnects today).
- `RoutingRulesResponse` / `save_routing_rules` **drop** the `bridge` field
  (interface config no longer lives with routing rules).

### Frontend

- **Sidebar:** add an `"interfaces"` item (between `subscriptions` and
  `routing`), with an icon + i18n label.
- **Interfaces page** (`src/components/interfaces/`):
  - `InterfaceList.tsx` — lists `InterfaceConfig` cards (label, interface,
    endpoint count, fwmark). Each card: **Use** toggle (set active — active one
    is visually marked), **Edit**, **Delete**. An **Add interface** button.
  - `InterfaceModal.tsx` — modeled on `AddSubModal`: fields for label,
    interface name, endpoints (comma/whitespace → `string[]`, reusing the
    `parseEndpoints` logic), fwmark (number | null). Used for both add and edit.
- **AppContext:** replace `bridge` with `interfaces: InterfaceConfig[]` and
  `activeInterfaceId: string | null`; actions for set/add/update/delete/active.
- **api/tauri.ts:** `InterfaceConfig` type + the new command wrappers; remove
  `bridge` from `RoutingRulesResponse` / `saveRoutingRules`.
- **RoutingPage.tsx:** remove the "Custom interface routing" settings block and
  the `setBridge`/bridge wiring. Keep the `Interface` option in the action
  `<select>` and its `actionColor` case. Optionally show a small inline hint
  when a rule uses `Interface` but no interface is active (links to the
  Interfaces page).
- **i18n:** new `interfaces.*` keys (page title, add/edit/delete, use/active,
  field labels, "no active interface" hint). The shipped `routing.bridge*`
  field-label keys that move out can be removed or repurposed.

### Naming

User-facing: **"Interfaces"** (page), **"Custom interface"** (an entry), the
rule action stays **"Interface"**. Internal code may keep `bridge`/`Bridge` for
the sing-box outbound tag and `RuleAction::Bridge` to avoid churning the
config-generation layer, OR rename to `interface` for clarity — the
implementation plan decides, but the on-wire outbound tag `"bridge"` can stay.

---

## Testing approach

- **Rust:** unit tests on `generate_config` — active interface emits the
  bridge outbound + anti-loop; no active interface emits neither and `Interface`
  rules fall back to proxy; deleting the active interface clears it. Migration
  test: a v1.1.0 `bridge` config deserializes into one active interface.
- **Commands:** add/update/delete/set-active round-trip; delete-active clears
  `active_interface_id`.
- **Frontend:** `npm run build` (no unit runner); manual checks — add several
  interfaces, switch active, confirm routing follows the active one, confirm the
  Routing page no longer shows interface fields.

## Open questions / future

- **Per-rule interface selection:** if multi-interface simultaneous routing is
  wanted later, the `Interface` action would need a per-rule interface id and
  the generator would emit one `direct` outbound per referenced interface. This
  spec deliberately defers it.
- **Validation:** basic IP/CIDR validation of endpoints and interface-name
  format in the modal.
- **Active indicator on connect:** whether the Home/status UI should surface
  which interface is active when any `Interface` rules exist.
