// @vitest-environment jsdom
import { describe, expect, it } from "vitest";

import { appErrorMessage } from "../services/errors";
import { defaultSettings } from "../services/preferences";

describe("native preference defaults", () => {
  it("keeps canonical settings inside Rust-validated bounds", () => {
    expect(defaultSettings.workerCount).toBeGreaterThanOrEqual(1);
    expect(defaultSettings.workerCount).toBeLessThanOrEqual(4);
    expect(defaultSettings.previewMaxSide).toBe(1024);
    expect(defaultSettings.autoPreview).toBe(true);
  });
});

describe("native error localization", () => {
  it("maps a canonical AppError code to one active locale", () => {
    const error = {
      code: "IMAGE_TOO_LARGE",
      message: "Gambar melebihi batas.",
    };

    expect(appErrorMessage(error, "id")).toBe("Gambar melebihi 30 MP");
    expect(appErrorMessage(error, "en")).toBe("Image exceeds 30 MP");
  });
});
