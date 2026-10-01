// @vitest-environment jsdom
import { beforeEach, describe, expect, it } from "vitest";
import {
  corruptPreferenceBackupKey,
  defaultSettings,
  preferenceStorageKey,
  readPreferences,
  writePreferences,
} from "../services/preferences";
import {
  ImportFileError,
  importFileErrorMessage,
} from "../services/import-files";

beforeEach(() => {
  localStorage.clear();
});

describe("preference recovery", () => {
  it("backs up corrupt preferences and allows a clean local rewrite", () => {
    const corrupt = "{not-json";
    localStorage.setItem(preferenceStorageKey, corrupt);

    const recovered = readPreferences();

    expect(recovered.settings).toEqual(defaultSettings);
    expect(localStorage.getItem(corruptPreferenceBackupKey)).toBe(corrupt);
    expect(recovered.warning).toContain("reset");

    expect(writePreferences(defaultSettings, [])).toBe("");
    expect(JSON.parse(localStorage.getItem(preferenceStorageKey) ?? "{}")).toMatchObject({
      version: 1,
      settings: defaultSettings,
      presets: [],
    });
  });
});

describe("import error localization", () => {
  it("returns one locale instead of bilingual error copy", () => {
    const error = new ImportFileError("ANIMATED_UNSUPPORTED");

    expect(importFileErrorMessage(error, "id")).toBe(
      "Gambar animasi tidak didukung",
    );
    expect(importFileErrorMessage(error, "en")).toBe(
      "Animated images are unsupported",
    );
  });
});
