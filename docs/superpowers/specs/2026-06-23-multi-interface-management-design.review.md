# Review: Multi-interface management ("Interfaces" page) spec

> **Reviews:** `2026-06-23-multi-interface-management-design.md`
> **Reviewer:** Claude (design review, requested 2026-06-23)
> **Method:** read both specs + grounded every code-level claim against the live
> IRBox tree (`src-tauri/`, `src/`). All symbols/line ranges the spec leans on
> were confirmed present with no drift (BridgeConfig, AppState, RuleAction,
> `generate_config` signature, `save_routing_rules` reconnect path, RoutingPage
> bridge fields, AppContext bridge wiring, `connect` server requirement). The
> code-level premises are sound; findings below are about *design completeness*,
> not factual errors.

## Verdict

**Approve the direction; resolve the four MAJOR items before writing the
implementation plan.** The core redesign (list + active selection + dedicated
page) is well-scoped and the "what changes" table is honest. The risk is
concentrated in two places the spec under-specifies: **(1) migration with no
versioning system**, and **(2) item B (interface-only mode), which is a much
larger backend change than its "Additional scope" placement suggests** — it
changes the type-level contract of the whole core-start path, not just the
`connect` command.

I'd also flag a framing point: this is titled "multi-interface" but the on-wire
capability is **unchanged** from v1.1.0 — still exactly one `bridge` outbound,
now sourced from the *active* entry instead of the global field. The genuinely
new capabilities are items **B** (interface-only mode) and **C** (liveness).
That's fine, but the plan should bill the effort accordingly: the page/modal/CRUD
is mostly mechanical; B and C are where the real work and risk live.

---

## MAJOR — resolve before planning

### M1. Migration is underspecified and there's no version field to lean on
Verified: `AppState` has **no version/schema field**; load is
`serde_json::from_str(...).unwrap_or_default()`. So migration is purely
serde-default-driven, which has sharp edges the spec glosses:

- **Old `bridge.interface` is `Option<String>`; new `interface` is required
  non-empty `String`.** Migration must create an entry **only when
  `interface` is `Some` and non-empty** — a v1.1.0 user who set a `routing_mark`
  or `endpoints` but never an interface should migrate to *zero* interfaces, not
  one broken entry. State this.
- **uuid source for the migrated entry.** The spec says `id: stable uuid` but
  never says who mints it. Pick one: backend mints on migration/save (recommended
  — single source of truth), frontend uses `crypto.randomUUID()` for new adds.
  Decide and write it down, because `save_interface(config)` takes a *full*
  config including `id`, which implies the frontend mints ids for new entries —
  that contradicts "backend mints on migration." Reconcile.
- **Downgrade is lossy and silent.** The spec says drop `bridge` "after migration
  writes the new shape." With no version field, a user who rolls back to v1.1.0
  then loses their interface config (old build can't read `interfaces`/
  `active_interface_id`, and `bridge` is gone). At minimum: keep `bridge`
  populated from the active interface for one release (dual-write), OR
  acknowledge downgrade-loss explicitly. Don't silently drop the only field the
  old build understands.
- Recommend the plan add a **migration unit test** (v1.1.0 JSON → one active
  interface) *and* a "bridge with no interface → zero interfaces" test.

### M2. Item B (interface-only mode) is a cross-cutting type change, not a command tweak
The spec says "the `connect` command path must accept 'no server'... This is the
main backend change." It's bigger than that. Verified current contract:
`generate_config(server: &Server, ...)`, `CoreManager::start(server: &Server,
...)`, and `xray::generate_config(server: &Server, ...)` **all take a
non-optional `Server`**, threaded from `connect` and the three reconnect sites.
Interface-only mode forces `Server` → `Option<&Server>` (or a mode enum) through
**every** core-start call site, including xray — which has no bridge support at
all. Consequences the spec must address:

- **xray + interface-only is a dead end.** xray degrades `Bridge → "proxy"` and
  has no `bridge` outbound; with no proxy server there is nothing to route to.
  The plan must either force core=sing-box when entering interface-only mode, or
  define and surface the failure. Today nothing prevents this combination.
- **Custom-protocol path** (`manager.rs`) also degrades `Bridge → "proxy"` and
  owns its own outbounds — same dead end. Same decision needed.
- The "no proxy outbound, default route `direct`" config is a genuinely new
  sing-box shape — call it out as a distinct `generate_config` branch with its
  own test, not a variation of the existing one.

This item may deserve to **ship as its own PR after the page/CRUD lands**,
exactly like the editable-rules item is separated. Recommend the plan sequence it
last and gate it behind sing-box.

### M3. Reconnect-on-every-mutation will churn the tunnel unnecessarily
The spec says every mutation "triggers the same reconnect path the routing
commands use." But only the **active** interface affects the emitted config.
Editing a *non-active* interface, or adding/deleting a non-active one, would
reconnect (drop + rebuild the running core) for **no on-wire change**. Verified
the reconnect path (`save_routing_rules`, commands.rs:696–710) fully restarts the
core. Spec the reconnect trigger precisely:

- Reconnect **only** when the change affects the resolved active interface:
  `set_active_interface`, or a `save_interface`/`delete_interface` whose `id ==
  active_interface_id`. Non-active mutations persist state but **do not**
  reconnect.
- Add a test asserting a non-active edit does not restart the core.

### M4. Deleting/clearing the active interface in interface-only mode → invalid config
Once B exists, this edge is reachable and unhandled: if the core is live in
interface-only mode (no server) and the user deletes the active interface (or
toggles it off), `active_interface_id` clears → the reconnect now has **no server
and no active interface** = nothing to build. The spec's `delete_interface`
"clears active + triggers reconnect" would try to reconnect into an empty config.
Define it: in that case **stop the core** rather than reconnect. (Pre-B this can't
happen because the core only runs with a server, so it's specifically a B
interaction — another reason to sequence M2 carefully.)

---

## MINOR — clarify in the plan

- **"Use" toggle semantics.** Can the user deactivate the active interface by
  clicking its "Use" again (→ `set_active_interface(None)`)? The command supports
  `Option`, but the UI behavior (toggle-off vs no-op) is unstated. Define it.
- **Where `label` defaults to interface name.** Stated as a rule but not located.
  Do it backend-side on save so every consumer sees the resolved label.
- **Routing-page "no active interface" hint should be required, not "optional."**
  With B making "active interface, no proxy" a first-class mode, a rule using
  `Interface` with nothing active is a more likely foot-gun than in v1.1.0.
  Promote the inline hint to required.
- **Validation should not be an open question.** A bad interface name binds to a
  nonexistent device and fails silently at the OS layer; this is exactly what
  liveness (C) is meant to catch, but cheap modal-side format validation of the
  interface name + endpoint CIDR/IP is worth making a deliverable, not a "maybe."
- **i18n / Farsi.** The predecessor spec notes `translations.ts` is English-only,
  yet README_FA exists and this spec says "Persian mirrored" for docs. Clarify
  whether the new `interfaces.*` **UI** keys need Farsi now or stay English like
  the rest of the UI. (Docs are clearly bilingual; the UI apparently isn't yet.)
- **Liveness (C): "exists" vs "up" are different.** `ip link show <iface>`
  succeeds for a *down* interface. Read
  `/sys/class/net/<iface>/operstate` (or netlink) to distinguish up/down/unknown,
  and prefer that over spawning `ip` on a poll timer. State the poll interval
  (the open question) — suggest 5–10s while active, paused when not.

---

## Recommendations on the spec's open questions

- **Interface-only default route → `direct`.** Agree with the recommendation.
  `block` would silently blackhole all non-matched traffic, which is surprising
  for a "route these few domains into my tunnel" mode. Direct = least surprise.
- **Per-rule interface selection → keep deferred.** The data model
  (`active_interface_id: Option<String>`) is forward-compatible with it; no need
  to pay for it now.
- **Editable routing rules → ship first, independently.** Verified `RoutingRule`
  already has `id: String` (models.rs:314), so `UPDATE_RULE`-by-id + existing
  `save_routing_rules` needs **no new backend command**. It's the lowest-risk,
  highest-daily-value item here and touches nothing in the interface redesign.
  Land it as its own small PR ahead of everything else.
- **Liveness depth → existence/operstate first; reachability probe is a later
  tier.** A handshake/endpoint probe is much more code and platform-specific;
  start with presence+operstate and leave the probe in "future."

---

## Suggested test additions (beyond the spec's list)

- Migration: `bridge` with `interface = None` but non-empty `endpoints` → **zero**
  interfaces (not one broken entry).
- Reconnect suppression: mutating a **non-active** interface persists but does not
  restart the core (M3).
- Interface-only + xray/custom core selected → rejected or forced to sing-box
  (M2), with a clear surfaced state.
- Delete/deactivate active interface while live in interface-only mode → core
  **stops**, not reconnect-into-empty (M4).
- Downgrade-shape test if dual-write is adopted (M1): after migration, the legacy
  `bridge` field still reflects the active interface.

## Nits

- The "What changes" table is the strongest part — keep it; consider adding a
  fourth column noting "on-wire sing-box config: unchanged" to preempt the
  "multi-interface = multiple outbounds?" misread.
- Spec says internal `bridge`/`Bridge` naming "may stay or rename." Recommend
  **keep it** — verified it threads through singbox/xray/manager/commands/TS;
  renaming is pure churn against the goal of minimal, upstream-clean diffs, and
  the user-facing label is already decoupled.
