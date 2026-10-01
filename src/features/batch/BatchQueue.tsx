import { useState } from "react";
import { useProject } from "../../stores/project.store";
import { useLanguage } from "../../shared/useLanguage";
import { useDemoBatch } from "./useDemoBatch";
import type { ExportFormat } from "../../types/project";
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
  const [height, setHeight] = useState(210);
  const [folder, setFolder] = useState("");
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
    <section className="batch" style={{ height: open ? height : undefined }}>
      <div className="batch-heading">
        <button
          className="text-button"
          aria-expanded={open}
          onClick={() => setOpen(!open)}
        >
          {open ? "⌄" : "›"} {t("Antrean gambar", "Image queue")}{" "}
          <span className="badge">{files.length}</span>
        </button>
        <span className="muted">
          {batch.busy
            ? t("Simulasi sedang berjalan", "Simulation running")
            : t(
                "Satu pengaturan untuk semua gambar",
                "One configuration for all images",
              )}
        </span>
      </div>
      {open && (
        <>
          <label className="batch-size">
            {t("Tinggi antrean", "Queue height")}
            <input
              aria-label={t("Tinggi antrean", "Queue height")}
              type="range"
              min="144"
              max="240"
              value={height}
              onChange={(e) => setHeight(Number(e.target.value))}
            />
          </label>
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
                      disabled={batch.busy}
                      onClick={() => batch.retry(job.id)}
                    >
                      {t("Coba lagi", "Retry")}
                    </button>
                  ) : (
                    <button
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
          <div className="batch-options">
            <div className="format-checks">
              {(["svg", "pdf", "eps"] as const).map((f) => (
                <label key={f}>
                  <input
                    type="checkbox"
                    disabled={batch.busy}
                    checked={formats.includes(f)}
                    onChange={(e) =>
                      setFormats(
                        e.target.checked
                          ? [...formats, f]
                          : formats.filter((x) => x !== f),
                      )
                    }
                  />
                  {f.toUpperCase()}
                </label>
              ))}
            </div>
            <input
              className="folder-input"
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
              onChange={(e) => setFolder(e.target.value)}
            />
            <label className="check">
              <input
                type="checkbox"
                disabled={batch.busy}
                checked={fail}
                onChange={(e) => setFail(e.target.checked)}
              />
              {t("Simulasi PDF gagal", "Simulate PDF failure")}
            </label>
            {batch.busy ? (
              <button onClick={batch.cancel} disabled={batch.cancelling}>
                {batch.cancelling
                  ? t("Membatalkan…", "Cancelling…")
                  : t("Batalkan", "Cancel")}
              </button>
            ) : (
              <button
                className="primary"
                disabled={!formats.length || !folder.trim() || importing}
                onClick={() => batch.start(formats, fail)}
              >
                {t("Simulasikan batch", "Simulate batch")}
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
        </>
      )}
    </section>
  );
}
