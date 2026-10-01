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
    fileId: string;
    signature: string;
  } | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const signature = JSON.stringify(params);
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
    if (!fileId || (!auto && !requested)) return;

    const requestId = crypto.randomUUID();
    let disposed = false;
    let submitted = false;

    const timer = window.setTimeout(() => {
      submitted = true;
      setBusy(true);
      setError("");

      void api
        .generatePreview({
          fileId,
          params: JSON.parse(signature),
          maxSide,
          requestId,
        })
        .then((response) => {
          if (disposed) return;
          const url = URL.createObjectURL(
            new Blob([response.svg], { type: "image/svg+xml" }),
          );
          setResult({ url, fileId, signature });
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
      setBusy(false);
      if (submitted) {
        void api.cancelPreview(requestId).catch(() => {
          // Cleanup cancellation is best-effort; stale results are discarded locally.
        });
      }
    };
  }, [fileId, signature, auto, maxSide, manual, language]);

  return {
    url: result?.fileId === fileId ? result.url : "",
    busy,
    error,
    stale: result?.signature !== signature,
  };
}
