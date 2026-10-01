import { useEffect, useRef, useState } from "react";
import { Modal } from "../../shared/Modal";
import { useLanguage } from "../../shared/useLanguage";
import type { SourceFile, ExportFormat } from "../../types/project";
export function ExportDialog({
  file,
  onClose,
}: {
  file: SourceFile;
  onClose: () => void;
}) {
  const t = useLanguage();
  const [format, setFormat] = useState<ExportFormat>("svg");
  const [status, setStatus] = useState("idle");
  const timer = useRef<ReturnType<typeof setTimeout>>();
  useEffect(() => () => clearTimeout(timer.current), []);
  return (
    <Modal title={t("Ekspor gambar", "Export image")} onClose={onClose}>
      <p className="muted">
        {file.name} · {file.width} × {file.height}
      </p>
      <div className="format-options">
        {(["svg", "pdf", "eps"] as const).map((f) => (
          <button
            key={f}
            disabled={status === "busy"}
            aria-pressed={format === f}
            onClick={() => {
              setFormat(f);
              setStatus("idle");
            }}
          >
            {f.toUpperCase()}
          </button>
        ))}
      </div>
      {format === "eps" && (
        <p className="warning">
          {t(
            "EPS menggunakan latar putih untuk transparansi.",
            "EPS flattens transparency onto white.",
          )}
        </p>
      )}
      <p>
        {t(
          "Ekspor produksi akan memakai resolusi penuh.",
          "Production export will use full resolution.",
        )}
      </p>
      <p className="demo-inline">
        {t(
          "Mode Demo: dialog Save As dan penyimpanan file belum terhubung.",
          "Demo mode: Save As and file export are not connected.",
        )}
      </p>
      <button
        className="primary wide"
        disabled={status === "busy"}
        onClick={() => {
          setStatus("busy");
          timer.current = setTimeout(() => setStatus("done"), 700);
        }}
      >
        {status === "busy"
          ? t("Simulasi proses…", "Simulating…")
          : t("Simulasikan ekspor", "Simulate export")}
      </button>
      <p role="status">
        {status === "done"
          ? t(
              "Simulasi selesai. Tidak ada file yang disimpan.",
              "Simulation complete. No file was saved.",
            )
          : ""}
      </p>
    </Modal>
  );
}
