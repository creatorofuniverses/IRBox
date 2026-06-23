# Interface Liveness Indicator Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Show an up / down / unknown liveness indicator on each interface card, reflecting whether the bound OS interface actually exists and is up, refreshed periodically while the Interfaces page is open.

**Architecture:** A pure backend function reads `/sys/class/net/<iface>/operstate` on Linux/Android (best-effort `unknown` elsewhere) and maps presence + operstate to `up`/`down`. A `get_interface_statuses` Tauri command returns `{id, status}` for every configured interface. The Interfaces page polls it every 7s while mounted and renders a colored dot per card.

**Tech Stack:** Rust + Tauri 2 (crate `irbox`). React 18 + TypeScript (Vite), `useEffect` + `setInterval` polling.

## Global Constraints

- **Liveness = presence + operstate, NOT operstate alone.** WireGuard/TUN devices report `operstate = "unknown"` while fully up (point-to-point, no carrier). So: interface directory present and operstate ≠ `"down"` ⇒ **up**; directory absent or operstate == `"down"` ⇒ **down**. Reading `operstate` alone (treating `"unknown"` as not-up) would wrongly show a working tunnel as not-up.
- **Linux/Android read `/sys/class/net/<iface>/operstate`** (no spawning `ip`). Non-Linux/Android returns **`"unknown"`** (best-effort; no new dependency — YAGNI).
- **Three states only:** `"up"` | `"down"` | `"unknown"`.
- **Poll interval ~7s while the Interfaces page is mounted and ≥1 interface exists**; polling pauses automatically when the page unmounts. (Per the spec: refreshed periodically while active, paused when none.)
- **Depth = presence/operstate only.** An active reachability probe (handshake to the endpoint) is explicitly future/out of scope.
- **Out of scope (deferred):** a status-panel indicator and a toast when the active interface goes down while connected — both need app-level (off-page) polling; the card indicator is the primary deliverable. The spec lists status-panel as "and/or" and the toast as "consider", so leaving them is spec-compliant.
- No JS test runner — frontend gate is `npm run build`. Backend gate is `cd src-tauri && cargo test`. New UI i18n keys are **English-only**. `README_FA.md` is the user's — do not touch it.

---

## File Structure

**Backend:**
- `src-tauri/src/core/iface_status.rs` — **create.** `pub fn interface_status(name: &str) -> &'static str` + unit tests.
- `src-tauri/src/core/mod.rs` — **modify.** Declare `pub mod iface_status;`.
- `src-tauri/src/commands.rs` — **modify.** `InterfaceStatus` struct + `get_interface_statuses` command.
- `src-tauri/src/lib.rs` — **modify.** Register the command.

**Frontend:**
- `src/api/tauri.ts` — **modify.** `InterfaceStatus` type + `getInterfaceStatuses` wrapper.
- `src/components/interfaces/InterfacesPage.tsx` — **modify.** Poll statuses; render a status dot per card.
- `src/i18n/translations.ts` — **modify.** `interfaces.status.up/down/unknown`.
- `src/index.css` — **modify.** `.iface-status` dot classes.

**Docs:**
- `README.md` / `docs/README.md` — **modify.** One line on the indicator. `README_FA.md` untouched.

---

## Task 1: Backend — interface liveness function + command

**Files:**
- Create: `src-tauri/src/core/iface_status.rs`
- Modify: `src-tauri/src/core/mod.rs` (add `pub mod iface_status;`)
- Modify: `src-tauri/src/commands.rs` (add `InterfaceStatus` + `get_interface_statuses` near `get_interfaces`)
- Modify: `src-tauri/src/lib.rs` (register the command after `commands::set_active_interface,` line 169)

**Interfaces:**
- Consumes: `AppState.interfaces` (each `InterfaceConfig` has `id` and `interface`).
- Produces: `core::iface_status::interface_status(name: &str) -> &'static str` (`"up"`/`"down"`/`"unknown"`); `get_interface_statuses() -> Vec<InterfaceStatus { id: String, status: String }>`. Consumed by Task 2.

- [ ] **Step 1: Write the failing test (create the module with tests first)**

Create `src-tauri/src/core/iface_status.rs` with the test module (the function body comes in Step 3 — write it as a stub that fails first):

```rust
/// Liveness of a bound network interface, for the UI status indicator.
/// - "up": interface present and not administratively down
/// - "down": interface absent, or operstate == "down"
/// - "unknown": platform without a presence check (non-Linux/Android)
pub fn interface_status(_name: &str) -> &'static str {
    "unknown"
}

#[cfg(test)]
mod tests {
    use super::*;

    // Loopback always exists on Linux/Android (operstate "unknown" → treated as up);
    // a clearly-bogus name is absent → down.
    #[cfg(any(target_os = "linux", target_os = "android"))]
    #[test]
    fn loopback_is_up_and_missing_is_down() {
        assert_eq!(interface_status("lo"), "up");
        assert_eq!(interface_status("nonexistent-iface-zzz"), "down");
    }
}
```

Declare the module in `src-tauri/src/core/mod.rs` (append after `pub mod xray;`):

```rust
pub mod iface_status;
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cd src-tauri && cargo test core::iface_status`
Expected: FAIL — `loopback_is_up_and_missing_is_down` fails because the stub returns `"unknown"` for `"lo"` (expected `"up"`).

- [ ] **Step 3: Implement `interface_status`**

Replace the stub body in `src-tauri/src/core/iface_status.rs`:

```rust
pub fn interface_status(name: &str) -> &'static str {
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        let dir = format!("/sys/class/net/{}", name);
        if !std::path::Path::new(&dir).exists() {
            return "down"; // not present (e.g. typo'd bind target, or tunnel not brought up)
        }
        // WireGuard/TUN devices report operstate "unknown" while fully up, so treat
        // anything that isn't an explicit "down" as up once the device exists.
        match std::fs::read_to_string(format!("{}/operstate", dir)) {
            Ok(s) if s.trim() == "down" => "down",
            _ => "up",
        }
    }
    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    {
        let _ = name;
        "unknown"
    }
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `cd src-tauri && cargo test core::iface_status`
Expected: PASS — `interface_status("lo") == "up"`, `interface_status("nonexistent-iface-zzz") == "down"`.

- [ ] **Step 5: Add the `get_interface_statuses` command**

In `src-tauri/src/commands.rs`, add the import near the other `use crate::core::...` lines (top of file):

```rust
use crate::core::iface_status;
```

Add the response struct and command next to `get_interfaces`:

```rust
#[derive(Serialize)]
pub struct InterfaceStatus {
    pub id: String,
    pub status: String,
}

#[tauri::command]
pub async fn get_interface_statuses(ctx: State<'_, AppContext>) -> Result<Vec<InterfaceStatus>, String> {
    let state = ctx.state.lock().await;
    Ok(state.interfaces.iter().map(|i| InterfaceStatus {
        id: i.id.clone(),
        status: iface_status::interface_status(&i.interface).to_string(),
    }).collect())
}
```

- [ ] **Step 6: Register the command**

In `src-tauri/src/lib.rs`, add to the `generate_handler!` list after `commands::set_active_interface,` (line 169):

```rust
            commands::set_active_interface,
            commands::get_interface_statuses,
```

- [ ] **Step 7: Build and run all backend tests**

Run: `cd src-tauri && cargo test`
Expected: PASS — the new `iface_status` test plus all existing tests; whole crate compiles.

- [ ] **Step 8: Commit**

```bash
git add src-tauri/src/core/iface_status.rs src-tauri/src/core/mod.rs src-tauri/src/commands.rs src-tauri/src/lib.rs
git commit -m "feat(interfaces): interface liveness status (operstate) + command

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_011btTtMPPUr9HEUYTvV6cUZ"
```

---

## Task 2: Frontend — poll statuses + card indicator

**Files:**
- Modify: `src/api/tauri.ts` (add type + wrapper near `getInterfaces`)
- Modify: `src/components/interfaces/InterfacesPage.tsx` (poll effect + status dot)
- Modify: `src/i18n/translations.ts` (three status keys)
- Modify: `src/index.css` (`.iface-status` classes)

**Interfaces:**
- Consumes (Task 1): `get_interface_statuses` → `InterfaceStatus[]`.
- Produces: a live indicator on each interface card. No exports consumed later.

- [ ] **Step 1: Add the API type + wrapper**

In `src/api/tauri.ts`, after the `InterfacesResponse` interface, add:

```ts
export interface InterfaceStatus {
  id: string;
  status: string; // "up" | "down" | "unknown"
}
```

In the `api` object, after the `getInterfaces` wrapper, add:

```ts
  getInterfaceStatuses: () => invoke<InterfaceStatus[]>("get_interface_statuses"),
```

- [ ] **Step 2: Add the i18n keys (English-only)**

In `src/i18n/translations.ts`, next to the other `interfaces.*` keys, add:

```ts
  "interfaces.status.up": { en: "Up" },
  "interfaces.status.down": { en: "Down" },
  "interfaces.status.unknown": { en: "Unknown" },
```

- [ ] **Step 3: Add the status-dot CSS**

In `src/index.css`, after the `.status-indicator.on { ... }` block (around line 156), add:

```css
.iface-status {
  display: inline-block;
  width: 8px;
  height: 8px;
  border-radius: 50%;
  margin-right: 6px;
  flex-shrink: 0;
  vertical-align: middle;
  background: var(--text-muted);
}
.iface-status-up {
  background: var(--success);
  box-shadow: 0 0 6px var(--success);
}
.iface-status-down {
  background: var(--danger);
}
.iface-status-unknown {
  background: var(--text-muted);
}
```

- [ ] **Step 4: Poll statuses in InterfacesPage**

In `src/components/interfaces/InterfacesPage.tsx`, change the React import (line 1) to include `useEffect`:

```tsx
import { useState, useEffect } from "react";
```

Add status state next to the existing `useState` hooks (after line 13):

```tsx
  const [statuses, setStatuses] = useState<Record<string, string>>({});
```

Add a polling effect after the `remove` handler (after line 47), before the `return`:

```tsx
  // Poll interface liveness every 7s while the page is mounted and interfaces exist.
  useEffect(() => {
    if (state.interfaces.length === 0) return;
    let cancelled = false;
    const poll = async () => {
      try {
        const list = await api.getInterfaceStatuses();
        if (!cancelled) {
          setStatuses(Object.fromEntries(list.map((s) => [s.id, s.status])));
        }
      } catch {
        // ignore polling errors
      }
    };
    poll();
    const timer = setInterval(poll, 7000);
    return () => {
      cancelled = true;
      clearInterval(timer);
    };
  }, [state.interfaces.length]);
```

- [ ] **Step 5: Render the status dot on each card**

In the same file, change the `sub-name` span (lines 65-68) to lead with the status dot:

```tsx
                  <span className="sub-name">
                    <span
                      className={`iface-status iface-status-${statuses[iface.id] ?? "unknown"}`}
                      title={t(`interfaces.status.${statuses[iface.id] ?? "unknown"}` as never)}
                    />
                    {iface.label}
                    {active && <span className="sub-featured-badge">{t("interfaces.active")}</span>}
                  </span>
```

(The `as never` cast satisfies the strict `TranslationKey` type for the dynamic key; the three concrete keys all exist from Step 2. If `tsc` rejects `as never`, replace the `title` with an explicit map: `title={statuses[iface.id] === "up" ? t("interfaces.status.up") : statuses[iface.id] === "down" ? t("interfaces.status.down") : t("interfaces.status.unknown")}`.)

- [ ] **Step 6: Type-check and build**

Run: `npm run build`
Expected: PASS — the new type/wrapper, keys, effect, and JSX all type-check.

- [ ] **Step 7: Manual verification (run the app)**

Run: `npm run tauri dev`

Verify:
1. **Up:** add an interface bound to a real, up device (e.g. `lo`, or your tunnel `awg0` when running) → the card shows a green dot; hovering the dot shows "Up".
2. **Down:** add an interface with a bogus name (e.g. `awg-nope`) → red dot, tooltip "Down". Bring a real tunnel down (`ip link set <iface> down`) → its dot turns red within ~7s.
3. **Polling pauses:** navigate away from the Interfaces page and back — no errors; the dot refreshes again.
4. **Non-Linux** (if applicable): a grey "Unknown" dot.

- [ ] **Step 8: Commit**

```bash
git add src/api/tauri.ts src/components/interfaces/InterfacesPage.tsx src/i18n/translations.ts src/index.css
git commit -m "feat(interfaces): live up/down status dot on interface cards

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_011btTtMPPUr9HEUYTvV6cUZ"
```

---

## Task 3: Docs — note the liveness indicator

**Files:**
- Modify: `README.md` (`## 🔀 Custom Interface Routing`), `docs/README.md` (`## Custom Interface Routing`)
- **Do not touch** `README_FA.md`.

**Interfaces:** Consumes the final behavior. No code consumers.

- [ ] **Step 1: Add a line to each Custom Interface Routing section**

Append to the existing `## 🔀 Custom Interface Routing` section in `README.md`:

```markdown
Each interface card shows a **live status dot** — green (up), red (down /
not present), or grey (unknown) — refreshed every few seconds while the
Interfaces page is open, so you can see at a glance whether the bound device
exists and is up.
```

Append to the existing `## Custom Interface Routing` section in `docs/README.md`:

```markdown
**Liveness indicator:** each interface card shows an up/down/unknown status
dot based on the OS interface state (Linux/Android read
`/sys/class/net/<iface>/operstate`; a present device that isn't explicitly
`down` counts as up, since WireGuard/TUN devices report `unknown` while up).
Other platforms show `unknown`. This is a presence/operstate check only — not
an active reachability probe.
```

- [ ] **Step 2: Sanity build + commit**

Run: `npm run build` (Expected: PASS — confirms nothing else regressed.)

```bash
git add README.md docs/README.md
git commit -m "docs: document interface liveness indicator

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_011btTtMPPUr9HEUYTvV6cUZ"
```

---

## Self-Review

**Spec coverage (item C):**
- "Liveness check: exists AND is up; Linux reads `/sys/class/net/<iface>/operstate`; Windows/macOS best-effort" → Task 1 `interface_status` (presence + operstate; `unknown` on non-Linux/Android).
- "Surface on the interface card — up/down/unknown — refreshed periodically while active, paused when none" → Task 2 (7s poll while page mounted + interfaces exist; auto-pauses on unmount).
- "Poll interval ~5–10s" → Task 2 (7s).
- "Depth: presence/operstate first; reachability probe future" → Global Constraints + Task 1 (no probe).
- Deferred (spec-optional): status-panel indicator + active-down toast — stated in Global Constraints (need app-level polling).

**Placeholder scan:** No "TBD"/"handle errors"/"similar to". Backend has a cfg-gated TDD test (runs on the Linux/Android dev target); the `as never` cast in Task 2 Step 5 has an explicit fallback if `tsc` rejects it.

**Type consistency:** `interface_status(name: &str) -> &'static str` (Task 1) returns one of `"up"/"down"/"unknown"`; `InterfaceStatus { id, status }` (Rust, Task 1) ↔ `InterfaceStatus { id: string; status: string }` (TS, Task 2) ↔ `get_interface_statuses` command ↔ `api.getInterfaceStatuses()`. The i18n keys `interfaces.status.{up,down,unknown}` (Task 2 Step 2) match the dynamic key used in Step 5. CSS classes `iface-status` + `iface-status-{up,down,unknown}` (Step 3) match the `className` template (Step 5).
