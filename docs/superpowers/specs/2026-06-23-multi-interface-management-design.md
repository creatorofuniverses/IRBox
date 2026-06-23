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
| **On-wire sing-box config** | at most one `bridge` outbound | **unchanged** — still at most one `bridge` outbound, now sourced from the *active* entry |

> **Framing:** despite the "multi-interface" title, the on-wire capability is
> **unchanged** from v1.1.0 — still exactly one `bridge` outbound, just sourced
> from the active entry instead of a single global field. The page/modal/CRUD is
> mostly mechanical. The genuinely new work and risk live in items **B**
> (interface-only mode) and **C** (liveness) below; the plan should bill effort
> accordingly. (This redesign does **not** enable routing to several interfaces
> at once — that stays a deferred future extension; see Open questions.)

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
  meaningless); `label` defaults to the interface name if left blank — resolved
  **backend-side in `save_interface`** so every consumer sees the filled label
  (not in the modal, where a blank submit could slip through).
- `active_interface_id` points at one entry, or `None` (no active interface).
  If it references a deleted entry, treat as `None`.
- **`id` ownership (single source of truth = backend).** `save_interface`
  treats an **empty `id`** as "new" and mints a fresh uuid backend-side; a
  **non-empty `id`** is an update by id. The frontend therefore never mints
  uuids — it submits `id: ""` for adds and the existing id for edits. Migration
  (below) likewise mints backend-side. This resolves the apparent contradiction
  between "backend mints" and `save_interface` taking a full config including
  `id`.
- **Downgrade safety (dual-write).** Because `AppState` has no version field
  (see Migration), a user who rolls back to v1.1.0 after this ships would lose
  their interface config — the old build can't read `interfaces` /
  `active_interface_id`. For **one release**, keep writing the legacy `bridge`
  field populated from the *active* interface (dual-write) so a rollback still
  finds a usable single-interface config. Drop `bridge` only in the release
  after.

### Migration (backward compatibility)

**There is no version/schema field on `AppState`** (load is
`serde_json::from_str(...).unwrap_or_default()`, commands.rs:826). Migration is
therefore purely serde-default-driven, which has sharp edges this section pins
down explicitly:

Persisted state from v1.1.0 carries `bridge: BridgeConfig` where
`interface: Option<String>`. On load:

- **Create an interface only when `bridge.interface` is `Some` and non-empty.**
  A v1.1.0 user who set a `routing_mark` or `endpoints` but *never* an interface
  migrates to **zero interfaces** (and `active_interface_id = None`), **not** one
  broken entry with an empty bind target. The new `interface` field is a
  required non-empty `String`; an empty one would be invalid.
- When an interface is present: mint a fresh uuid **backend-side**, set
  `label = the interface name`, carry over `routing_mark` + `endpoints`, and set
  that entry active.
- Keep deserializing the old `bridge` field (`#[serde(default)]`) so existing
  config files don't error. Per the dual-write decision (Data model), **keep
  `bridge` populated from the active interface for one release** rather than
  dropping it immediately — this is the rollback-safety net. Drop it the release
  after.
- A one-shot migration on first load is sufficient; document the exact step in
  the implementation plan.

Migration tests (see Testing): a v1.1.0 `bridge` with a real interface → one
active interface; a v1.1.0 `bridge` with `interface = None` but non-empty
`endpoints` → **zero** interfaces.

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
- `save_interface(config: InterfaceConfig)` — empty `id` ⇒ add (backend mints
  uuid); non-empty `id` ⇒ update by id. Fills `label` from `interface` if blank.
- `delete_interface(id: String)` — also clears active if it pointed here.
- `set_active_interface(id: Option<String>)` — mark active (or none).
- `RoutingRulesResponse` / `save_routing_rules` **drop** the `bridge` field
  (interface config no longer lives with routing rules).

**Reconnect trigger (M3 — do not churn the tunnel).** Only the **active**
interface affects the emitted config, and the reconnect path fully restarts the
core (`save_routing_rules`, commands.rs:695–710). So a mutation reconnects a
running core **only when it touches the resolved active interface**:

- `set_active_interface(...)` — always (the active selection changed).
- `save_interface` / `delete_interface` where `id == active_interface_id` —
  yes.
- `save_interface` / `delete_interface` of a **non-active** entry, or an add of
  a new (non-active) entry — **persist state, do NOT reconnect** (no on-wire
  change).

**Delete/deactivate the active interface (M4 — don't reconnect into an empty
config).** Once interface-only mode (item B) exists, deleting the active
interface — or toggling it off — while the core is live with **no proxy server**
leaves nothing to build (no server, no active interface). In that case the
command must **stop the core**, not reconnect. (Pre-B this can't occur: the core
only runs with a server, so a cleared active interface just reconnects into a
proxy-only config. The stop-core branch is specifically a B interaction — another
reason to sequence B carefully.)

### Frontend

- **Sidebar:** add an `"interfaces"` item (between `subscriptions` and
  `routing`), with an icon + i18n label.
- **Interfaces page** (`src/components/interfaces/`):
  - `InterfaceList.tsx` — lists `InterfaceConfig` cards (label, interface,
    endpoint count, fwmark). Each card: **Use** toggle (set active — active one
    is visually marked), **Edit**, **Delete**. An **Add interface** button.
    **Toggle-off semantics:** clicking **Use** on the already-active card calls
    `set_active_interface(None)` (deactivate) — the active selection is a
    toggle, not a one-way latch. Reflect the off state visually.
  - `InterfaceModal.tsx` — modeled on `AddSubModal`: fields for label,
    interface name, endpoints (comma/whitespace → `string[]`, reusing the
    `parseEndpoints` logic), fwmark (number | null). Used for both add and edit.
- **AppContext:** replace `bridge` with `interfaces: InterfaceConfig[]` and
  `activeInterfaceId: string | null`; actions for set/add/update/delete/active.
- **api/tauri.ts:** `InterfaceConfig` type + the new command wrappers; remove
  `bridge` from `RoutingRulesResponse` / `saveRoutingRules`.
- **RoutingPage.tsx:** remove the "Custom interface routing" settings block and
  the `setBridge`/bridge wiring. Keep the `Interface` option in the action
  `<select>` and its `actionColor` case. **Required (not optional):** show an
  inline hint when a rule uses `Interface` but no interface is active, linking to
  the Interfaces page. With item B making "active interface, no proxy" a
  first-class mode, an `Interface` rule with nothing active is a more likely
  foot-gun than in v1.1.0, so this hint is a deliverable.
- **i18n:** new `interfaces.*` keys (page title, add/edit/delete, use/active,
  field labels, "no active interface" hint). The shipped `routing.bridge*`
  field-label keys that move out can be removed or repurposed. **Farsi scope:**
  `translations.ts` is English-only today (the predecessor spec notes this), so
  the new `interfaces.*` **UI** keys stay **English**, consistent with the rest
  of the UI. The *docs* (README_FA, below) are bilingual and **are** mirrored to
  Persian — UI keys and docs are separate concerns; don't conflate them.

### Naming

User-facing: **"Interfaces"** (page), **"Custom interface"** (an entry), the
rule action stays **"Interface"**. **Keep** the internal `bridge`/`Bridge`
naming — the sing-box outbound tag `"bridge"`, `RuleAction::Bridge`, the
`bridge` config field. It threads through singbox / xray / manager / commands /
TS; renaming is pure churn against the goal of a minimal, upstream-clean diff,
and the user-facing label is already decoupled from the internal name.

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

> **Scope warning — this is a cross-cutting type change, not a `connect`
> tweak, and should ship as its own PR after the page/CRUD lands** (mirroring how
> editable rules is separated). Verified current contract:
> `singbox::generate_config(server: &Server, …)` (singbox.rs:7),
> `CoreManager::start(server: &Server, …)` (manager.rs:116), and
> `xray::generate_config(server: &Server, …)` (xray.rs:7) **all take a
> non-optional `&Server`**, threaded from `connect` and every reconnect site.
> Interface-only mode forces `&Server` → `Option<&Server>` (or a mode enum)
> through **every** core-start call site. Bill it accordingly and sequence it
> last.

**Design sketch:**
- The core can start in an **interface-only mode**: sing-box runs with **no
  proxy outbound**, default route `direct`, the `bridge` outbound bound to the
  active interface, and `Interface`-action rules routing into it (everything
  else stays direct). Anti-loop endpoints still apply in TUN mode. This "no
  proxy outbound" config is a **genuinely new sing-box shape** — implement it as
  a distinct `generate_config` branch with its **own test**, not a tweak of the
  existing one.
- **Gate interface-only mode to sing-box.** xray degrades `Bridge → "proxy"`
  (xray.rs:332) and emits no `bridge` outbound; the Custom-protocol path
  (`manager.rs:304`) likewise degrades to `"proxy"` and owns its own outbounds.
  With **no proxy server**, both have nothing to route to — interface-only +
  xray/custom is a **dead end**. The plan must **force core = sing-box when
  entering interface-only mode** (or define + surface the failure). Today
  nothing prevents the bad combination.
- The Connect button's enabled-state becomes: enabled if **a proxy server is
  selected OR an active interface exists**. The status panel reflects which mode
  is live (proxy / interface-only / both).
- `connect` command path must accept "no server, active interface" and build the
  appropriate config (today it assumes a server). This is the entry point, but
  the real surface is the type change above plus the new generator branch.
- **Interaction with M4:** once this lands, deleting/deactivating the active
  interface while live in interface-only mode must **stop the core** (see
  Commands → M4), not reconnect into an empty config.

**Resolved:** default route in interface-only mode is **`direct`** (non-matching
traffic untouched — least surprise for a "route a few domains into my tunnel"
mode; `block` would silently blackhole everything unmatched).

### C. Interface liveness check + status indicator

Mirror how proxy connection status is surfaced, for the active interface:

- A **liveness check** that the bound interface actually exists **and is up**.
  These are different states: `ip link show <iface>` (and mere presence)
  succeeds for a **down** interface. On Linux, read
  `/sys/class/net/<iface>/operstate` (or netlink) to distinguish
  **up / down / unknown** — prefer that over spawning `ip` on a poll timer.
  Windows/macOS: best-effort via the OS interface list.
- Surface it in the UI: a status indicator on the interface card (and/or status
  panel) — up / down / unknown — refreshed periodically while active, like the
  proxy indicator. **Poll interval: ~5–10s while an interface is active, paused
  when none is active.**
- If the active interface goes down while connected, show it clearly (and
  consider a toast), since `Interface` rules would then fall through / fail.
- **Depth:** start with presence + operstate. An active reachability probe
  (handshake to the interface's endpoint) is much more code and
  platform-specific — leave it in "future".

### D. Input validation (required deliverable, not an open question)

A bad interface name binds to a nonexistent device and fails **silently** at the
OS layer — exactly the failure liveness (C) is meant to *detect after the fact*.
Cheap modal-side format validation prevents it up front, so it's a deliverable,
not a "maybe":

- Validate the **interface name** format in `InterfaceModal` (non-empty, no
  whitespace, OS-plausible) before save.
- Validate each **endpoint** as IP / CIDR before save.
- Block submit with a clear inline error on failure.

---

## Related improvement: editable routing rules (separable)

> **Independent of the interface redesign — recommend shipping this FIRST, as
> its own small PR, ahead of everything else.** It touches only the Routing page
> + rule persistence, not interfaces, and needs **no new backend command**:
> `RoutingRule` already has a stable `id: String` (models.rs:314), so an
> `UPDATE_RULE`-by-id reusing the existing `save_routing_rules` is enough. It's
> the lowest-risk, highest-daily-value item here.

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

## Suggested sequencing (PR order)

The review's recurring theme: separate the cheap mechanical work from the
risky type-level work. Suggested order, smallest/lowest-risk first:

1. **Editable routing rules** — independent, no new backend command. Lands first.
2. **Interfaces page + CRUD + active selection + migration (M1, M3, M4-pre-B)** —
   the core redesign. On-wire behavior unchanged from v1.1.0.
3. **Item B (interface-only mode, M2)** — the cross-cutting `&Server` →
   `Option<&Server>` change, gated to sing-box, with the new generator branch and
   the M4 stop-core interaction. Ships **last**, as its own PR.
4. **Item C (liveness)** — presence + operstate indicator; can land alongside or
   after B.

Docs (item A) update with whichever PR changes the user-facing flow.

## Testing approach

- **Rust:** unit tests on `generate_config` — active interface emits the
  bridge outbound + anti-loop; no active interface emits neither and `Interface`
  rules fall back to proxy; deleting the active interface clears it.
- **Migration (M1):** a v1.1.0 `bridge` with a real interface → one active
  interface; **a v1.1.0 `bridge` with `interface = None` but non-empty
  `endpoints` → ZERO interfaces** (not one broken entry); and — if dual-write is
  adopted — after migration the legacy `bridge` field still reflects the active
  interface (downgrade-shape test).
- **Commands:** add/update/delete/set-active round-trip; delete-active clears
  `active_interface_id`.
- **Reconnect suppression (M3):** mutating a **non-active** interface persists
  state but does **not** restart the core.
- **Dead-end guard (M2):** interface-only mode with xray/custom core selected →
  rejected or forced to sing-box, with a clearly surfaced state.
- **Stop-not-reconnect (M4):** delete/deactivate the active interface while live
  in interface-only mode → core **stops**, not reconnect-into-empty.
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

## Resolved during review

These were open questions; the review settled them and they're now folded into
the body above:

- **Interface-only default route (B):** **`direct`** (see item B). `block` would
  silently blackhole all unmatched traffic.
- **Validation:** now a **required deliverable** (item D), not an open question.
- **Liveness depth (C):** **presence + operstate first**; reachability probe is
  a later tier. Poll **~5–10s** while active. (See item C.)
- **Editable rules scope:** **ship first, independently** — no new backend
  command needed (`RoutingRule.id`, models.rs:314).
- **Per-rule interface selection:** **keep deferred** — the data model
  (`active_interface_id: Option<String>`) stays forward-compatible with it.
- **Internal `bridge`/`Bridge` naming:** **keep it** (see Naming).

## Open questions / future

- **Per-rule interface selection (future):** if multi-interface simultaneous
  routing is wanted later, the `Interface` action would need a per-rule
  interface id and the generator would emit one `direct` outbound per referenced
  interface. Deliberately deferred.
- **Active indicator on connect:** whether the Home/status UI should surface
  which interface is active when any `Interface` rules exist.
- **Liveness reachability probe (future):** an active handshake/endpoint probe
  beyond presence + operstate — more code and platform-specific.
