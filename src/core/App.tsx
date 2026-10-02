import { useCallback, useEffect, useState } from "react";

import { api } from "../services/api";
import { appErrorMessage } from "../services/errors";
import { BatchQueue } from "../features/batch/BatchQueue";
import { CompareCanvas } from "../features/compare/CompareCanvas";
import { ExportDialog } from "../features/export/ExportDialog";
import { ParamPanel } from "../features/params/ParamPanel";
import { SettingsDialog } from "../features/settings/SettingsDialog";
import { useLanguage } from "../shared/useLanguage";
import { useProject } from "../stores/project.store";

export function App() {
  const files = useProject((state) => state.files);
  const activeId = useProject((state) => state.activeId);
  const appSettings = useProject((state) => state.settings);
  const notice = useProject((state) => state.notice);
  const ready = useProject((state) => state.ready);
  const importing = useProject((state) => state.importing);
  const batchBusy = useProject((state) => state.batchBusy);
  const initialize = useProject((state) => state.initialize);
  const openFiles = useProject((state) => state.openFiles);
  const importPaths = useProject((state) => state.importPaths);
  const notify = useProject((state) => state.notify);
  const t = useLanguage();
  const [settings, setSettings] = useState(false);
  const [exporting, setExporting] = useState(false);
  const [panel, setPanel] = useState(true);
  const [dragging, setDragging] = useState(false);
  const active = files.find((file) => file.id === activeId);

  const open = useCallback(() => {
    const state = useProject.getState();
    if (!state.batchBusy && !state.importing && state.ready) {
      void openFiles();
    }
  }, [openFiles]);

  useEffect(() => {
    void initialize();
  }, [initialize]);

  useEffect(() => {
    let disposed = false;
    let stop: (() => void) | undefined;

    void api
      .watchNativeDrops((event) => {
        if (event.type === "over") {
          setDragging(true);
          return;
        }

        setDragging(false);
        if (event.type === "drop") {
          const state = useProject.getState();
          if (!state.batchBusy && !state.importing && state.ready) {
            void importPaths(event.paths);
          }
        }
      })
      .then((unlisten) => {
        if (disposed) unlisten();
        else stop = unlisten;
      })
      .catch((error) => {
        if (!disposed) {
          notify(
            appErrorMessage(error, useProject.getState().settings.language),
          );
        }
      });

    return () => {
      disposed = true;
      stop?.();
    };
  }, [importPaths, notify]);

  useEffect(() => {
    let disposed = false;
    let stop: (() => void) | undefined;
    let asking = false;

    void api
      .watchCloseRequests((preventDefault) => {
        if (!useProject.getState().batchBusy) return;

        preventDefault();
        if (asking) return;
        asking = true;

        const language = useProject.getState().settings.language;
        void api
          .confirmCloseWhileBusy(language)
          .then((confirmed) => {
            if (!confirmed || disposed) return;
            return api.destroyCurrentWindow();
          })
          .catch((error) => {
            if (!disposed) {
              notify(
                appErrorMessage(
                  error,
                  useProject.getState().settings.language,
                ),
              );
            }
          })
          .finally(() => {
            asking = false;
          });
      })
      .then((unlisten) => {
        if (disposed) unlisten();
        else stop = unlisten;
      })
      .catch((error) => {
        if (!disposed) {
          notify(
            appErrorMessage(error, useProject.getState().settings.language),
          );
        }
      });

    return () => {
      disposed = true;
      stop?.();
    };
  }, [notify]);

  useEffect(() => {
    const query = window.matchMedia("(prefers-color-scheme: light)");
    const apply = () => {
      document.documentElement.dataset.theme =
        appSettings.theme === "system"
          ? query.matches
            ? "light"
            : "dark"
          : appSettings.theme;
      document.documentElement.lang = appSettings.language;
    };

    apply();
    query.addEventListener("change", apply);
    return () => query.removeEventListener("change", apply);
  }, [appSettings.theme, appSettings.language]);

  useEffect(() => {
    const key = (event: KeyboardEvent) => {
      if (document.querySelector("dialog[open]")) return;

      if (event.ctrlKey && event.key.toLowerCase() === "o") {
        event.preventDefault();
        open();
      }

      if (event.ctrlKey && event.shiftKey && event.key.toLowerCase() === "e") {
        event.preventDefault();
        if (useProject.getState().activeId) setExporting(true);
      }
    };

    window.addEventListener("keydown", key);
    return () => {
      window.removeEventListener("keydown", key);
    };
  }, [open]);

  return (
    <main className={`app ${panel ? "" : "panel-collapsed"}`}>
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
        <button onClick={open} disabled={!ready || importing || batchBusy}>
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
          disabled={!active || batchBusy}
          onClick={() => setExporting(true)}
        >
          {t("Ekspor…", "Export…")} ↗
        </button>
      </header>

      {notice && (
        <div className="notice" role="alert">
          <span>{notice}</span>
          <button
            aria-label={t("Tutup pemberitahuan", "Dismiss notification")}
            onClick={() => notify("")}
          >
            ×
          </button>
        </div>
      )}

      <div className="workspace">
        {panel && <ParamPanel />}
        <div className="work-area">
          <CompareCanvas file={active} onOpen={open} />
          {files.length > 0 && <BatchQueue />}
        </div>
      </div>

      <footer className="status-bar">
        <span className="status-dot" />
        {!ready
          ? t("Memuat pengaturan…", "Loading settings…")
          : importing
            ? t("Membaca gambar…", "Reading images…")
            : t("Lokal · Engine aktif", "Local · Engine active")}
        <span className="status-meta">
          {active
            ? `${active.width} × ${active.height} px · ${(
                active.bytes / 1024
              ).toFixed(1)} KB`
            : "PNG / JPG / WEBP / BMP"}
        </span>
        <span>
          {t(
            "Ekspor: resolusi sumber penuh",
            "Export: full source resolution",
          )}
        </span>
      </footer>

      {dragging && (
        <div className="drop-overlay">
          {batchBusy
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
