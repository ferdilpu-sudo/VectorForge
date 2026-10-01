import { describe, it, expect } from "vitest";
import {
  defaults,
  validParams,
  validPresetName,
  builtIns,
} from "../types/params";
import { generatePreview } from "../services/mock-engine";
describe("parameter contract", () => {
  it("accepts defaults and fractional segment length", () => {
    expect(validParams(defaults)).toBe(true);
    expect(validParams({ ...defaults, lengthThreshold: 3.5 })).toBe(true);
  });
  it("rejects non-finite, out of range and fractional integer values", () => {
    for (const value of [NaN, Infinity, 0, 9, 1.5])
      expect(validParams({ ...defaults, colorPrecision: value })).toBe(false);
  });
  it("rejects duplicate preset names ignoring case and surrounding spaces", () => {
    expect(validPresetName("  seimbang  ", builtIns)).toBe(false);
    expect(validPresetName(" ", builtIns)).toBe(false);
    expect(validPresetName("x".repeat(41), builtIns)).toBe(false);
    expect(validPresetName("Logo saya", builtIns)).toBe(true);
  });
  it("cancels mock work without returning stale data", async () => {
    const controller = new AbortController();
    const result = generatePreview(
      { fileId: "a", params: defaults, maxSide: 1024, requestId: "one" },
      controller.signal,
    );
    controller.abort();
    await expect(result).rejects.toHaveProperty("name", "AbortError");
  });
});
