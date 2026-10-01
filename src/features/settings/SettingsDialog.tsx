import { useState } from "react";
import { defaultSettings } from "../../services/preferences";
import { Modal } from "../../shared/Modal";
import { useProject } from "../../stores/project.store";
import { useLanguage } from "../../shared/useLanguage";
export function SettingsDialog({ onClose }: { onClose: () => void }) {
  const s = useProject((s) => s.settings);
  const update = useProject((s) => s.setSettings);
  const t = useLanguage();
  const [confirmReset, setConfirmReset] = useState(false);
  return (
    <Modal title={t("Pengaturan", "Settings")} onClose={onClose}>
      <p className="muted">
        {t(
          "Preferensi demo disimpan di browser ini.",
          "Demo preferences are stored in this browser.",
        )}
      </p>
      <label className="field">
        {t("Bahasa", "Language")}
        <select
          value={s.language}
          onChange={(e) =>
            update({ ...s, language: e.target.value as "id" | "en" })
          }
        >
          <option value="id">Indonesia</option>
          <option value="en">English</option>
        </select>
      </label>
      <label className="field">
        {t("Tema", "Theme")}
        <select
          value={s.theme}
          onChange={(e) =>
            update({ ...s, theme: e.target.value as typeof s.theme })
          }
        >
          <option value="dark">{t("Gelap", "Dark")}</option>
          <option value="light">{t("Terang", "Light")}</option>
          <option value="system">{t("Sistem", "System")}</option>
        </select>
      </label>
      <label className="field">
        {t("Worker (untuk engine mendatang)", "Workers (future engine)")}
        <select
          value={s.workerCount}
          onChange={(e) =>
            update({ ...s, workerCount: Number(e.target.value) })
          }
        >
          {[1, 2, 3, 4].map((n) => (
            <option key={n}>{n}</option>
          ))}
        </select>
      </label>
      <label className="field">
        {t("Sisi maksimum preview", "Preview maximum side")}
        <select
          value={s.previewMaxSide}
          onChange={(e) =>
            update({ ...s, previewMaxSide: Number(e.target.value) })
          }
        >
          {[512, 1024, 2048].map((n) => (
            <option key={n}>{n}</option>
          ))}
        </select>
      </label>
      {confirmReset ? (
        <p className="demo-inline">
          {t(
            "Kembalikan pengaturan ke default? Preset tetap disimpan.",
            "Reset settings to defaults? Presets are kept.",
          )}{" "}
          <button
            onClick={() => {
              update({ ...defaultSettings });
              setConfirmReset(false);
            }}
          >
            {t("Ya, reset", "Yes, reset")}
          </button>{" "}
          <button onClick={() => setConfirmReset(false)}>
            {t("Batal", "Cancel")}
          </button>
        </p>
      ) : (
        <button className="wide" onClick={() => setConfirmReset(true)}>
          {t("Reset Pengaturan", "Reset settings")}
        </button>
      )}
      <button className="primary wide" onClick={onClose}>
        {t("Selesai", "Done")}
      </button>
    </Modal>
  );
}
