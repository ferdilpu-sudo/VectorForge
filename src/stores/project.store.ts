import { create } from "zustand";

import { api } from "../services/api";
import { appErrorMessage } from "../services/errors";
import { defaultSettings } from "../services/preferences";
import {
  builtIns,
  defaults,
  validPresetName,
  type Preset,
  type TraceParams,
} from "../types/params";
import type { AppSettings, SourceFile } from "../types/project";

let settingsQueue = Promise.resolve();
let settingsSequence = 0;

interface State {
  files: SourceFile[];
  activeId: string;
  params: TraceParams;
  presetId: string;
  presets: Preset[];
  settings: AppSettings;
  notice: string;
  ready: boolean;
  importing: boolean;
  batchBusy: boolean;
  initialize: () => Promise<void>;
  select: (id: string) => void;
  openFiles: () => Promise<void>;
  importPaths: (paths: string[]) => Promise<void>;
  remove: (id: string) => Promise<void>;
  setParams: (params: TraceParams) => void;
  choosePreset: (id: string) => void;
  savePreset: (name: string) => Promise<boolean>;
  deletePreset: (id: string) => Promise<void>;
  setSettings: (settings: AppSettings) => void;
  notify: (text: string) => void;
}

function userPresets(presets: Preset[]): Preset[] {
  return presets.filter((preset) => !preset.builtIn);
}

function chooseInitialPreset(settings: AppSettings, presets: Preset[]): Preset {
  const all = [...builtIns, ...presets];
  return (
    all.find((preset) => preset.id === settings.lastPresetId) ??
    builtIns[0]
  );
}

export const useProject = create<State>((set, get) => ({
  files: [],
  activeId: "",
  params: { ...defaults },
  presetId: builtIns[0].id,
  presets: [],
  settings: { ...defaultSettings },
  notice: "",
  ready: false,
  importing: false,
  batchBusy: false,

  initialize: async () => {
    if (get().ready) return;

    let settings = get().settings;
    let presets = get().presets;
    const errors: string[] = [];

    try {
      settings = await api.getSettings();
    } catch (error) {
      errors.push(appErrorMessage(error, settings.language));
    }

    try {
      presets = userPresets(await api.listPresets());
    } catch (error) {
      errors.push(appErrorMessage(error, settings.language));
    }

    const selected = chooseInitialPreset(settings, presets);
    set({
      settings,
      presets,
      presetId: selected.id,
      params: { ...selected.params },
      notice: errors.join(" · "),
      ready: true,
    });
  },

  select: (activeId) => set({ activeId }),
  notify: (notice) => set({ notice }),
  setParams: (params) => set({ params }),

  openFiles: async () => {
    if (get().importing || get().batchBusy) return;
    try {
      const paths = await api.pickSourcePaths();
      if (paths.length) await get().importPaths(paths);
    } catch (error) {
      set({ notice: appErrorMessage(error, get().settings.language) });
    }
  },

  importPaths: async (paths) => {
    if (get().importing || get().batchBusy || paths.length === 0) return;
    set({ importing: true });
    try {
      const result = await api.importFiles(paths);
      const fingerprints = new Set(get().files.map((file) => file.fingerprint));
      const accepted: SourceFile[] = [];
      const duplicateIds: string[] = [];

      for (const file of result.files) {
        if (fingerprints.has(file.fingerprint)) {
          duplicateIds.push(file.id);
          continue;
        }
        fingerprints.add(file.fingerprint);
        accepted.push(file);
      }

      if (duplicateIds.length) {
        await api.releaseFiles(duplicateIds);
      }

      set((state) => ({
        files: [...state.files, ...accepted],
        activeId: state.activeId || accepted[0]?.id || "",
        notice: result.rejected
          .map(
            (rejection) =>
              `${rejection.name}: ${appErrorMessage(
                rejection.error,
                state.settings.language,
              )}`,
          )
          .join(" · "),
      }));
    } catch (error) {
      set({ notice: appErrorMessage(error, get().settings.language) });
    } finally {
      set({ importing: false });
    }
  },

  remove: async (id) => {
    if (get().batchBusy || get().importing) return;
    try {
      await api.releaseFiles([id]);
      set((state) => {
        const files = state.files.filter((file) => file.id !== id);
        return {
          files,
          activeId:
            state.activeId === id ? (files[0]?.id ?? "") : state.activeId,
        };
      });
    } catch (error) {
      set({ notice: appErrorMessage(error, get().settings.language) });
    }
  },

  choosePreset: (id) => {
    const preset = [...builtIns, ...get().presets].find(
      (candidate) => candidate.id === id,
    );
    if (!preset) return;

    set({ presetId: id, params: { ...preset.params } });
    get().setSettings({ ...get().settings, lastPresetId: id });
  },

  savePreset: async (name) => {
    if (!validPresetName(name, [...builtIns, ...get().presets])) return false;

    try {
      const preset = await api.savePreset(name.trim(), get().params);
      set((state) => ({
        presets: [...state.presets, preset],
        presetId: preset.id,
      }));
      get().setSettings({ ...get().settings, lastPresetId: preset.id });
      return true;
    } catch (error) {
      set({ notice: appErrorMessage(error, get().settings.language) });
      return false;
    }
  },

  deletePreset: async (id) => {
    try {
      await api.deletePreset(id);
      set((state) => ({
        presets: state.presets.filter((preset) => preset.id !== id),
      }));
      if (get().presetId === id) {
        get().choosePreset(builtIns[0].id);
      }
    } catch (error) {
      set({ notice: appErrorMessage(error, get().settings.language) });
    }
  },

  setSettings: (settings) => {
    const sequence = ++settingsSequence;
    set({ settings });

    settingsQueue = settingsQueue.then(async () => {
      try {
        const normalized = await api.saveSettings(settings);
        if (sequence === settingsSequence) {
          set({ settings: normalized });
        }
      } catch (error) {
        if (sequence === settingsSequence) {
          set({ notice: appErrorMessage(error, get().settings.language) });
        }
      }
    });
  },
}));
