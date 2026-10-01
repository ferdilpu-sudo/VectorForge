import { useCallback, useEffect, useRef, useState } from "react";
import { useProject } from "../stores/project.store";
import { useLanguage } from "../shared/useLanguage";
import { ParamPanel } from "../features/params/ParamPanel";
import { CompareCanvas } from "../features/compare/CompareCanvas";
import { BatchQueue } from "../features/batch/BatchQueue";
import { SettingsDialog } from "../features/settings/SettingsDialog";
import { ExportDialog } from "../features/export/ExportDialog";
export function App() {
  const s = useProject();
  const t = useLanguage();
  const input = useRef<HTMLInputElement>(null);
  const [settings, setSettings] = useState(false);
  const [exporting, setExporting] = useState(false);
  const [panel, setPanel] = useState(true);
  const [dragging, setDragging] = useState(false);
  const active = s.files.find((f) => f.id === s.activeId);
  const open = useCallback(() => {
    if (!useProject.getState().batchBusy && !useProject.getState().importing)
      input.current?.click();
  }, []);
  useEffect(() => {
    const q = window.matchMedia("(prefers-color-scheme: light)");
    const apply = () => {
      document.documentElement.dataset.theme =
        s.settings.theme === "system"
          ? q.matches
            ? "light"
            : "dark"
          : s.settings.theme;
      document.documentElement.lang = s.settings.language;
    };
    apply();
    q.addEventListener("change", apply);
    return () => q.removeEventListener("change", apply);
  }, [s.settings.theme, s.settings.language]);
  useEffect(() => {
    const key = (e: KeyboardEvent) => {
      if (document.querySelector("dialog[open]")) return;
      if (e.ctrlKey && e.key.toLowerCase() === "o") {
        e.preventDefault();
        open();
      }
      if (e.ctrlKey && e.shiftKey && e.key.toLowerCase() === "e") {
        e.preventDefault();
        if (useProject.getState().activeId) setExporting(true);
      }
    };
    const leave = (e: BeforeUnloadEvent) => {
      if (useProject.getState().batchBusy) {
        e.preventDefault();
      }
    };
    window.addEventListener("keydown", key);
    window.addEventListener("beforeunload", leave);
    return () => {
      window.removeEventListener("keydown", key);
      window.removeEventListener("beforeunload", leave);
    };
  }, [open]);
  return (
    <main
      className={`app ${panel ? "" : "panel-collapsed"}`}
      onDragOver={(e) => {
        e.preventDefault();
        if (e.dataTransfer.types.includes("Files")) setDragging(true);
      }}
      onDragLeave={(e) => {
        if (!e.currentTarget.contains(e.relatedTarget as Node | null))
          setDragging(false);
      }}
      onDrop={(e) => {
        e.preventDefault();
        setDragging(false);
        void s.importFiles(Array.from(e.dataTransfer.files));
      }}
    >
      <header className="app-toolbar">
        <div className="brand">
          <span className="brand-mark">
            V<span>•</span>
          </span>
          VectorForge<span className="version">0.1</span>
        </div>
        <button
          onClick={() => setPanel(!panel)}
          aria-label={t("Tampilkan/sembunyikan panel", "Toggle settings panel")}
          aria-expanded={panel}
        >
          ☷
        </button>
        <button onClick={open} disabled={s.importing || s.batchBusy}>
          ＋ {t("Buka Gambar", "Open image")}
        </button>
        <span className="filename" title={active?.name}>
          {active?.name ?? t("Workspace baru", "New workspace")}
        </span>
        <button onClick={() => setSettings(true)}>
          {t("Pengaturan", "Settings")}
        </button>
        <button
          className="primary"
          disabled={!active || s.batchBusy}
          onClick={() => setExporting(true)}
        >
          {t("Ekspor…", "Export…")} ↗
        </button>
      </header>
      <div className="demo-banner">
        <span>◉ {t("MODE DEMO", "DEMO MODE")}</span>
        {t(
          "Preview dan ekspor simulasi · Engine belum terhubung.",
          "Simulated preview and export · Engine not connected.",
        )}
      </div>
      <input
        hidden
        ref={input}
        type="file"
        accept=".png,.jpg,.jpeg,.webp,.bmp"
        multiple
        onChange={(e) => {
          void s.importFiles(Array.from(e.target.files ?? []));
          e.target.value = "";
        }}
      />
      {s.notice && (
        <div className="notice" role="alert">
          <span>{s.notice}</span>
          <button
            aria-label={t("Tutup pemberitahuan", "Dismiss notification")}
            onClick={() => s.notify("")}
          >
            ×
          </button>
        </div>
      )}
      <div className="workspace">
        {panel && <ParamPanel />}
        <div className="work-area">
          <CompareCanvas file={active} onOpen={open} />
          {s.files.length > 0 && <BatchQueue />}
        </div>
      </div>
      <footer className="status-bar">
        <span className="status-dot" />
        {s.importing
          ? t("Membaca gambar…", "Reading images…")
          : t("Lokal · Mode Demo", "Local · Demo mode")}
        <span className="status-meta">
          {active
            ? `${active.width} × ${active.height} px · ${(active.bytes / 1024).toFixed(1)} KB`
            : "PNG / JPG / WEBP / BMP"}
        </span>
        <span>
          {t(
            "Ekspor produksi: resolusi penuh",
            "Production export: full resolution",
          )}
        </span>
      </footer>
      {dragging && (
        <div className="drop-overlay">
          {s.batchBusy
            ? t("Tunggu batch selesai", "Wait for the batch to finish")
            : t("Lepaskan untuk menambahkan gambar", "Drop to add images")}
        </div>
      )}
      {settings && <SettingsDialog onClose={() => setSettings(false)} />}
      {exporting && active && (
        <ExportDialog file={active} onClose={() => setExporting(false)} />
      )}
    </main>
  );
}
