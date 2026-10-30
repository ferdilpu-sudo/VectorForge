export interface TraceParams {
  colorPrecision: number;
  filterSpeckle: number;
  layerDifference: number;
  cornerThreshold: number;
  lengthThreshold: number;
  // Curve simplification tolerance in source pixels; 0 preserves legacy output.
  smoothness: number;
  segmentationMode: "color-cluster" | "watershed";
  // 0 disables palette reduction; positive values target a maximum color count.
  paletteLimit: number;
  geometryMode: "classic" | "line-aware";
  mode: "spline" | "polygon";
  hierarchical: "stacked" | "cutout";
}
export interface Preset {
  id: string;
  name: string;
  params: TraceParams;
  createdAt: string;
  builtIn: boolean;
}
export const defaults: TraceParams = {
  colorPrecision: 6,
  filterSpeckle: 4,
  layerDifference: 16,
  cornerThreshold: 60,
  lengthThreshold: 4,
  smoothness: 0,
  segmentationMode: "color-cluster",
  paletteLimit: 0,
  geometryMode: "classic",
  mode: "spline",
  hierarchical: "stacked",
};
export const ranges = {
  colorPrecision: [1, 8, 1],
  filterSpeckle: [0, 128, 1],
  layerDifference: [0, 128, 1],
  cornerThreshold: [0, 180, 1],
  lengthThreshold: [3.5, 10, 0.1],
  smoothness: [0, 3, 0.25],
  paletteLimit: [0, 64, 1],
} as const;
export function validParams(p: TraceParams): boolean {
  return (
    Object.entries(ranges).every(([key, [min, max, step]]) => {
      const n = p[key as keyof typeof ranges];
      return (
        Number.isFinite(n) &&
        n >= min &&
        n <= max &&
        (step !== 1 || Number.isInteger(n))
      );
    }) &&
    ["color-cluster", "watershed"].includes(p.segmentationMode) &&
    ["classic", "line-aware"].includes(p.geometryMode) &&
    ["spline", "polygon"].includes(p.mode) &&
    ["stacked", "cutout"].includes(p.hierarchical)
  );
}
export const builtIns: Preset[] = [
  ["balanced", "Seimbang", {}],
  [
    "logo",
    "Logo & Flat",
    { colorPrecision: 4, filterSpeckle: 8, layerDifference: 32 },
  ],
  [
    "photo",
    "Foto Detail",
    { colorPrecision: 8, filterSpeckle: 2, layerDifference: 8 },
  ],
  ["poster", "Poster Halus", { colorPrecision: 7, cornerThreshold: 90 }],
  [
    "ai-smooth",
    "AI Smooth Illustration",
    {
      colorPrecision: 6,
      filterSpeckle: 5,
      layerDifference: 20,
      cornerThreshold: 75,
      smoothness: 1.5,
      mode: "spline",
    },
  ],
  [
    "professional-logo",
    "Professional Logo",
    {
      colorPrecision: 5,
      filterSpeckle: 3,
      layerDifference: 20,
      cornerThreshold: 45,
      smoothness: 0,
      hierarchical: "cutout",
      segmentationMode: "color-cluster",
      paletteLimit: 0,
      geometryMode: "line-aware",
    },
  ],
  [
    "professional-illustration",
    "Professional Illustration",
    {
      colorPrecision: 6,
      filterSpeckle: 5,
      layerDifference: 18,
      cornerThreshold: 75,
      smoothness: 0.5,
      hierarchical: "cutout",
      segmentationMode: "watershed",
      paletteLimit: 24,
    },
  ],
].map(([id, name, params]) => ({
  id: `builtin-${id}`,
  name: String(name),
  params: { ...defaults, ...(params as Partial<TraceParams>) },
  builtIn: true,
  createdAt: "2026-09-30T00:00:00Z",
}));
export function validPresetName(name: string, presets: Preset[]): boolean {
  return (
    [...name.trim()].length > 0 &&
    [...name.trim()].length <= 40 &&
    !presets.some((p) => p.name.toLowerCase() === name.trim().toLowerCase())
  );
}
