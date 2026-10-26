import { useEffect, useRef, useState } from "react";

import { api } from "../../services/api";
import { appErrorMessage } from "../../services/errors";
import { useProject } from "../../stores/project.store";

export function usePreview(manual: number) {
  const fileId = useProject((state) => state.activeId);
  const params = useProject((state) => state.params);
  const auto = useProject((state) => state.settings.autoPreview);
  const maxSide = useProject((state) => state.settings.previewMaxSide);
  const language = useProject((state) => state.settings.language);
  const [result, setResult] = useState<{
    url: string;
    signature: string;
    requestId: string;
  } | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  // Invalidate on every property that can change the generated SVG.
  const signature = JSON.stringify({ fileId, params, maxSide });
  const lastManual = useRef(manual);

  useEffect(
    () => () => {
      if (result?.url) URL.revokeObjectURL(result.url);
    },
    [result?.url],
  );

  useEffect(() => {
    const requested = lastManual.current !== manual;
    lastManual.current = manual;
    if (!fileId) {
      setBusy(false);
      setError("");
      return;
    }
    if (!auto && !requested) {
      setBusy(false);
      return;
    }

    const requestId = crypto.randomUUID();
    let disposed = false;
    let submitted = false;
    setBusy(true);
    setError("");

    const timer = window.setTimeout(() => {
      submitted = true;
      void api
        .generatePreview({
          fileId,
          params: JSON.parse(signature).params,
          maxSide,
          requestId,
        })
        .then((response) => {
          if (disposed) return;
          if (response.requestId !== requestId || response.fileId !== fileId) {
            setError(
              language === "id"
                ? "Respons preview tidak cocok dengan permintaan."
                : "Preview response does not match the request.",
            );
            return;
          }
          const url = URL.createObjectURL(
            new Blob([response.svg], { type: "image/svg+xml" }),
          );
          setResult({ url, signature, requestId });
        })
        .catch((reason) => {
          if (!disposed) setError(appErrorMessage(reason, language));
        })
        .finally(() => {
          if (!disposed) setBusy(false);
        });
    }, 300);

    return () => {
      disposed = true;
      window.clearTimeout(timer);
      if (submitted) {
        void api.cancelPreview(requestId).catch(() => {
          // Cancellation is best-effort. Disposed responses are always ignored.
        });
      }
    };
  }, [fileId, signature, auto, maxSide, manual, language]);

  const current = Boolean(fileId && result?.signature === signature);
  return {
    url: current ? (result?.url ?? "") : "",
    busy,
    error,
    stale: Boolean(fileId && !current),
    needsRefresh: Boolean(fileId && !auto && !current),
  };
}
