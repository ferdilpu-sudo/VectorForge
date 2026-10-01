import { useRef, useState } from "react";

import { api } from "../../services/api";
import { appErrorMessage } from "../../services/errors";
import { useLanguage } from "../../shared/useLanguage";
import { useProject } from "../../stores/project.store";
import type { Destination, ExportFormat } from "../../types/project";
import { useBatch } from "./useBatch";

const MIN_QUEUE_HEIGHT = 144;
const MAX_QUEUE_HEIGHT = 240;

export function BatchQueue() {
  const files = useProject((state) => state.files);
  const active = useProject((state) => state.activeId);
  const select = useProject((state) => state.select);
  const remove = useProject((state) => state.remove);
  const importing = useProject((state) => state.importing);
  const language = useProject((state) => state.settings.language);
  const notify = useProject((state) => state.notify);
  const t = useLanguage();
  const batch = useBatch();
  const [formats, setFormats] = useState<ExportFormat[]>(["svg"]);
  const [open, setOpen] = useState(true);
  const [height, setHeight] = useState(178);
  const [destination, setDestination] = useState<Destination | null>(null);
  const resizeStart = useRef<{ y: number; height: number } | null>(null);

  const clampHeight = (value: number) =>
    Math.max(MIN_QUEUE_HEIGHT, Math.min(MAX_QUEUE_HEIGHT, value));

  const chooseFolder = async () => {
    if (batch.busy) return;
    try {
      const selected = await api.chooseDestination({ kind: "directory" });
      if (selected) setDestination(selected);
    } catch (error) {
      notify(appErrorMessage(error, language));
    }
  };

  const completed = batch.items.filter((item) =>
    ["done", "partial", "failed", "cancelled"].includes(item.status),
  ).length;

  const labels: Record<string, string> = {
    queued: t("Menunggu", "Queued"),
    processing: t("Proses", "Processing"),
    done: t("Selesai", "Done"),
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
          aria-valuemin={MIN_QUEUE_HEIGHT}
          aria-valuemax={MAX_QUEUE_HEIGHT}
          aria-valuenow={height}
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
            ? t("Batch berjalan", "Batch running")
            : t(
                "Satu pengaturan untuk semua gambar",
                "One configuration for all images",
              )}
        </span>
      </div>

      {open && (
        <div className="batch-body">
          <div className="file-strip">
            {files.map((file) => {
              const job = batch.items.find((item) => item.fileId === file.id);
              const detail =
                job?.status === "processing"
                  ? t(job.stage, job.stage)
                  : job
                    ? labels[job.status]
                    : `${file.width} × ${file.height}`;

              return (
                <div
                  key={file.id}
                  className={`file-card ${active === file.id ? "selected" : ""}`}
                >
                  <button
                    className="file-select"
                    onClick={() => select(file.id)}
                  >
                    <img src={file.previewUrl} alt="" />
                    <span>
                      <strong title={file.name}>{file.name}</strong>
                      <small
                        className={
                          job?.status === "failed" || job?.status === "partial"
                            ? "danger"
                            : ""
                        }
                      >
                        {detail}
                      </small>
                    </span>
                  </button>
                  {job &&
                  (job.status === "failed" || job.status === "partial") ? (
                    <button
                      className="file-card-action"
                      disabled={batch.busy}
                      onClick={() => void batch.retry(job.id)}
                    >
                      {t("Coba lagi", "Retry")}
                    </button>
                  ) : (
                    <button
                      className="file-card-remove"
                      aria-label={`${t("Hapus dari antrean", "Remove from queue")} ${file.name}`}
                      disabled={batch.busy || importing}
                      onClick={() => void remove(file.id)}
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
                aria-label={t("Folder tujuan", "Destination folder")}
                placeholder={t("Pilih folder tujuan", "Choose destination folder")}
                value={destination?.displayPath ?? ""}
                disabled={batch.busy}
                readOnly
                onClick={() => void chooseFolder()}
                onKeyDown={(event) => {
                  if (event.key === "Enter" || event.key === " ") {
                    event.preventDefault();
                    void chooseFolder();
                  }
                }}
              />
            </label>

            {batch.busy ? (
              <button
                className="batch-run-button"
                onClick={() => void batch.cancel()}
                disabled={batch.cancelling}
              >
                {batch.cancelling
                  ? t("Membatalkan…", "Cancelling…")
                  : t("Batalkan", "Cancel")}
              </button>
            ) : (
              <button
                className="primary batch-run-button"
                disabled={!formats.length || !destination || importing}
                onClick={() => {
                  if (destination) void batch.start(formats, destination);
                }}
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
                {batch.busy
                  ? t("Memproses output", "Processing outputs")
                  : t("Batch selesai", "Batch finished")}
              </span>
            </div>
          )}
        </div>
      )}
    </section>
  );
}
