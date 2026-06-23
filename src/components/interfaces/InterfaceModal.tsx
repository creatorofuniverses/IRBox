import { useState, useEffect } from "react";
import { Modal } from "../ui/Modal";
import { Button } from "../ui/Button";
import { Spinner } from "../ui/Spinner";
import { useApp } from "../../context/AppContext";
import { api, InterfaceConfig } from "../../api/tauri";
import { t } from "../../i18n/translations";

interface Props {
  open: boolean;
  onClose: () => void;
  /** The interface to edit, or null to add a new one. */
  editing: InterfaceConfig | null;
}

const parseEndpoints = (raw: string): string[] =>
  raw.split(/[\s,]+/).map((s) => s.trim()).filter((s) => s.length > 0);

// Loose IP / CIDR check: dotted-quad or IPv6-ish, optional /prefix.
const isEndpoint = (s: string): boolean =>
  /^[0-9a-fA-F:.]+(\/\d{1,3})?$/.test(s);

export function InterfaceModal({ open, onClose, editing }: Props) {
  const { dispatch, toast } = useApp();
  const [label, setLabel] = useState("");
  const [iface, setIface] = useState("");
  const [endpoints, setEndpoints] = useState("");
  const [mark, setMark] = useState("");
  const [loading, setLoading] = useState(false);

  // Re-seed the form whenever the modal opens (add vs edit).
  useEffect(() => {
    if (open) {
      setLabel(editing?.label ?? "");
      setIface(editing?.interface ?? "");
      setEndpoints(editing?.endpoints.join(", ") ?? "");
      setMark(editing?.routing_mark != null ? String(editing.routing_mark) : "");
    }
  }, [open, editing]);

  const handleSave = async () => {
    const interfaceName = iface.trim();
    if (!interfaceName) {
      toast(t("interfaces.errInterfaceRequired"), "error");
      return;
    }
    if (/\s/.test(interfaceName)) {
      toast(t("interfaces.errInterfaceFormat"), "error");
      return;
    }
    const eps = parseEndpoints(endpoints);
    const bad = eps.find((e) => !isEndpoint(e));
    if (bad) {
      toast(`${t("interfaces.errEndpoint")}: ${bad}`, "error");
      return;
    }
    const config: InterfaceConfig = {
      id: editing?.id ?? "",
      label: label.trim(),
      interface: interfaceName,
      routing_mark: mark.trim() === "" ? null : Number(mark),
      endpoints: eps,
    };
    setLoading(true);
    try {
      await api.saveInterface(config);
      const res = await api.getInterfaces();
      dispatch({ type: "SET_INTERFACES", interfaces: res.interfaces, activeInterfaceId: res.active_interface_id });
      onClose();
    } catch (e) {
      toast(`${e}`, "error");
    }
    setLoading(false);
  };

  return (
    <Modal open={open} onClose={onClose} title={editing ? t("interfaces.editTitle") : t("interfaces.addTitle")}>
      <div className="form-group">
        <label className="form-label">{t("interfaces.interfaceField")}</label>
        <input
          className="form-input"
          type="text"
          placeholder={t("interfaces.interfacePlaceholder")}
          value={iface}
          onChange={(e) => setIface(e.target.value)}
        />
      </div>
      <div className="form-group">
        <label className="form-label">{t("interfaces.labelField")}</label>
        <input
          className="form-input"
          type="text"
          placeholder={t("interfaces.labelPlaceholder")}
          value={label}
          onChange={(e) => setLabel(e.target.value)}
        />
      </div>
      <div className="form-group">
        <label className="form-label">{t("interfaces.endpointsField")}</label>
        <input
          className="form-input"
          type="text"
          placeholder={t("interfaces.endpointsPlaceholder")}
          value={endpoints}
          onChange={(e) => setEndpoints(e.target.value)}
        />
      </div>
      <div className="form-group">
        <label className="form-label">{t("interfaces.markField")}</label>
        <input
          className="form-input"
          type="number"
          placeholder={t("interfaces.markPlaceholder")}
          value={mark}
          onChange={(e) => setMark(e.target.value)}
        />
      </div>
      <div className="form-actions">
        <Button onClick={handleSave} disabled={loading}>
          {loading ? <Spinner size={14} /> : t("common.save")}
        </Button>
        <Button variant="ghost" onClick={onClose}>
          {t("common.cancel")}
        </Button>
      </div>
    </Modal>
  );
}
