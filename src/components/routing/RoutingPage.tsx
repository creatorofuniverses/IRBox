import { useState, useCallback, useRef, useEffect } from "react";
import { useApp } from "../../context/AppContext";
import { api, RoutingRule, RuleAction, BridgeConfig } from "../../api/tauri";
import { t } from "../../i18n/translations";
import { getLang } from "../../i18n/translations";

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
    (rules: RoutingRule[], defaultRoute: string, bridge: BridgeConfig) => {
      dispatch({ type: "SET_ROUTING_RULES", rules, defaultRoute, bridge });
      clearTimeout(saveTimer.current);
      saveTimer.current = setTimeout(async () => {
        try {
          await api.saveRoutingRules(rules, defaultRoute, bridge);
          toast(t("routing.saved"), "success");
        } catch (e) {
          toast(`${e}`, "error");
        }
      }, 800);
    },
    [dispatch, toast]
  );

  const setDefaultRoute = (route: string) => {
    save(state.routingRules, route, state.bridge);
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
    save([...state.routingRules, rule], state.defaultRoute, state.bridge);
    setNewDomain("");
  };

  const removeRule = (id: string) => {
    save(
      state.routingRules.filter((r) => r.id !== id),
      state.defaultRoute,
      state.bridge
    );
  };

  const toggleRule = (id: string) => {
    save(
      state.routingRules.map((r) =>
        r.id === id ? { ...r, enabled: !r.enabled } : r
      ),
      state.defaultRoute,
      state.bridge
    );
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
    save([...state.routingRules, ...newRules], state.defaultRoute, state.bridge);
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

  const parseEndpoints = (raw: string): string[] =>
    raw.split(/[\s,]+/).map((s) => s.trim()).filter((s) => s.length > 0);

  const setBridge = (patch: Partial<BridgeConfig>) =>
    save(state.routingRules, state.defaultRoute, { ...state.bridge, ...patch });

  const lang = getLang();

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

      {/* Bridge / Custom interface routing settings */}
      <div className="settings-section">
        <div className="settings-label">{t("routing.bridgeSettings")}</div>
        <div className="vpn-mode-desc">{t("routing.bridgeHelp")}</div>
        <div className="form-group">
          <label className="form-label">{t("routing.bridgeInterface")}</label>
          <input
            className="form-input"
            type="text"
            placeholder={t("routing.bridgeInterfacePlaceholder")}
            value={state.bridge.interface ?? ""}
            onChange={(e) => setBridge({ interface: e.target.value.trim() || null })}
          />
        </div>
        <div className="form-group">
          <label className="form-label">{t("routing.bridgeEndpoints")}</label>
          <input
            className="form-input"
            type="text"
            placeholder={t("routing.bridgeEndpointsPlaceholder")}
            value={state.bridge.endpoints.join(", ")}
            onChange={(e) => setBridge({ endpoints: parseEndpoints(e.target.value) })}
          />
        </div>
        <div className="form-group">
          <label className="form-label">{t("routing.bridgeMark")}</label>
          <input
            className="form-input"
            type="number"
            placeholder={t("routing.bridgeMarkPlaceholder")}
            value={state.bridge.routing_mark ?? ""}
            onChange={(e) =>
              setBridge({ routing_mark: e.target.value === "" ? null : Number(e.target.value) })
            }
          />
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
        {state.routingRules.length === 0 ? (
          <div className="empty-list">{t("routing.noRules")}</div>
        ) : (
          <div className="routing-rules-list">
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
          </div>
        )}
      </div>
    </div>
  );
}
