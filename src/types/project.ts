import type { Preset, TraceParams } from "./params";

export interface AppError {
  code:
    | "FILE_NOT_FOUND"
    | "UNSUPPORTED_FORMAT"
    | "IMAGE_CORRUPT"
    | "IMAGE_TOO_LARGE"
    | "INVALID_PARAMS"
    | "TRACE_FAILED"
    | "EXPORT_FAILED"
    | "WRITE_FAILED"
    | "CANCELLED"
    | "PRESET_DUPLICATE"
    | "PRESET_READ_ONLY"
    | "NOT_FOUND"
    | "ACCESS_DENIED"
    | "SOURCE_CHANGED"
    | "BATCH_BUSY"
    | "INVALID_STATE"
    | "OUTPUT_TOO_LARGE"
    | "DATA_VERSION_UNSUPPORTED"
    | "DATA_CORRUPT";
  message: string;
  details?: string;
}

export interface SourceFile {
  id: string;
  name: string;
  width: number;
  height: number;
  bytes: number;
  format: "png" | "jpeg" | "webp" | "bmp";
  hasAlpha: boolean;
  previewUrl: string;
  fingerprint: string;
}

export interface ImportRejection {
  name: string;
  error: AppError;
}

export interface ImportResult {
  files: SourceFile[];
  rejected: ImportRejection[];
}

export type ExportFormat = "svg" | "pdf" | "eps";

export interface DestinationRequest {
  kind: "file" | "directory";
  suggestedName?: string;
  format?: ExportFormat;
}

export interface Destination {
  id: string;
  displayPath: string;
  kind: "file" | "directory";
}

export interface AppSettings {
  language: "id" | "en";
  theme: "dark" | "light" | "system";
  lastOutDir?: string;
  workerCount: number;
  autoPreview: boolean;
  previewMaxSide: number;
  lastPresetId?: string;
}

export interface TraceStats {
  pathCount: number;
  colorCount: number;
  svgBytes: number;
  width: number;
  height: number;
}

export interface PreviewRequest {
  fileId: string;
  params: TraceParams;
  maxSide: number;
  requestId: string;
}

export interface PreviewResult {
  requestId: string;
  fileId: string;
  svg: string;
  stats: TraceStats;
  elapsedMs: number;
}

export interface ExportRequest {
  fileId: string;
  params: TraceParams;
  format: ExportFormat;
  destinationId: string;
  allowLargeOutput: boolean;
}

export interface ExportResult {
  outputId: string;
  outPath: string;
  format: ExportFormat;
  bytes: number;
  stats: TraceStats;
  elapsedMs: number;
}

export type ItemStatus =
  | "queued"
  | "processing"
  | "done"
  | "partial"
  | "failed"
  | "cancelled";
export type OutputStatus =
  | "queued"
  | "processing"
  | "done"
  | "failed"
  | "cancelled";
export type JobStage =
  | "waiting"
  | "decoding"
  | "tracing"
  | "exporting"
  | "finished";

export interface BatchRequest {
  fileIds: string[];
  params: TraceParams;
  formats: ExportFormat[];
  destinationId: string;
  overwrite: boolean;
}

export interface BatchHandle {
  batchId: string;
  runId: string;
}

export interface BatchOutput {
  format: ExportFormat;
  status: OutputStatus;
  outputId?: string;
  outPath?: string;
  bytes?: number;
  error?: AppError;
}

export interface BatchItem {
  id: string;
  fileId: string;
  name: string;
  status: ItemStatus;
  stage: JobStage;
  outputs: BatchOutput[];
  error?: AppError;
  elapsedMs?: number;
}

export interface BatchProgress {
  batchId: string;
  runId: string;
  sequence: number;
  status: "running" | "cancelling" | "finished";
  items: BatchItem[];
  completedCount: number;
  doneCount: number;
  partialCount: number;
  failedCount: number;
  cancelledCount: number;
  total: number;
}

export interface BatchSummary {
  batchId: string;
  runId: string;
  sequence: number;
  succeeded: number;
  partial: number;
  failed: number;
  cancelledCount: number;
  cancelRequested: boolean;
  total: number;
  totalElapsedMs: number;
}

export type NativeDropEvent =
  | { type: "over"; paths?: never }
  | { type: "drop"; paths: string[] }
  | { type: "leave"; paths?: never };

export type { Preset };
