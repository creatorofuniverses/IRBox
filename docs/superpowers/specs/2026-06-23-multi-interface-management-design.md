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

## Additional scope (from real-world testing of v1.1.0)

These came out of testing the shipped feature against a live interface. They
are part of this redesign.

### A. Documentation (required deliverable)

The shipped v1.1.0 documented routing *into* an interface in `README.md`
(`## 🔀 Custom Interface Routing`) and `README_FA.md`, but **not** in the
documentation hub `docs/README.md`, which has no mention of custom interfaces at
all. This redesign must:

- Rewrite the README / README_FA "Custom Interface Routing" how-to for the new
  multi-interface flow (add an interface on the **Interfaces** page → mark one
  **active** → set rules to the **Interface** action → enable). Persian
  mirrored.
- Add a "Custom Interface Routing" section to `docs/README.md` (the hub),
  covering the same workflow plus the OS-side `table = off` / fwmark setup and
  the anti-loop endpoints rationale.

### B. Enable with an active interface even when no proxy server is connected

**Problem:** today the core (sing-box) only starts when the user connects to a
**proxy server** via the main Connect button (`api.connect(selectedServerId)` in
`StatusPanel.tsx`). Routing rules — including the `Interface` action — only take
effect while the core runs, so a user who *only* wants to route selected domains
into a custom interface (no proxy) currently can't: routing "works only if proxy
enabled."

**Goal:** let the user "go live" with just an **active interface** and no proxy
server selected.

**Design sketch:**
- The core can start in an **interface-only mode**: sing-box runs with **no
  proxy outbound**, default route `direct`, the `bridge` outbound bound to the
  active interface, and `Interface`-action rules routing into it (everything
  else stays direct). Anti-loop endpoints still apply in TUN mode.
- The Connect button's enabled-state becomes: enabled if **a proxy server is
  selected OR an active interface exists**. The status panel reflects which mode
  is live (proxy / interface-only / both).
- `connect` command path must accept "no server, active interface" and build the
  appropriate config (today it assumes a server). This is the main backend
  change for this item.

**Open question:** confirm default route in interface-only mode is `direct`
(recommended — non-matching traffic untouched) vs `block`.

### C. Interface liveness check + status indicator

Mirror how proxy connection status is surfaced, for the active interface:

- A **liveness check** that the bound interface actually exists / is up
  (Linux: presence via `ip link show <iface>` or netlink; Windows/macOS:
  best-effort via the OS interface list).
- Surface it in the UI: a status indicator on the interface card (and/or status
  panel) — up / down / unknown — refreshed periodically while active, like the
  proxy indicator.
- If the active interface goes down while connected, show it clearly (and
  consider a toast), since `Interface` rules would then fall through / fail.

---

## Related improvement: editable routing rules (separable)

> **Independent of the interface redesign** — included here at the user's
> request, but it can ship as its own PR/plan. Touches only the Routing page +
> rule persistence, not interfaces.

**Problem:** routing rules can't be edited in place. To change a rule's domain
or action you must delete it and add a new one — not user-friendly.

**Goal:** edit an existing rule's domain and action without delete + re-add.

**Design sketch:**
- Add an **Edit** affordance per rule row in `RoutingPage.tsx` that opens the
  add-rule form pre-filled (or makes the row's fields editable inline).
- Dispatch an `UPDATE_RULE` action (by rule `id`) in `AppContext`; persist via
  the existing `save_routing_rules` (rules are already stored as a list, so the
  backend needs no new command — just save the mutated list).
- Keep the existing add / remove / toggle behavior.

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
- **Interface-only mode (B):** `generate_config` test for "no proxy server +
  active interface" → no proxy outbound, default route `direct`, bridge outbound
  present, `Interface` rules → `bridge`. Manual: enable with only an interface
  active (no server selected) and confirm matching domains egress via it.
- **Liveness (C):** unit-test the interface-presence check against a known-up
  and a nonexistent interface name; manual: bring the interface down while
  connected and confirm the indicator reflects it.
- **Editable rules:** manual — edit a rule's domain and action in place and
  confirm it persists and re-applies without a delete + re-add.

## Open questions / future

- **Per-rule interface selection:** if multi-interface simultaneous routing is
  wanted later, the `Interface` action would need a per-rule interface id and
  the generator would emit one `direct` outbound per referenced interface. This
  spec deliberately defers it.
- **Validation:** basic IP/CIDR validation of endpoints and interface-name
  format in the modal.
- **Active indicator on connect:** whether the Home/status UI should surface
  which interface is active when any `Interface` rules exist.
- **Interface-only default route (B):** confirm `direct` (recommended) vs
  `block` for non-matching traffic when no proxy server is connected.
- **Liveness depth (C):** mere existence/up check vs. an active reachability
  probe to the interface's endpoint(s); and the poll interval.
- **Editable rules scope (separable):** ship independently of the interface
  redesign? It only touches the Routing page + rule persistence and could land
  in its own small PR/plan first.
