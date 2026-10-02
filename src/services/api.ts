import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { confirm, open } from "@tauri-apps/plugin-dialog";

import type { Preset, TraceParams } from "../types/params";
import type {
  AppError,
  AppSettings,
  BatchHandle,
  BatchProgress,
  BatchRequest,
  BatchSummary,
  Destination,
  DestinationRequest,
  ExportRequest,
  ExportResult,
  ImportResult,
  NativeDropEvent,
  PreviewRequest,
  PreviewResult,
} from "../types/project";

export class NativeAppError extends Error {
  readonly code: AppError["code"];
  readonly details?: string;

  constructor(error: AppError) {
    super(error.message);
    this.name = "NativeAppError";
    this.code = error.code;
    this.details = error.details;
  }
}

function toNativeError(error: unknown): NativeAppError {
  if (
    typeof error === "object" &&
    error !== null &&
    "code" in error &&
    "message" in error &&
    typeof error.code === "string" &&
    typeof error.message === "string"
  ) {
    return new NativeAppError(error as AppError);
  }

  return new NativeAppError({
    code: "INVALID_STATE",
    message: "Operasi native VectorForge gagal.",
    details: error instanceof Error ? error.message : String(error),
  });
}

async function call<T>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (error) {
    throw toNativeError(error);
  }
}

async function pickSourcePaths(): Promise<string[]> {
  const selected = await open({
    multiple: true,
    directory: false,
    filters: [
      {
        name: "Images",
        extensions: ["png", "jpg", "jpeg", "webp", "bmp"],
      },
    ],
  });

  if (!selected) return [];
  return Array.isArray(selected) ? selected : [selected];
}

async function subscribeBatchEvents(
  onProgress: (progress: BatchProgress) => void,
  onDone: (summary: BatchSummary) => void,
): Promise<UnlistenFn> {
  const stopProgress = await listen<BatchProgress>(
    "batch://progress",
    (event) => onProgress(event.payload),
  );

  try {
    const stopDone = await listen<BatchSummary>("batch://done", (event) =>
      onDone(event.payload),
    );
    return () => {
      stopProgress();
      stopDone();
    };
  } catch (error) {
    stopProgress();
    throw toNativeError(error);
  }
}

async function watchCloseRequests(
  handler: (preventDefault: () => void) => void,
): Promise<UnlistenFn> {
  try {
    return await getCurrentWindow().onCloseRequested((event) => {
      handler(() => event.preventDefault());
    });
  } catch (error) {
    throw toNativeError(error);
  }
}

async function confirmCloseWhileBusy(
  language: AppSettings["language"],
): Promise<boolean> {
  try {
    return await confirm(
      language === "id"
        ? "Batch masih berjalan. Tutup VectorForge dan hentikan pekerjaan aktif?"
        : "A batch is still running. Close VectorForge and stop active work?",
      {
        title: "VectorForge",
        kind: "warning",
      },
    );
  } catch (error) {
    throw toNativeError(error);
  }
}

async function destroyCurrentWindow(): Promise<void> {
  try {
    await getCurrentWindow().destroy();
  } catch (error) {
    throw toNativeError(error);
  }
}

async function watchNativeDrops(
  handler: (event: NativeDropEvent) => void,
): Promise<UnlistenFn> {
  try {
    return await getCurrentWebview().onDragDropEvent((event) => {
      if (event.payload.type === "drop") {
        handler({ type: "drop", paths: event.payload.paths });
      } else if (event.payload.type === "over") {
        handler({ type: "over" });
      } else {
        handler({ type: "leave" });
      }
    });
  } catch (error) {
    throw toNativeError(error);
  }
}

export const api = {
  mode: "native" as const,
  pickSourcePaths,
  watchNativeDrops,
  watchCloseRequests,
  confirmCloseWhileBusy,
  destroyCurrentWindow,
  importFiles: (paths: string[]) =>
    call<ImportResult>("import_files", { request: { paths } }),
  releaseFiles: (fileIds: string[]) =>
    call<void>("release_files", { fileIds }),
  chooseDestination: (request: DestinationRequest) =>
    call<Destination | null>("choose_destination", { request }),
  generatePreview: (request: PreviewRequest) =>
    call<PreviewResult>("generate_preview", { request }),
  cancelPreview: (requestId: string) =>
    call<void>("cancel_preview", { requestId }),
  exportFile: (request: ExportRequest) =>
    call<ExportResult>("export_file", { request }),
  openOutputFolder: (outputId: string) =>
    call<void>("open_output_folder", { outputId }),
  startBatch: (request: BatchRequest) =>
    call<BatchHandle>("start_batch", { request }),
  getBatch: (batchId: string) =>
    call<BatchProgress>("get_batch", { batchId }),
  cancelBatch: (batchId: string, runId: string) =>
    call<void>("cancel_batch", { batchId, runId }),
  retryBatchItem: (batchId: string, itemId: string) =>
    call<BatchHandle>("retry_batch_item", { batchId, itemId }),
  subscribeBatchEvents,
  listPresets: () => call<Preset[]>("list_presets"),
  savePreset: (name: string, params: TraceParams) =>
    call<Preset>("save_preset", { request: { name, params } }),
  deletePreset: (id: string) => call<void>("delete_preset", { id }),
  getSettings: () => call<AppSettings>("get_settings"),
  saveSettings: (settings: AppSettings) =>
    call<AppSettings>("save_settings", { settings }),
};
