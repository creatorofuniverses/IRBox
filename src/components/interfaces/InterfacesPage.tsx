import { useState } from "react";
import { useApp } from "../../context/AppContext";
import { api, InterfaceConfig } from "../../api/tauri";
import { t } from "../../i18n/translations";
import { Button } from "../ui/Button";
import { InterfaceModal } from "./InterfaceModal";

export function InterfacesPage() {
  const { state, dispatch, toast } = useApp();
  void state.langTick;

  const [modalOpen, setModalOpen] = useState(false);
  const [editing, setEditing] = useState<InterfaceConfig | null>(null);

  const refresh = async () => {
    const res = await api.getInterfaces();
    dispatch({ type: "SET_INTERFACES", interfaces: res.interfaces, activeInterfaceId: res.active_interface_id });
  };

  const openAdd = () => {
    setEditing(null);
    setModalOpen(true);
  };

  const openEdit = (iface: InterfaceConfig) => {
    setEditing(iface);
    setModalOpen(true);
  };

  const toggleActive = async (id: string) => {
    const next = state.activeInterfaceId === id ? null : id;
    try {
      await api.setActiveInterface(next);
      await refresh();
    } catch (e) {
      toast(`${e}`, "error");
    }
  };

  const remove = async (id: string) => {
    try {
      await api.deleteInterface(id);
      await refresh();
    } catch (e) {
      toast(`${e}`, "error");
    }
  };

  return (
    <div className="sub-page">
      <div className="sub-header">
        <h2>{t("interfaces.title")}</h2>
        <Button onClick={openAdd}>{t("interfaces.add")}</Button>
      </div>

      {state.interfaces.length === 0 ? (
        <div className="empty-list">{t("interfaces.empty")}</div>
      ) : (
        <div className="sub-list">
          {state.interfaces.map((iface) => {
            const active = state.activeInterfaceId === iface.id;
            return (
              <div key={iface.id} className={`sub-card ${active ? "sub-featured" : ""}`}>
                <div className="sub-info">
                  <span className="sub-name">
                    {iface.label}
                    {active && <span className="sub-featured-badge">{t("interfaces.active")}</span>}
                  </span>
                  <span className="sub-url">{iface.interface}</span>
                  <span className="sub-meta">
                    {iface.endpoints.length} endpoints
                    {iface.routing_mark != null ? ` · fwmark ${iface.routing_mark}` : ""}
                  </span>
                </div>
                <div className="sub-actions">
                  <Button
                    variant={active ? "primary" : "secondary"}
                    size="sm"
                    onClick={() => toggleActive(iface.id)}
                  >
                    {t("interfaces.use")}
                  </Button>
                  <Button variant="secondary" size="sm" onClick={() => openEdit(iface)}>
                    {t("common.edit")}
                  </Button>
                  <Button variant="danger" size="sm" onClick={() => remove(iface.id)}>
                    {t("common.delete")}
                  </Button>
                </div>
              </div>
            );
          })}
        </div>
      )}

      <InterfaceModal open={modalOpen} onClose={() => setModalOpen(false)} editing={editing} />
    </div>
  );
}
