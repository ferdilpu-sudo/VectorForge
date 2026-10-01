import { useEffect, useRef, useState } from "react";
import { useProject } from "../../stores/project.store";
import { api } from "../../services/api";
export function usePreview(manual: number) {
  const fileId = useProject((s) => s.activeId);
  const params = useProject((s) => s.params);
  const auto = useProject((s) => s.settings.autoPreview);
  const maxSide = useProject((s) => s.settings.previewMaxSide);
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
    const controller = new AbortController();
    const timer = setTimeout(() => {
      setBusy(true);
      setError("");
      api
        .generatePreview(
          {
            fileId,
            params: JSON.parse(signature),
            maxSide,
            requestId: crypto.randomUUID(),
          },
          controller.signal,
        )
        .then((r) => {
          if (controller.signal.aborted) return;
          const url = URL.createObjectURL(
            new Blob([r.svg], { type: "image/svg+xml" }),
          );
          setResult({ url, fileId, signature });
        })
        .catch((e) => {
          if (!controller.signal.aborted)
            setError(e instanceof Error ? e.message : "Preview error");
        })
        .finally(() => {
          if (!controller.signal.aborted) setBusy(false);
        });
    }, 300);
    return () => {
      clearTimeout(timer);
      controller.abort();
      setBusy(false);
    };
  }, [fileId, signature, auto, maxSide, manual]);
  return {
    url: result?.fileId === fileId ? result.url : "",
    busy,
    error,
    stale: result?.signature !== signature,
  };
}
