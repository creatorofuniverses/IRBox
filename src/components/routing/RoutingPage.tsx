import { useState, useCallback, useRef, useEffect } from "react";
import { useApp } from "../../context/AppContext";
import { api, RoutingRule, RuleAction } from "../../api/tauri";
import { t } from "../../i18n/translations";

export interface Preset {
  id: string;
  name_en: string;
  action: RuleAction;
  domains: string[];
}

const FALLBACK_PRESETS: Preset[] = [
  {
    id: "block_ads",
    name_en: "Block Ads",
    action: "block",
    domains: [
      "doubleclick.net",
      "googlesyndication.com",
      "googleadservices.com",
      "adnxs.com",
      "ads.yahoo.com",
      "moatads.com",
      "adcolony.com",
      "direct.yandex.ru",
    ],
  },
];

const PRESETS_URL =
  "https://gitea.com/IranGuard/IRBox/raw/branch/master/configs.json";

let idCounter = Date.now();
function nextId() {
  return String(++idCounter);
}

export function RoutingPage() {
  const { state, dispatch, toast } = useApp();
  void state.langTick;

  const [newDomain, setNewDomain] = useState("");
  const [newAction, setNewAction] = useState<RuleAction>("direct");
  const [presets, setPresets] = useState<Preset[]>(FALLBACK_PRESETS);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editDomain, setEditDomain] = useState("");
  const [editAction, setEditAction] = useState<RuleAction>("direct");
  const saveTimer = useRef<ReturnType<typeof setTimeout>>();

  useEffect(() => {
    fetch(PRESETS_URL)
      .then((res) => {
        if (!res.ok) throw new Error("HTTP " + res.status);
        return res.json();
      })
      .then((data: { presets: Preset[] }) => {
        if (data.presets && data.presets.length > 0) {
          setPresets(data.presets);
        }
      })
      .catch(() => {
        // Use fallback presets silently
      });
  }, []);

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

  const setDefaultRoute = (route: string) => {
    save(state.routingRules, route);
  };

  const addRule = () => {
    const domain = newDomain.trim().toLowerCase();
    if (!domain) return;
    if (state.routingRules.some((r) => r.domain === domain)) {
      toast(`Rule for ${domain} already exists`, "error");
      return;
    }
    const rule: RoutingRule = {
      id: nextId(),
      domain,
      action: newAction,
      enabled: true,
    };
    save([...state.routingRules, rule], state.defaultRoute);
    setNewDomain("");
  };

  const removeRule = (id: string) => {
    save(
      state.routingRules.filter((r) => r.id !== id),
      state.defaultRoute
    );
  };

  const toggleRule = (id: string) => {
    save(
      state.routingRules.map((r) =>
        r.id === id ? { ...r, enabled: !r.enabled } : r
      ),
      state.defaultRoute
    );
  };

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
      state.defaultRoute
    );
    setEditingId(null);
  };

  const addPreset = (domains: string[], action: RuleAction) => {
    const existing = new Set(state.routingRules.map((r) => r.domain));
    const newRules = domains
      .filter((d) => !existing.has(d))
      .map((domain) => ({
        id: nextId(),
        domain,
        action,
        enabled: true,
      }));
    if (newRules.length === 0) {
      toast("All preset domains already added", "info");
      return;
    }
    save([...state.routingRules, ...newRules], state.defaultRoute);
  };

  const actionColor = (action: RuleAction) => {
    switch (action) {
      case "proxy":
        return "var(--accent)";
      case "direct":
        return "var(--success)";
      case "block":
        return "var(--danger)";
      case "bridge":
        return "var(--warning)";
    }
  };

  const needsActiveIface =
    state.activeInterfaceId === null &&
    state.routingRules.some((r) => r.enabled && r.action === "bridge");

  return (
    <div className="routing-page">
      <h2>{t("routing.title")}</h2>

      {/* Default route toggle */}
      <div className="settings-section">
        <div className="settings-label">{t("routing.defaultRoute")}</div>
        <div className="vpn-mode-group">
          <label
            className={`vpn-mode-card ${state.defaultRoute === "proxy" ? "active" : ""}`}
            onClick={() => setDefaultRoute("proxy")}
          >
            <input
              type="radio"
              name="defaultRoute"
              checked={state.defaultRoute === "proxy"}
              onChange={() => setDefaultRoute("proxy")}
            />
            <div className="vpn-mode-info">
              <span className="vpn-mode-title">{t("routing.proxyAll")}</span>
              <span className="vpn-mode-desc">{t("routing.proxyAllDesc")}</span>
            </div>
          </label>
          <label
            className={`vpn-mode-card ${state.defaultRoute === "direct" ? "active" : ""}`}
            onClick={() => setDefaultRoute("direct")}
          >
            <input
              type="radio"
              name="defaultRoute"
              checked={state.defaultRoute === "direct"}
              onChange={() => setDefaultRoute("direct")}
            />
            <div className="vpn-mode-info">
              <span className="vpn-mode-title">{t("routing.directAll")}</span>
              <span className="vpn-mode-desc">{t("routing.directAllDesc")}</span>
            </div>
          </label>
        </div>
      </div>

      {/* Add rule form */}
      <div className="settings-section">
        <div className="settings-label">{t("routing.addRule")}</div>
        <div className="routing-add-form">
          <input
            className="form-input routing-domain-input"
            type="text"
            placeholder={t("routing.domainPlaceholder")}
            value={newDomain}
            onChange={(e) => setNewDomain(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && addRule()}
          />
          <select
            className="sort-select routing-action-select"
            value={newAction}
            onChange={(e) => setNewAction(e.target.value as RuleAction)}
          >
            <option value="direct">{t("routing.direct")}</option>
            <option value="proxy">{t("routing.proxy")}</option>
            <option value="block">{t("routing.block")}</option>
            <option value="bridge">{t("routing.bridge")}</option>
          </select>
          <button className="btn btn-primary btn-sm" onClick={addRule}>
            {t("common.add")}
          </button>
        </div>
      </div>

      {/* Presets */}
      <div className="settings-section">
        <div className="settings-label">{t("routing.presets")}</div>
        <div className="routing-presets">
          {presets.map((preset) => (
            <button
              key={preset.id}
              className="btn btn-secondary btn-sm"
              onClick={() => addPreset(preset.domains, preset.action)}
            >
              {preset.name_en}
            </button>
          ))}
        </div>
      </div>

      {/* Rules list */}
      <div className="settings-section">
        <div className="settings-label">
          {t("routing.rules")} ({state.routingRules.length})
        </div>
        {needsActiveIface && (
          <div
            className="vpn-mode-desc"
            style={{ cursor: "pointer", color: "var(--warning)" }}
            onClick={() => dispatch({ type: "SET_PAGE", page: "interfaces" })}
          >
            {t("interfaces.noActiveHint")}
          </div>
        )}
        {state.routingRules.length === 0 ? (
          <div className="empty-list">{t("routing.noRules")}</div>
        ) : (
          <div className="routing-rules-list">
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
          </div>
        )}
      </div>
    </div>
  );
}
