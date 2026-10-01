import { useState } from "react";

import { defaultSettings } from "../../services/preferences";
import { Modal } from "../../shared/Modal";
import { useLanguage } from "../../shared/useLanguage";
import { useProject } from "../../stores/project.store";

export function SettingsDialog({ onClose }: { onClose: () => void }) {
  const settings = useProject((state) => state.settings);
  const update = useProject((state) => state.setSettings);
  const t = useLanguage();
  const [confirmReset, setConfirmReset] = useState(false);

  return (
    <Modal title={t("Pengaturan", "Settings")} onClose={onClose}>
      <p className="muted">
        {t(
          "Preferensi disimpan lokal di data aplikasi VectorForge.",
          "Preferences are stored locally in VectorForge app data.",
        )}
      </p>

      <label className="field">
        {t("Bahasa", "Language")}
        <select
          value={settings.language}
          onChange={(event) =>
            update({
              ...settings,
              language: event.target.value as "id" | "en",
            })
          }
        >
          <option value="id">Indonesia</option>
          <option value="en">English</option>
        </select>
      </label>

      <label className="field">
        {t("Tema", "Theme")}
        <select
          value={settings.theme}
          onChange={(event) =>
            update({
              ...settings,
              theme: event.target.value as typeof settings.theme,
            })
          }
        >
          <option value="dark">{t("Gelap", "Dark")}</option>
          <option value="light">{t("Terang", "Light")}</option>
          <option value="system">{t("Sistem", "System")}</option>
        </select>
      </label>

      <label className="field">
        {t("Worker engine", "Engine workers")}
        <select
          value={settings.workerCount}
          onChange={(event) =>
            update({ ...settings, workerCount: Number(event.target.value) })
          }
        >
          {[1, 2, 3, 4].map((count) => (
            <option key={count}>{count}</option>
          ))}
        </select>
      </label>

      <label className="field">
        {t("Sisi maksimum preview", "Preview maximum side")}
        <select
          value={settings.previewMaxSide}
          onChange={(event) =>
            update({
              ...settings,
              previewMaxSide: Number(event.target.value),
            })
          }
        >
          {[512, 1024, 2048].map((size) => (
            <option key={size}>{size}</option>
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
