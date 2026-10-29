import { describe, it, expect } from "vitest";
import {
  defaults,
  validParams,
  validPresetName,
  builtIns,
} from "../types/params";
describe("parameter contract", () => {
  it("accepts defaults and fractional segment length", () => {
    expect(validParams(defaults)).toBe(true);
    expect(validParams({ ...defaults, lengthThreshold: 3.5 })).toBe(true);
  });
  it("ships AI Smooth as opt-in without changing legacy defaults", () => {
    expect(defaults.smoothness).toBe(0);
    const ai = builtIns.find((p) => p.id === "builtin-ai-smooth");
    expect(ai).toBeDefined();
    expect(ai?.params.smoothness).toBe(1.5);
    expect(ai?.params.mode).toBe("spline");
    expect(builtIns.find((p) => p.id === "builtin-logo")?.params.smoothness).toBe(0);
  });
  it("accepts bounded smoothing and rejects invalid values", () => {
    expect(validParams({ ...defaults, smoothness: 0 })).toBe(true);
    expect(validParams({ ...defaults, smoothness: 1.5 })).toBe(true);
    expect(validParams({ ...defaults, smoothness: 3 })).toBe(true);
    for (const value of [-0.25, 3.25, NaN, Infinity]) {
      expect(validParams({ ...defaults, smoothness: value })).toBe(false);
    }
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
});
