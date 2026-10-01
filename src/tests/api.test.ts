// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from "vitest";

const tauri = vi.hoisted(() => {
  const invoke = vi.fn();
  const listen = vi.fn();
  const open = vi.fn();
  const onDragDropEvent = vi.fn();
  return { invoke, listen, open, onDragDropEvent };
});

vi.mock("@tauri-apps/api/core", () => ({ invoke: tauri.invoke }));
vi.mock("@tauri-apps/api/event", () => ({ listen: tauri.listen }));
vi.mock("@tauri-apps/api/webview", () => ({
  getCurrentWebview: () => ({ onDragDropEvent: tauri.onDragDropEvent }),
}));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: tauri.open }));

import { api } from "../services/api";
import { defaults } from "../types/params";

beforeEach(() => {
  vi.clearAllMocks();
});

describe("IPC command contract", () => {
  it("wraps canonical Rust command arguments exactly once", async () => {
    tauri.invoke.mockResolvedValue({});

    const request = {
      fileId: "00000000-0000-4000-8000-000000000001",
      params: { ...defaults },
      maxSide: 1024,
      requestId: "00000000-0000-4000-8000-000000000002",
    };
    await api.generatePreview(request);

    expect(tauri.invoke).toHaveBeenCalledWith("generate_preview", { request });

    await api.releaseFiles(["00000000-0000-4000-8000-000000000001"]);
    expect(tauri.invoke).toHaveBeenCalledWith("release_files", {
      fileIds: ["00000000-0000-4000-8000-000000000001"],
    });

    await api.saveSettings({
      language: "id",
      theme: "dark",
      workerCount: 2,
      autoPreview: true,
      previewMaxSide: 1024,
    });
    expect(tauri.invoke).toHaveBeenCalledWith("save_settings", {
      settings: {
        language: "id",
        theme: "dark",
        workerCount: 2,
        autoPreview: true,
        previewMaxSide: 1024,
      },
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
