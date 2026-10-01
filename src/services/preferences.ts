import {
  builtIns,
  validParams,
  validPresetName,
  type Preset,
} from "../types/params";
import type { AppSettings } from "../types/project";
const key = "vectorforge-demo-preferences-v1";
export const defaultSettings: AppSettings = {
  language: "id",
  theme: "dark",
  workerCount: 2,
  autoPreview: true,
  previewMaxSide: 1024,
};
export function readPreferences(): {
  settings: AppSettings;
  presets: Preset[];
  warning: string;
} {
  try {
    const raw = localStorage.getItem(key);
    if (!raw) return { settings: defaultSettings, presets: [], warning: "" };
    const d = JSON.parse(raw);
    if (d.version !== 1)
      throw new Error("Versi pengaturan demo tidak didukung.");
    const s = d.settings;
    if (
      !s ||
      !["id", "en"].includes(s.language) ||
      !["dark", "light", "system"].includes(s.theme) ||
      !Number.isInteger(s.workerCount) ||
      s.workerCount < 1 ||
      s.workerCount > 4 ||
      typeof s.autoPreview !== "boolean" ||
      !Number.isInteger(s.previewMaxSide) ||
      s.previewMaxSide < 512 ||
      s.previewMaxSide > 2048 ||
      !Array.isArray(d.presets)
    )
      throw new Error("Pengaturan demo tidak valid.");
    const presets: Preset[] = [];
    for (const v of d.presets) {
      if (
        !v ||
        typeof v.id !== "string" ||
        typeof v.name !== "string" ||
        typeof v.createdAt !== "string" ||
        v.builtIn !== false ||
        !v.params ||
        !validParams(v.params) ||
        !validPresetName(v.name, [...builtIns, ...presets])
      )
        throw new Error("Preset demo tidak valid.");
      presets.push(v);
    }
    return { settings: s, presets, warning: "" };
  } catch (e) {
    return {
      settings: defaultSettings,
      presets: [],
      warning: e instanceof Error ? e.message : "Pengaturan demo gagal dibaca.",
    };
  }
}
export function writePreferences(
  settings: AppSettings,
  presets: Preset[],
): string {
  try {
    localStorage.setItem(
      key,
      JSON.stringify({ version: 1, settings, presets }),
    );
    return "";
  } catch {
    return "Pengaturan demo tidak dapat disimpan di browser.";
  }
}
