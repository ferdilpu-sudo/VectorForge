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
  it("exposes opt-in professional modes and retains the classic legacy defaults", () => {
    expect(defaults.segmentationMode).toBe("color-cluster");
    expect(defaults.paletteLimit).toBe(0);
    const logo = builtIns.find((preset) => preset.id === "builtin-professional-logo");
    const illustration = builtIns.find(
      (preset) => preset.id === "builtin-professional-illustration",
    );
    expect(logo?.params.hierarchical).toBe("cutout");
    expect(logo?.params.smoothness).toBe(0);
    expect(illustration?.params.segmentationMode).toBe("watershed");
    expect(illustration?.params.paletteLimit).toBe(24);
    expect(
      builtIns.find((p) => p.id === "builtin-logo")?.params.segmentationMode,
    ).toBe("color-cluster");
  });
  it("rejects invalid segmentation choices and palette limits", () => {
    expect(
      validParams({ ...defaults, segmentationMode: "watershed", paletteLimit: 24 }),
    ).toBe(true);
    expect(validParams({ ...defaults, paletteLimit: 65 })).toBe(false);
    expect(validParams({ ...defaults, paletteLimit: -1 })).toBe(false);
    expect(validParams({ ...defaults, paletteLimit: 1.5 })).toBe(false);
    expect(validParams({ ...defaults, segmentationMode: "invalid" as "watershed" })).toBe(false);
  });
  it("keeps legacy geometry classic and enables line-aware only on precision logo", () => {
    expect(defaults.geometryMode).toBe("classic");
    expect(builtIns.find((item) => item.id === "builtin-logo")?.params.geometryMode).toBe("classic");
    expect(
      builtIns.find((item) => item.id === "builtin-professional-logo")?.params.geometryMode,
    ).toBe("line-aware");
    expect(
      builtIns.find((item) => item.id === "builtin-professional-illustration")?.params.geometryMode,
    ).toBe("classic");
    expect(validParams({ ...defaults, geometryMode: "line-aware" })).toBe(true);
    expect(validParams({ ...defaults, geometryMode: "wrong" as "classic" })).toBe(false);
  });
  it("enables the quality guard only for professional presets", () => {
    expect(defaults.qualityGuard).toBe(false);
    expect(builtIns.find((p) => p.id === "builtin-logo")?.params.qualityGuard).toBe(false);
    expect(builtIns.find((p) => p.id === "builtin-professional-logo")?.params.qualityGuard).toBe(true);
    expect(
      builtIns.find((p) => p.id === "builtin-professional-illustration")?.params.qualityGuard,
    ).toBe(true);
    expect(validParams({ ...defaults, qualityGuard: true })).toBe(true);
    expect(validParams({ ...defaults, qualityGuard: 1 as unknown as boolean })).toBe(false);
  });
  it("keeps raster feedback opt-in and validates boolean flags", () => {
    expect(defaults.rasterFeedback).toBe(false);
    expect(builtIns.every((preset) => preset.params.rasterFeedback === false)).toBe(true);
    expect(validParams({ ...defaults, rasterFeedback: true })).toBe(true);
    expect(validParams({ ...defaults, rasterFeedback: "yes" as unknown as boolean })).toBe(false);
  });
  it("rejects duplicate preset names ignoring case and surrounding spaces", () => {
    expect(validPresetName("  seimbang  ", builtIns)).toBe(false);
    expect(validPresetName(" ", builtIns)).toBe(false);
    expect(validPresetName("x".repeat(41), builtIns)).toBe(false);
    expect(validPresetName("Logo saya", builtIns)).toBe(true);
  });
});
