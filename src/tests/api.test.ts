// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from "vitest";

const tauri = vi.hoisted(() => {
  const invoke = vi.fn();
  const listen = vi.fn();
  const open = vi.fn();
  const onDragDropEvent = vi.fn();
  const onCloseRequested = vi.fn();
  const close = vi.fn();
  const confirm = vi.fn();
  return {
    invoke,
    listen,
    open,
    onDragDropEvent,
    onCloseRequested,
    close,
    confirm,
  };
});

vi.mock("@tauri-apps/api/core", () => ({ invoke: tauri.invoke }));
vi.mock("@tauri-apps/api/event", () => ({ listen: tauri.listen }));
vi.mock("@tauri-apps/api/webview", () => ({
  getCurrentWebview: () => ({ onDragDropEvent: tauri.onDragDropEvent }),
}));
vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({
    onCloseRequested: tauri.onCloseRequested,
    close: tauri.close,
  }),
}));
vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: tauri.open,
  confirm: tauri.confirm,
}));

import { api } from "../services/api";
import { defaults } from "../types/params";

beforeEach(() => {
  vi.clearAllMocks();
});

describe("IPC command contract", () => {
  it("wraps every canonical Rust command with the documented arguments", async () => {
    tauri.invoke.mockResolvedValue({});

    const fileId = "00000000-0000-4000-8000-000000000001";
    const requestId = "00000000-0000-4000-8000-000000000002";
    const destinationId = "00000000-0000-4000-8000-000000000003";
    const batchId = "00000000-0000-4000-8000-000000000004";
    const runId = "00000000-0000-4000-8000-000000000005";
    const itemId = "00000000-0000-4000-8000-000000000006";
    const outputId = "00000000-0000-4000-8000-000000000007";
    const presetId = "00000000-0000-4000-8000-000000000008";

    await api.importFiles(["C:\\a.png"]);
    expect(tauri.invoke).toHaveBeenLastCalledWith("import_files", {
      request: { paths: ["C:\\a.png"] },
    });

    await api.releaseFiles([fileId]);
    expect(tauri.invoke).toHaveBeenLastCalledWith("release_files", {
      fileIds: [fileId],
    });

    const destinationRequest = { kind: "directory" as const };
    await api.chooseDestination(destinationRequest);
    expect(tauri.invoke).toHaveBeenLastCalledWith("choose_destination", {
      request: destinationRequest,
    });

    const previewRequest = {
      fileId,
      params: { ...defaults },
      maxSide: 1024,
      requestId,
    };
    await api.generatePreview(previewRequest);
    expect(tauri.invoke).toHaveBeenLastCalledWith("generate_preview", {
      request: previewRequest,
    });

    await api.cancelPreview(requestId);
    expect(tauri.invoke).toHaveBeenLastCalledWith("cancel_preview", {
      requestId,
    });

    const exportRequest = {
      fileId,
      params: { ...defaults },
      format: "svg" as const,
      destinationId,
      allowLargeOutput: false,
    };
    await api.exportFile(exportRequest);
    expect(tauri.invoke).toHaveBeenLastCalledWith("export_file", {
      request: exportRequest,
    });

    await api.openOutputFolder(outputId);
    expect(tauri.invoke).toHaveBeenLastCalledWith("open_output_folder", {
      outputId,
    });

    const batchRequest = {
      fileIds: [fileId],
      params: { ...defaults },
      formats: ["svg" as const],
      destinationId,
      overwrite: false,
    };
    await api.startBatch(batchRequest);
    expect(tauri.invoke).toHaveBeenLastCalledWith("start_batch", {
      request: batchRequest,
    });

    await api.getBatch(batchId);
    expect(tauri.invoke).toHaveBeenLastCalledWith("get_batch", { batchId });

    await api.cancelBatch(batchId, runId);
    expect(tauri.invoke).toHaveBeenLastCalledWith("cancel_batch", {
      batchId,
      runId,
    });

    await api.retryBatchItem(batchId, itemId);
    expect(tauri.invoke).toHaveBeenLastCalledWith("retry_batch_item", {
      batchId,
      itemId,
    });

    await api.listPresets();
    expect(tauri.invoke).toHaveBeenLastCalledWith("list_presets", undefined);

    await api.savePreset("Saya", defaults);
    expect(tauri.invoke).toHaveBeenLastCalledWith("save_preset", {
      request: { name: "Saya", params: defaults },
    });

    await api.deletePreset(presetId);
    expect(tauri.invoke).toHaveBeenLastCalledWith("delete_preset", {
      id: presetId,
    });

    await api.getSettings();
    expect(tauri.invoke).toHaveBeenLastCalledWith("get_settings", undefined);

    const settings = {
      language: "id" as const,
      theme: "dark" as const,
      workerCount: 2,
      autoPreview: true,
      previewMaxSide: 1024,
    };
    await api.saveSettings(settings);
    expect(tauri.invoke).toHaveBeenLastCalledWith("save_settings", {
      settings,
    });
  });

  it("uses native dialog paths instead of browser File objects", async () => {
    tauri.open.mockResolvedValue(["C:\\a.png", "C:\\b.jpg"]);

    await expect(api.pickSourcePaths()).resolves.toEqual([
      "C:\\a.png",
      "C:\\b.jpg",
    ]);
  });
});

describe("native listener lifecycle", () => {
  it("unsubscribes both batch listeners", async () => {
    const stopProgress = vi.fn();
    const stopDone = vi.fn();
    tauri.listen
      .mockResolvedValueOnce(stopProgress)
      .mockResolvedValueOnce(stopDone);

    const stop = await api.subscribeBatchEvents(vi.fn(), vi.fn());
    stop();

    expect(stopProgress).toHaveBeenCalledTimes(1);
    expect(stopDone).toHaveBeenCalledTimes(1);
  });

  it("maps native close requests and confirms active batch shutdown", async () => {
    const stop = vi.fn();
    let closeHandler:
      | ((event: { preventDefault: () => void }) => void)
      | undefined;
    tauri.onCloseRequested.mockImplementation(
      async (callback: typeof closeHandler) => {
        closeHandler = callback;
        return stop;
      },
    );
    tauri.confirm.mockResolvedValue(true);
    tauri.close.mockResolvedValue(undefined);

    const prevented = vi.fn();
    const unlisten = await api.watchCloseRequests((preventDefault) => {
      preventDefault();
    });
    closeHandler?.({ preventDefault: prevented });

    expect(prevented).toHaveBeenCalledTimes(1);
    await expect(api.confirmCloseWhileBusy("id")).resolves.toBe(true);
    expect(tauri.confirm).toHaveBeenCalledWith(
      "Batch masih berjalan. Tutup VectorForge dan hentikan pekerjaan aktif?",
      { title: "VectorForge", kind: "warning" },
    );

    await api.closeCurrentWindow();
    expect(tauri.close).toHaveBeenCalledTimes(1);

    unlisten();
    expect(stop).toHaveBeenCalledTimes(1);
  });

  it("maps native drag/drop events and returns the native unlisten function", async () => {
    const stop = vi.fn();
    let handler:
      | ((event: {
          payload:
            | { type: "over" }
            | { type: "drop"; paths: string[] }
            | { type: "leave" };
        }) => void)
      | undefined;

    tauri.onDragDropEvent.mockImplementation(
      async (callback: typeof handler) => {
        handler = callback;
        return stop;
      },
    );

    const received: string[][] = [];
    const unlisten = await api.watchNativeDrops((event) => {
      if (event.type === "drop") received.push(event.paths);
    });

    handler?.({ payload: { type: "drop", paths: ["C:\\a.png"] } });
    expect(received).toEqual([["C:\\a.png"]]);

    unlisten();
    expect(stop).toHaveBeenCalledTimes(1);
  });
});
