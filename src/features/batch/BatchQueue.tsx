import { useRef, useState } from "react";
import { useProject } from "../../stores/project.store";
import { useLanguage } from "../../shared/useLanguage";
import { useDemoBatch } from "./useDemoBatch";
import type { ExportFormat } from "../../types/project";

const MIN_QUEUE_HEIGHT = 144;
const MAX_QUEUE_HEIGHT = 240;

export function BatchQueue() {
  const files = useProject((s) => s.files);
  const active = useProject((s) => s.activeId);
  const select = useProject((s) => s.select);
  const remove = useProject((s) => s.remove);
  const importing = useProject((s) => s.importing);
  const t = useLanguage();
  const batch = useDemoBatch();
  const [formats, setFormats] = useState<ExportFormat[]>(["svg"]);
  const [fail, setFail] = useState(false);
  const [open, setOpen] = useState(true);
  const [height, setHeight] = useState(178);
  const [folder, setFolder] = useState("");
  const resizeStart = useRef<{ y: number; height: number } | null>(null);

  const clampHeight = (value: number) =>
    Math.max(MIN_QUEUE_HEIGHT, Math.min(MAX_QUEUE_HEIGHT, value));

  const completed = batch.items.filter((i) =>
    ["done", "partial", "failed", "cancelled"].includes(i.status),
  ).length;

  const labels: Record<string, string> = {
    queued: t("Menunggu", "Queued"),
    processing: t("Proses", "Processing"),
    done: t("Selesai (demo)", "Done (demo)"),
    partial: t("Sebagian gagal", "Partial failure"),
    failed: t("Gagal", "Failed"),
    cancelled: t("Dibatalkan", "Cancelled"),
  };

  return (
    <section
      className={`batch ${open ? "batch-open" : "batch-collapsed"}`}
      style={{ height: open ? height : undefined }}
    >
      {open && (
        <div
          className="batch-resize-handle"
          role="separator"
          aria-orientation="horizontal"
          aria-label={t("Ubah tinggi antrean", "Resize queue")}
          tabIndex={0}
          onKeyDown={(event) => {
            if (event.key === "ArrowUp") {
              event.preventDefault();
              setHeight((value) => clampHeight(value + 12));
            }
            if (event.key === "ArrowDown") {
              event.preventDefault();
              setHeight((value) => clampHeight(value - 12));
            }
          }}
          onPointerDown={(event) => {
            event.currentTarget.setPointerCapture(event.pointerId);
            resizeStart.current = { y: event.clientY, height };
          }}
          onPointerMove={(event) => {
            if (!resizeStart.current) return;
            const delta = resizeStart.current.y - event.clientY;
            setHeight(clampHeight(resizeStart.current.height + delta));
          }}
          onPointerUp={() => {
            resizeStart.current = null;
          }}
          onPointerCancel={() => {
            resizeStart.current = null;
          }}
        >
          <span />
        </div>
      )}

      <div className="batch-heading">
        <button
          className="batch-toggle"
          aria-expanded={open}
          onClick={() => setOpen(!open)}
        >
          <span className="batch-chevron" aria-hidden="true">
            {open ? "⌄" : "›"}
          </span>
          <strong>{t("Antrean", "Queue")}</strong>
          <span className="badge">{files.length}</span>
        </button>
        <span className="batch-heading-meta">
          {batch.busy
            ? t("Simulasi berjalan", "Simulation running")
            : t(
                "Satu pengaturan untuk semua gambar",
                "One configuration for all images",
              )}
        </span>
      </div>

      {open && (
        <div className="batch-body">
          <div className="file-strip">
            {files.map((f) => {
              const job = batch.items.find((i) => i.fileId === f.id);
              return (
                <div
                  key={f.id}
                  className={`file-card ${active === f.id ? "selected" : ""}`}
                >
                  <button className="file-select" onClick={() => select(f.id)}>
                    <img src={f.previewUrl} alt="" />
                    <span>
                      <strong title={f.name}>{f.name}</strong>
                      <small
                        className={
                          job?.status === "failed" || job?.status === "partial"
                            ? "danger"
                            : ""
                        }
                      >
                        {job ? labels[job.status] : `${f.width} × ${f.height}`}
                      </small>
                    </span>
                  </button>
                  {job &&
                  (job.status === "failed" || job.status === "partial") ? (
                    <button
                      className="file-card-action"
                      disabled={batch.busy}
                      onClick={() => batch.retry(job.id)}
                    >
                      {t("Coba lagi", "Retry")}
                    </button>
                  ) : (
                    <button
                      className="file-card-remove"
                      aria-label={`${t("Hapus dari antrean", "Remove from queue")} ${f.name}`}
                      disabled={batch.busy || importing}
                      onClick={() => remove(f.id)}
                    >
                      ×
                    </button>
                  )}
                </div>
              );
            })}
          </div>

          <div className="batch-command-row">
            <div
              className="format-group"
              role="group"
              aria-label={t("Format keluaran", "Output formats")}
            >
              {(["svg", "pdf", "eps"] as const).map((format) => {
                const selected = formats.includes(format);
                return (
                  <label
                    key={format}
                    className={`format-chip ${selected ? "selected" : ""}`}
                  >
                    <input
                      type="checkbox"
                      disabled={batch.busy}
                      checked={selected}
                      onChange={(event) =>
                        setFormats(
                          event.target.checked
                            ? [...formats, format]
                            : formats.filter((item) => item !== format),
                        )
                      }
                    />
                    {format.toUpperCase()}
                  </label>
                );
              })}
            </div>

            <label className="folder-field">
              <span aria-hidden="true">▰</span>
              <input
                aria-label={t(
                  "Folder tujuan simulasi",
                  "Simulated destination folder",
                )}
                placeholder={t(
                  "Folder tujuan (simulasi)",
                  "Destination folder (simulation)",
                )}
                value={folder}
                disabled={batch.busy}
                onChange={(event) => setFolder(event.target.value)}
              />
            </label>

            <details className="batch-test">
              <summary>{t("Uji demo", "Demo test")}</summary>
              <label className="check">
                <input
                  type="checkbox"
                  disabled={batch.busy}
                  checked={fail}
                  onChange={(event) => setFail(event.target.checked)}
                />
                {t("Simulasi PDF gagal", "Simulate PDF failure")}
              </label>
            </details>

            {batch.busy ? (
              <button
                className="batch-run-button"
                onClick={batch.cancel}
                disabled={batch.cancelling}
              >
                {batch.cancelling
                  ? t("Membatalkan…", "Cancelling…")
                  : t("Batalkan", "Cancel")}
              </button>
            ) : (
              <button
                className="primary batch-run-button"
                disabled={!formats.length || !folder.trim() || importing}
                onClick={() => batch.start(formats, fail)}
              >
                {t("Mulai batch", "Start batch")}
              </button>
            )}
          </div>

          {batch.items.length > 0 && (
            <div className="batch-progress">
              <progress max={batch.items.length} value={completed} />
              <span role="status">
                {completed}/{batch.items.length} ·{" "}
                {t("Tidak ada file disimpan", "No files saved")}
              </span>
            </div>
          )}
        </div>
      )}
    </section>
  );
}
