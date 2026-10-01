import type { PreviewRequest, PreviewResult } from "../types/project";
export async function generatePreview(
  request: PreviewRequest,
  signal: AbortSignal,
): Promise<PreviewResult> {
  await new Promise<void>((resolve, reject) => {
    const abort = () => {
      clearTimeout(timer);
      reject(new DOMException("Cancelled", "AbortError"));
    };
    const timer = setTimeout(() => {
      signal.removeEventListener("abort", abort);
      resolve();
    }, 450);
    if (signal.aborted) abort();
    else signal.addEventListener("abort", abort, { once: true });
  });
  // Deliberately a labelled sample, never a traced representation of the imported image.
  const colors = ["#60a5fa", "#2563eb", "#142b54"];
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="800" height="600" viewBox="0 0 800 600"><rect width="800" height="600" fill="#182337"/><circle cx="400" cy="245" r="140" fill="${colors[0]}"/><path d="M150 470L350 160L510 470Z" fill="${colors[1]}"/><path d="M345 470L520 260L660 470Z" fill="${colors[2]}"/><text x="400" y="548" text-anchor="middle" fill="white" font-family="sans-serif" font-size="22">DEMO — NOT A TRACE</text></svg>`;
  return {
    fileId: request.fileId,
    requestId: request.requestId,
    svg,
    elapsedMs: 450,
  };
}
