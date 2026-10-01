import type { TraceParams } from "./params";
export interface SourceFile {
  id: string;
  name: string;
  width: number;
  height: number;
  bytes: number;
  previewUrl: string;
  fingerprint: string;
}
export type ExportFormat = "svg" | "pdf" | "eps";
export interface AppSettings {
  language: "id" | "en";
  theme: "dark" | "light" | "system";
  workerCount: number;
  autoPreview: boolean;
  previewMaxSide: number;
}
export interface PreviewRequest {
  fileId: string;
  params: TraceParams;
  maxSide: number;
  requestId: string;
}
export interface PreviewResult {
  fileId: string;
  requestId: string;
  svg: string;
  elapsedMs: number;
}
export type OutputStatus =
  "queued" | "processing" | "done" | "failed" | "cancelled";
export interface DemoOutput {
  format: ExportFormat;
  status: OutputStatus;
}
export interface DemoItem {
  id: string;
  fileId: string;
  name: string;
  status: OutputStatus | "partial";
  stage: string;
  outputs: DemoOutput[];
}
