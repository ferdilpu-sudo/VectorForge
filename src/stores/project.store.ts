import { create } from "zustand";
import {
  defaults,
  builtIns,
  validPresetName,
  type TraceParams,
  type Preset,
} from "../types/params";
import type { AppSettings, SourceFile } from "../types/project";
import { readPreferences, writePreferences } from "../services/preferences";
import { api } from "../services/api";
import { importFileErrorMessage } from "../services/import-files";
const initial = readPreferences();
interface State {
  files: SourceFile[];
  activeId: string;
  params: TraceParams;
  presetId: string;
  presets: Preset[];
  settings: AppSettings;
  notice: string;
  importing: boolean;
  batchBusy: boolean;
  select: (id: string) => void;
  importFiles: (files: File[]) => Promise<void>;
  remove: (id: string) => void;
  setParams: (p: TraceParams) => void;
  choosePreset: (id: string) => void;
  savePreset: (name: string) => boolean;
  deletePreset: (id: string) => void;
  setSettings: (s: AppSettings) => void;
  notify: (text: string) => void;
}
export const useProject = create<State>((set, get) => ({
  files: [],
  activeId: "",
  params: { ...defaults },
  presetId: builtIns[0].id,
  presets: initial.presets,
  settings: initial.settings,
  notice: initial.warning,
  importing: false,
  batchBusy: false,
  select: (activeId) => set({ activeId }),
  notify: (notice) => set({ notice }),
  setParams: (params) => set({ params }),
  importFiles: async (files) => {
    if (get().importing || get().batchBusy) return;
    set({ importing: true });
    const errors: string[] = [];
    try {
      for (const f of files) {
        try {
          const item = await api.readBrowserFile(f);
          if (get().files.some((x) => x.fingerprint === item.fingerprint)) {
            URL.revokeObjectURL(item.previewUrl);
            continue;
          }
          set((s) => ({
            files: [...s.files, item],
            activeId: s.activeId || item.id,
          }));
        } catch (e) {
          errors.push(
            `${f.name}: ${importFileErrorMessage(e, get().settings.language)}`,
          );
        }
      }
    } finally {
      set({ importing: false, notice: errors.join(" · ") });
    }
  },
  remove: (id) => {
    if (get().batchBusy || get().importing) return;
    const source = get().files.find((f) => f.id === id);
    if (source) URL.revokeObjectURL(source.previewUrl);
    set((s) => {
      const files = s.files.filter((f) => f.id !== id);
      return {
        files,
        activeId: s.activeId === id ? (files[0]?.id ?? "") : s.activeId,
      };
    });
  },
  choosePreset: (id) => {
    const preset = [...builtIns, ...get().presets].find((p) => p.id === id);
    if (preset) set({ presetId: id, params: { ...preset.params } });
  },
  savePreset: (name) => {
    if (!validPresetName(name, [...builtIns, ...get().presets])) return false;
    const preset: Preset = {
      id: crypto.randomUUID(),
      name: name.trim(),
      params: { ...get().params },
      builtIn: false,
      createdAt: new Date().toISOString(),
    };
    const presets = [...get().presets, preset];
    const warning = writePreferences(get().settings, presets);
    set({ presets, presetId: preset.id, notice: warning });
    return true;
  },
  deletePreset: (id) => {
    const presets = get().presets.filter((p) => p.id !== id);
    set({
      presets,
      notice: writePreferences(get().settings, presets),
    });
    if (get().presetId === id) get().choosePreset(builtIns[0].id);
  },
  setSettings: (settings) => {
    set({
      settings,
      notice: writePreferences(settings, get().presets),
    });
  },
}));
