import { useState } from "react";

import { api } from "../../services/api";
import { appErrorMessage, hasAppErrorCode } from "../../services/errors";
import { Modal } from "../../shared/Modal";
import { useLanguage } from "../../shared/useLanguage";
import { useProject } from "../../stores/project.store";
import type {
  Destination,
  ExportFormat,
  ExportResult,
  SourceFile,
} from "../../types/project";

type ExportStatus = "idle" | "busy" | "confirm-large" | "done";

function suggestedName(name: string, format: ExportFormat): string {
  const dot = name.lastIndexOf(".");
  const stem = dot > 0 ? name.slice(0, dot) : name;
  return `${stem}.${format}`;
}

export function ExportDialog({
  file,
  onClose,
}: {
  file: SourceFile;
  onClose: () => void;
}) {
  const t = useLanguage();
  const params = useProject((state) => state.params);
  const language = useProject((state) => state.settings.language);
  const [format, setFormat] = useState<ExportFormat>("svg");
  const [status, setStatus] = useState<ExportStatus>("idle");
  const [destination, setDestination] = useState<Destination | null>(null);
  const [result, setResult] = useState<ExportResult | null>(null);
  const [error, setError] = useState("");

  const resetTarget = (nextFormat: ExportFormat) => {
    setFormat(nextFormat);
    setStatus("idle");
    setDestination(null);
    setResult(null);
    setError("");
  };

  const runExport = async (allowLargeOutput: boolean) => {
    setError("");
    setStatus("busy");

    try {
      let target = destination;
      if (!target) {
        target = await api.chooseDestination({
          kind: "file",
          format,
          suggestedName: suggestedName(file.name, format),
        });
        if (!target) {
          setStatus("idle");
          return;
        }
        setDestination(target);
      }

      const exported = await api.exportFile({
        fileId: file.id,
        params: { ...params },
        format,
        destinationId: target.id,
        allowLargeOutput,
      });
      setResult(exported);
      setStatus("done");
    } catch (reason) {
      if (
        format === "svg" &&
        !allowLargeOutput &&
        hasAppErrorCode(reason, "OUTPUT_TOO_LARGE")
      ) {
        setStatus("confirm-large");
        return;
      }

      setError(appErrorMessage(reason, language));
      setStatus("idle");
    }
  };

  return (
    <Modal title={t("Ekspor gambar", "Export image")} onClose={onClose}>
      <p className="muted">
        {file.name} · {file.width} × {file.height}
      </p>
      <div className="format-options">
        {(["svg", "pdf", "eps"] as const).map((candidate) => (
          <button
            key={candidate}
            disabled={status === "busy"}
            aria-pressed={format === candidate}
            onClick={() => resetTarget(candidate)}
          >
            {candidate.toUpperCase()}
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
          "Ekspor memakai resolusi sumber penuh.",
          "Export uses the full source resolution.",
        )}
      </p>

      {status === "confirm-large" ? (
        <div className="warning">
          <p>
            {t(
              "SVG melebihi 50 MiB. Lanjutkan ekspor file besar?",
              "SVG exceeds 50 MiB. Continue with the large export?",
            )}
          </p>
          <button onClick={() => setStatus("idle")}>
            {t("Batal", "Cancel")}
          </button>{" "}
          <button
            className="primary"
            onClick={() => void runExport(true)}
          >
            {t("Lanjutkan", "Continue")}
          </button>
        </div>
      ) : (
        <button
          className="primary wide"
          disabled={status === "busy"}
          onClick={() => void runExport(false)}
        >
          {status === "busy"
            ? t("Mengekspor…", "Exporting…")
            : t("Pilih lokasi & ekspor", "Choose location & export")}
        </button>
      )}

      {error && (
        <p role="alert" className="danger">
          {error}
        </p>
      )}

      <p role="status">
        {status === "done" && result
          ? t(
              `Tersimpan: ${result.outPath}`,
              `Saved: ${result.outPath}`,
            )
          : ""}
      </p>

      {status === "done" && result && (
        <button
          className="wide"
          onClick={() => {
            void api.openOutputFolder(result.outputId).catch((reason) => {
              setError(appErrorMessage(reason, language));
            });
          }}
        >
          {t("Buka folder output", "Open output folder")}
        </button>
      )}
    </Modal>
  );
}
