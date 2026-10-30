import { useState } from "react";
import { useProject } from "../../stores/project.store";
import { builtIns, ranges } from "../../types/params";
import { useLanguage } from "../../shared/useLanguage";
import { Modal } from "../../shared/Modal";
import { ParamField } from "./ParamField";

type Translate = (id: string, en: string) => string;

function presetLabel(
  id: string,
  fallback: string,
  builtIn: boolean,
  t: Translate,
): string {
  if (!builtIn) return fallback;
  switch (id) {
    case "builtin-balanced":
      return t("Seimbang", "Balanced");
    case "builtin-logo":
      return t("Logo & Flat", "Logo & Flat");
    case "builtin-photo":
      return t("Foto Detail", "Detailed Photo");
    case "builtin-poster":
      return t("Poster Halus", "Smooth Poster");
    case "builtin-ai-smooth":
      return t("AI Ilustrasi Halus (Bawaan)", "AI Smooth Illustration (Built-in)");
    case "builtin-professional-logo":
      return t("Logo Presisi (Eksperimen)", "Precision Logo (Experimental)");
    case "builtin-professional-illustration":
      return t("Ilustrasi Profesional (Eksperimen)", "Professional Illustration (Experimental)");
    default:
      return fallback;
  }
}

export function ParamPanel() {
  const params = useProject((state) => state.params);
  const presetId = useProject((state) => state.presetId);
  const presets = useProject((state) => state.presets);
  const appSettings = useProject((state) => state.settings);
  const setParams = useProject((state) => state.setParams);
  const choosePreset = useProject((state) => state.choosePreset);
  const savePreset = useProject((state) => state.savePreset);
  const deletePreset = useProject((state) => state.deletePreset);
  const setSettings = useProject((state) => state.setSettings);
  const t = useLanguage();
  const [saving, setSaving] = useState(false);
  const [name, setName] = useState("");
  const [error, setError] = useState("");
  const [deleting, setDeleting] = useState(false);
  const selected = [...builtIns, ...presets].find((p) => p.id === presetId);
  const dirty = JSON.stringify(selected?.params) !== JSON.stringify(params);
  const field = (key: keyof typeof ranges, label: string) => {
    const [min, max, step] = ranges[key];
    return (
      <ParamField
        key={key}
        label={label}
        value={params[key]}
        min={min}
        max={max}
        step={step}
        onChange={(n) => setParams({ ...params, [key]: n })}
      />
    );
  };
  return (
    <aside className="param-panel">
      <h2>{t("Pengaturan vektor", "Vector settings")}</h2>
      <label className="field">
        Preset
        <select
          value={presetId}
          onChange={(e) => choosePreset(e.target.value)}
        >
          {[...builtIns, ...presets].map((p) => (
            <option key={p.id} value={p.id}>
              {presetLabel(p.id, p.name, p.builtIn, t)}
            </option>
          ))}
        </select>
      </label>
      <div className="preset-meta">
        <span className="muted">
          {dirty
            ? t("● Diubah", "● Modified")
            : t("Pengaturan tersimpan", "Saved settings")}
        </span>
        <button
          className="text-button"
          onClick={() => {
            setSaving(true);
            setError("");
          }}
        >
          {t("Simpan preset", "Save preset")}
        </button>
      </div>
      <hr />
      <h3>{t("Detail gambar", "Image detail")}</h3>
      {field("colorPrecision", t("Detail Warna", "Color precision"))}
      {field("filterSpeckle", t("Buang Noda Kecil", "Remove speckles"))}
      <p className="helper">
        {t(
          "Nilai lebih besar menghapus detail kecil.",
          "Higher values remove small details.",
        )}
      </p>
      <label className="field">
        {t("Mode Kurva", "Curve mode")}
        <select
          value={params.mode}
          onChange={(e) =>
            setParams({
              ...params,
              mode: e.target.value as "spline" | "polygon",
            })
          }
        >
          <option value="spline">Spline</option>
          <option value="polygon">Polygon</option>
        </select>
      </label>
      {field("smoothness", t("Kehalusan Kurva", "Curve smoothness"))}
      <p className="helper">
        {params.mode === "polygon"
          ? t(
              "Kehalusan kurva hanya berlaku pada mode Spline.",
              "Curve smoothing only applies in Spline mode.",
            )
          : t(
              "0 = bentuk asli; nilai lebih besar merapikan kurva, tetapi bisa mengurangi detail.",
              "0 = original curves; higher values simplify paths but may remove detail.",
            )}
      </p>
      <details>
        <summary>{t("Pengaturan Lanjutan", "Advanced settings")}</summary>
        <label className="field">
          {t("Segmentasi Gambar", "Image segmentation")}
          <select
            value={params.segmentationMode}
            onChange={(e) =>
              setParams({
                ...params,
                segmentationMode: e.target.value as "color-cluster" | "watershed",
              })
            }
          >
            <option value="color-cluster">
              {t("Warna (Klasik)", "Color clustering (classic)")}
            </option>
            <option value="watershed">
              {t("Tepi Objek (Eksperimen)", "Object edges (experimental)")}
            </option>
          </select>
        </label>
        <label className="field">
          {t("Rekonstruksi Geometri", "Geometry reconstruction")}
          <select
            value={params.geometryMode}
            onChange={(e) =>
              setParams({
                ...params,
                geometryMode: e.target.value as "classic" | "line-aware",
              })
            }
          >
            <option value="classic">{t("Klasik", "Classic")}</option>
            <option value="line-aware">
              {t("Jaga Garis Lurus (Eksperimen)", "Preserve straight edges (experimental)")}
            </option>
          </select>
        </label>
        <p className="helper">
          {t(
            "Menormalkan handle Bézier yang hampir lurus tanpa memindahkan titik sudut; berlaku hanya untuk Spline.",
            "Straightens nearly linear Bézier handles without moving corner anchors; Spline only.",
          )}
        </p>
        <label className="check">
          <input
            type="checkbox"
            checked={params.qualityGuard}
            onChange={(e) => setParams({ ...params, qualityGuard: e.target.checked })}
          />
          {t("Pelindung Kualitas Kontur (Eksperimen)", "Contour Quality Guard (Experimental)")}
        </label>
        <p className="helper">
          {t(
            "Batalkan smoothing pada kontur yang terlalu berubah. Belum memeriksa kesetiaan warna atau segmentasi gambar.",
            "Reject excessive contour drift from smoothing. Does not evaluate segmentation or color fidelity.",
          )}
        </p>
        {field("paletteLimit", t("Batas Jumlah Warna", "Color palette limit"))}
        <p className="helper">
          {t(
            "0 = tanpa batas palet tambahan. Nilai lain menyederhanakan warna dan dapat menghapus gradasi.",
            "0 = no additional palette limit. Other values simplify colors and may remove gradients.",
          )}
        </p>
        {field("layerDifference", t("Selisih Layer", "Layer difference"))}
        {field("cornerThreshold", t("Sudut Tajam", "Corner threshold"))}
        {field("lengthThreshold", t("Panjang Segmen", "Segment length"))}
        <label className="field">
          {t("Susunan Layer", "Hierarchy")}
          <select
            value={params.hierarchical}
            onChange={(e) =>
              setParams({
                ...params,
                hierarchical: e.target.value as "stacked" | "cutout",
              })
            }
          >
            <option value="stacked">Stacked</option>
            <option value="cutout">Cutout</option>
          </select>
        </label>
      </details>
      <hr />
      <label className="check">
        <input
          type="checkbox"
          checked={appSettings.autoPreview}
          onChange={(e) =>
            setSettings({ ...appSettings, autoPreview: e.target.checked })
          }
        />
        {t("Preview otomatis", "Automatic preview")}
      </label>
      <button
        className="wide"
        onClick={() => choosePreset(presetId)}
      >
        {t("Reset Parameter", "Reset parameters")}
      </button>
      {selected && !selected.builtIn && (
        <button
          className="text-button danger"
          onClick={() => setDeleting(true)}
        >
          {t("Hapus preset ini", "Delete this preset")}
        </button>
      )}
      <div className="local-note">
        ◈{" "}
        {t("Gambar tetap di perangkat Anda", "Your images stay on your device")}
      </div>
      {saving && (
        <Modal
          title={t("Simpan preset", "Save preset")}
          onClose={() => setSaving(false)}
        >
          <form
            onSubmit={(e) => {
              e.preventDefault();
              void savePreset(name).then((saved) => {
                if (saved) {
                  setSaving(false);
                  setName("");
                } else {
                  setError(
                    t(
                      "Nama wajib unik dan 1–40 karakter.",
                      "Use a unique name, 1–40 characters.",
                    ),
                  );
                }
              });
            }}
          >
            <label className="field">
              {t("Nama preset", "Preset name")}
              <input
                autoFocus
                value={name}
                onChange={(e) => setName(e.target.value)}
              />
            </label>
            {error && (
              <p role="alert" className="danger">
                {error}
              </p>
            )}
            <button className="primary" type="submit">
              {t("Simpan", "Save")}
            </button>
          </form>
        </Modal>
      )}
      {deleting && (
        <Modal
          title={t("Hapus preset?", "Delete preset?")}
          onClose={() => setDeleting(false)}
        >
          <p>{selected?.name}</p>
          <button onClick={() => setDeleting(false)}>
            {t("Batal", "Cancel")}
          </button>{" "}
          <button
            className="primary"
            onClick={() => {
              void deletePreset(presetId);
              setDeleting(false);
            }}
          >
            {t("Hapus", "Delete")}
          </button>
        </Modal>
      )}
    </aside>
  );
}
