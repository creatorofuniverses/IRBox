# Editable Routing Rules Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let the user edit an existing routing rule's domain and action in place, without deleting and re-adding it.

**Architecture:** Frontend-only change confined to `RoutingPage.tsx` (plus two i18n keys and a little CSS). Each rule row gets an **Edit** affordance that switches that one row into an inline edit state (domain `<input>` + action `<select>` + Save/Cancel). Saving recomputes the rules list and persists through the **existing** `save()` helper — the same debounced `SET_ROUTING_RULES` dispatch + `api.saveRoutingRules(...)` path that add/remove/toggle already use. **No backend command and no new reducer action are needed:** `save_routing_rules` already accepts and persists the full rules list, and `RoutingRule` already has a stable `id` (`src-tauri/src/proxy/models.rs:314`).

**Tech Stack:** React + TypeScript (Vite), Tauri 2. State via `useReducer` in `src/context/AppContext.tsx`. i18n via a flat key map in `src/i18n/translations.ts`.

## Global Constraints

- **No new JS test runner.** The project has no unit-test framework (`package.json` `build` is `tsc && vite build`; there are zero `*.test`/`*.spec` files). The automated gate for this feature is `npm run build` (TypeScript type-check + production build); behavior is verified manually. Do **not** add vitest/jest for this feature (YAGNI, not an established pattern).
- **Follow the existing RoutingPage persistence pattern.** All mutations (`addRule`, `removeRule`, `toggleRule`) recompute the full rules array and call the local `save(rules, defaultRoute, bridge)` helper (`RoutingPage.tsx:65-79`). The edit must do the same — **do not** introduce a dedicated `UPDATE_RULE` reducer action (see Notes).
- **Preserve existing add / remove / toggle behavior unchanged.**
- **i18n:** every user-facing string goes through `t("...")`. New UI keys may be English-only (the rest of the UI's `routing.*`/`common.*` keys are mixed en/ru; match what exists — supply `ru` where trivial, English-only is acceptable).
- This plan is **independent** of the multi-interface redesign and the `state.bridge` field — leave all `bridge` wiring exactly as-is (pass `state.bridge` straight through to `save()`).

---

## File Structure

- `src/i18n/translations.ts` — **modify.** Add `common.save` (and reuse existing `common.cancel`). Edit-button uses a localized `title`; add `routing.editRule`.
- `src/components/routing/RoutingPage.tsx` — **modify.** Add edit state + `startEdit`/`cancelEdit`/`saveEdit` handlers; render an inline edit row when a row is being edited and an Edit button otherwise.
- `src/index.css` — **modify.** Add `.routing-rule-edit` (the per-row edit button, mirroring `.routing-rule-delete`) and `.routing-rule-card.editing` / `.routing-rule-edit-input` layout styles.

No files are created; no backend (`src-tauri/`) files change.

---

## Task 1: Add i18n keys for editing

**Files:**
- Modify: `src/i18n/translations.ts:161-164` (the `// Common` block) and `:137` area (the `routing.*` block)

**Interfaces:**
- Consumes: nothing.
- Produces: translation keys `common.save`, `routing.editRule`, consumed by Task 2. (`common.cancel` already exists at `translations.ts:162` and is reused as-is.)

- [ ] **Step 1: Add `common.save` next to the existing `common.cancel`**

In `src/i18n/translations.ts`, find the Common block:

```ts
  // Common
  "common.add": { en: "Add", ru: "Добавить" },
  "common.cancel": { en: "Cancel", ru: "Отмена" },
  "common.import": { en: "Import", ru: "Импорт" },
```

Insert a `common.save` line after `common.cancel`:

```ts
  // Common
  "common.add": { en: "Add", ru: "Добавить" },
  "common.cancel": { en: "Cancel", ru: "Отмена" },
  "common.save": { en: "Save", ru: "Сохранить" },
  "common.import": { en: "Import", ru: "Импорт" },
```

- [ ] **Step 2: Add `routing.editRule` (edit-button title) in the routing block**

Find the existing routing rule keys (around `translations.ts:133`):

```ts
  "routing.noRules": { en: "No routing rules yet. Add one above.", ru: "Правил пока нет. Добавьте первое выше." },
```

Add a `routing.editRule` key immediately after it:

```ts
  "routing.noRules": { en: "No routing rules yet. Add one above.", ru: "Правил пока нет. Добавьте первое выше." },
  "routing.editRule": { en: "Edit rule", ru: "Изменить правило" },
```

- [ ] **Step 3: Type-check that the keys are well-formed**

Run: `npm run build`
Expected: PASS (TypeScript compiles, no errors). This confirms the new entries match the translation map's value type.

- [ ] **Step 4: Commit**

```bash
git add src/i18n/translations.ts
git commit -m "i18n: add common.save and routing.editRule keys for editable rules"
```

---

## Task 2: Inline edit a routing rule in RoutingPage

**Files:**
- Modify: `src/components/routing/RoutingPage.tsx` (add state near `:44-46`; add handlers near `:110-118`; replace the rules-list render at `:289-323`)
- Modify: `src/index.css` (add styles after `.routing-rule-delete:hover`, `:1459`)
- Test: none (no JS unit runner — verify via `npm run build` + manual checks below)

**Interfaces:**
- Consumes: from Task 1 — `t("common.save")`, `t("common.cancel")`, `t("routing.editRule")`. From existing code — the local `save(rules, defaultRoute, bridge)` helper (`RoutingPage.tsx:65`), `state.routingRules`, `state.defaultRoute`, `state.bridge`, `toast`, the `RoutingRule`/`RuleAction` types from `../../api/tauri`, and `actionColor` (`:137`).
- Produces: nothing consumed by later tasks (this is the last task of this plan).

- [ ] **Step 1: Add edit state to the component**

In `RoutingPage.tsx`, find the existing state declarations (`:44-46`):

```tsx
  const [newDomain, setNewDomain] = useState("");
  const [newAction, setNewAction] = useState<RuleAction>("direct");
  const [presets, setPresets] = useState<Preset[]>(FALLBACK_PRESETS);
```

Add three edit-state hooks right after them:

```tsx
  const [newDomain, setNewDomain] = useState("");
  const [newAction, setNewAction] = useState<RuleAction>("direct");
  const [presets, setPresets] = useState<Preset[]>(FALLBACK_PRESETS);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editDomain, setEditDomain] = useState("");
  const [editAction, setEditAction] = useState<RuleAction>("direct");
```

- [ ] **Step 2: Add the edit handlers next to the existing rule handlers**

Find `toggleRule` (`RoutingPage.tsx:110-118`):

```tsx
  const toggleRule = (id: string) => {
    save(
      state.routingRules.map((r) =>
        r.id === id ? { ...r, enabled: !r.enabled } : r
      ),
      state.defaultRoute,
      state.bridge
    );
  };
```

Add `startEdit`, `cancelEdit`, and `saveEdit` immediately after it. `saveEdit` mirrors `addRule`'s normalization and duplicate guard (`:85-100`), but excludes the rule being edited from the duplicate check and writes through the same `save()` helper:

```tsx
  const startEdit = (rule: RoutingRule) => {
    setEditingId(rule.id);
    setEditDomain(rule.domain);
    setEditAction(rule.action);
  };

  const cancelEdit = () => {
    setEditingId(null);
  };

  const saveEdit = () => {
    if (!editingId) return;
    const domain = editDomain.trim().toLowerCase();
    if (!domain) return;
    if (state.routingRules.some((r) => r.id !== editingId && r.domain === domain)) {
      toast(`Rule for ${domain} already exists`, "error");
      return;
    }
    save(
      state.routingRules.map((r) =>
        r.id === editingId ? { ...r, domain, action: editAction } : r
      ),
      state.defaultRoute,
      state.bridge
    );
    setEditingId(null);
  };
```

- [ ] **Step 3: Render an Edit button on each row and an inline editor for the row being edited**

Replace the rules `.map(...)` block (`RoutingPage.tsx:290-321`) — currently:

```tsx
            {state.routingRules.map((rule) => (
              <div
                key={rule.id}
                className={`routing-rule-card ${!rule.enabled ? "disabled" : ""}`}
              >
                <span
                  className="routing-action-badge"
                  style={{
                    background: `color-mix(in srgb, ${actionColor(rule.action)} 15%, transparent)`,
                    color: actionColor(rule.action),
                    borderColor: `color-mix(in srgb, ${actionColor(rule.action)} 30%, transparent)`,
                  }}
                >
                  {rule.action}
                </span>
                <span className="routing-rule-domain">{rule.domain}</span>
                <label className="toggle routing-rule-toggle">
                  <input
                    type="checkbox"
                    checked={rule.enabled}
                    onChange={() => toggleRule(rule.id)}
                  />
                  <span className="toggle-slider" />
                </label>
                <button
                  className="routing-rule-delete"
                  onClick={() => removeRule(rule.id)}
                >
                  x
                </button>
              </div>
            ))}
```

with the conditional version (when `editingId === rule.id`, show the inline editor; otherwise show the display row with a new Edit button before Delete):

```tsx
            {state.routingRules.map((rule) =>
              editingId === rule.id ? (
                <div key={rule.id} className="routing-rule-card editing">
                  <select
                    className="sort-select routing-action-select"
                    value={editAction}
                    onChange={(e) => setEditAction(e.target.value as RuleAction)}
                  >
                    <option value="direct">{t("routing.direct")}</option>
                    <option value="proxy">{t("routing.proxy")}</option>
                    <option value="block">{t("routing.block")}</option>
                    <option value="bridge">{t("routing.bridge")}</option>
                  </select>
                  <input
                    className="form-input routing-rule-edit-input"
                    type="text"
                    value={editDomain}
                    onChange={(e) => setEditDomain(e.target.value)}
                    onKeyDown={(e) => {
                      if (e.key === "Enter") saveEdit();
                      if (e.key === "Escape") cancelEdit();
                    }}
                    autoFocus
                  />
                  <button className="btn btn-primary btn-sm" onClick={saveEdit}>
                    {t("common.save")}
                  </button>
                  <button className="btn btn-secondary btn-sm" onClick={cancelEdit}>
                    {t("common.cancel")}
                  </button>
                </div>
              ) : (
                <div
                  key={rule.id}
                  className={`routing-rule-card ${!rule.enabled ? "disabled" : ""}`}
                >
                  <span
                    className="routing-action-badge"
                    style={{
                      background: `color-mix(in srgb, ${actionColor(rule.action)} 15%, transparent)`,
                      color: actionColor(rule.action),
                      borderColor: `color-mix(in srgb, ${actionColor(rule.action)} 30%, transparent)`,
                    }}
                  >
                    {rule.action}
                  </span>
                  <span className="routing-rule-domain">{rule.domain}</span>
                  <label className="toggle routing-rule-toggle">
                    <input
                      type="checkbox"
                      checked={rule.enabled}
                      onChange={() => toggleRule(rule.id)}
                    />
                    <span className="toggle-slider" />
                  </label>
                  <button
                    className="routing-rule-edit"
                    title={t("routing.editRule")}
                    onClick={() => startEdit(rule)}
                  >
                    ✎
                  </button>
                  <button
                    className="routing-rule-delete"
                    onClick={() => removeRule(rule.id)}
                  >
                    x
                  </button>
                </div>
              )
            )}
```

- [ ] **Step 4: Add CSS for the edit button and the editing row**

In `src/index.css`, find the end of the delete-button styles (`:1457-1459`):

```css
.routing-rule-delete:hover {
  color: var(--danger);
}
```

Add these rules immediately after:

```css
.routing-rule-edit {
  background: none;
  border: none;
  color: var(--text-muted);
  cursor: pointer;
  font-size: 14px;
  padding: 0 4px;
  opacity: 0;
  transition: all 0.15s;
}
.routing-rule-card:hover .routing-rule-edit {
  opacity: 1;
}
.routing-rule-edit:hover {
  color: var(--accent);
}
.routing-rule-edit-input {
  flex: 1;
  font-family: 'Consolas', 'Fira Code', monospace;
  font-size: 13px;
}
```

- [ ] **Step 5: Type-check and build**

Run: `npm run build`
Expected: PASS — `tsc` reports no type errors (the `RuleAction` cast, the new handlers, and the JSX all type-check) and `vite build` completes.

- [ ] **Step 6: Manual verification (no unit runner — run the app)**

Run: `npm run tauri dev`

Verify each scenario:
1. **Edit domain:** click ✎ on a rule, change the domain, click **Save** → the row shows the new domain; the "Routing rules saved" toast appears; reopening the app (or the Routing page) shows the change persisted.
2. **Edit action:** click ✎, change the action in the `<select>`, **Save** → the action badge/color updates and persists.
3. **Cancel:** click ✎, change the field, click **Cancel** (or press Escape) → the row reverts to its original values; nothing is saved.
4. **Enter to save:** click ✎, type a domain, press **Enter** → saves (same as clicking Save).
5. **Duplicate guard:** edit a rule's domain to match another existing rule's domain → an error toast appears and the edit is rejected (no save). Editing a rule and saving it with its **own** unchanged domain succeeds (not flagged as duplicate).
6. **No regressions:** add a rule, toggle a rule on/off, delete a rule, and apply a preset — all still work as before.

- [ ] **Step 7: Commit**

```bash
git add src/components/routing/RoutingPage.tsx src/index.css
git commit -m "feat(routing): edit rule domain and action in place"
```

---

## Notes / deviations from the spec

- **No `UPDATE_RULE` reducer action.** The spec's design sketch says "Dispatch an `UPDATE_RULE` action (by rule id) in `AppContext`." The established RoutingPage pattern is different: `addRule`/`removeRule`/`toggleRule` do **not** use per-operation reducer actions — they recompute the full rules array and call the local `save()` helper, which dispatches the single `SET_ROUTING_RULES` action and debounce-persists via `api.saveRoutingRules`. This plan follows that existing pattern (DRY, "follow established patterns") rather than adding a one-off reducer action that none of the sibling operations use. The end result is identical to the spec's intent — edit by id, persisted through `save_routing_rules`, no new backend command.
- **Inline editing, not a pre-filled add form.** The spec offered either "pre-fill the add-rule form" or "inline editable row." This plan uses inline editing (the row itself becomes editable) — it keeps the edit in context next to the rule and avoids a focus jump to the top-of-page add form.

## Self-review

- **Spec coverage:** "edit an existing rule's domain and action without delete + re-add" → Task 2 Steps 2-3. "Edit affordance per rule row" → Step 3 (the ✎ button). "Persist via existing `save_routing_rules`, no new backend command" → `save()` helper, confirmed `save_routing_rules` takes the full list. "Keep existing add/remove/toggle" → unchanged; verified in Step 6 scenario 6.
- **Placeholders:** none — every step has the literal code/commands.
- **Type consistency:** `editAction: RuleAction`, `editingId: string | null`, `editDomain: string` are used consistently across Steps 1-3; handler names `startEdit`/`cancelEdit`/`saveEdit` match between Step 2 (definition) and Step 3 (JSX usage); `t("common.save")`/`t("common.cancel")`/`t("routing.editRule")` match the keys added in Task 1.
