import type { AppSettings } from "../types/project";

const detectedWorkers =
  typeof navigator === "undefined"
    ? 1
    : Math.min(4, Math.max(1, navigator.hardwareConcurrency || 1));

export const defaultSettings: AppSettings = {
  language: "id",
  theme: "dark",
  workerCount: detectedWorkers,
  autoPreview: true,
  previewMaxSide: 1024,
};
