// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, renderHook, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const native = vi.hoisted(() => {
  let onProgress:
    | ((value: import("../types/project").BatchProgress) => void)
    | undefined;
  let onDone:
    | ((value: import("../types/project").BatchSummary) => void)
    | undefined;

  const unlisten = vi.fn();
  return {
    generatePreview: vi.fn(),
    cancelPreview: vi.fn(),
    saveSettings: vi.fn(),
    subscribeBatchEvents: vi.fn(
      async (
        progress: (value: import("../types/project").BatchProgress) => void,
        done: (value: import("../types/project").BatchSummary) => void,
      ) => {
        onProgress = progress;
        onDone = done;
        return unlisten;
      },
    ),
    startBatch: vi.fn(),
    getBatch: vi.fn(),
    cancelBatch: vi.fn(),
    retryBatchItem: vi.fn(),
    unlisten,
    progress: (value: import("../types/project").BatchProgress) =>
      onProgress?.(value),
    done: (value: import("../types/project").BatchSummary) => onDone?.(value),
  };
});

vi.mock("../services/api", () => ({
  api: {
    generatePreview: native.generatePreview,
    cancelPreview: native.cancelPreview,
    saveSettings: native.saveSettings,
    subscribeBatchEvents: native.subscribeBatchEvents,
    startBatch: native.startBatch,
    getBatch: native.getBatch,
    cancelBatch: native.cancelBatch,
    retryBatchItem: native.retryBatchItem,
  },
}));

import { useBatch } from "../features/batch/useBatch";
import { usePreview } from "../features/compare/usePreview";
import { ParamPanel } from "../features/params/ParamPanel";
import { defaultSettings } from "../services/preferences";
import { useProject } from "../stores/project.store";
import { builtIns, defaults } from "../types/params";
import type { BatchProgress, Destination } from "../types/project";

let serial = 0;

function progress(
  sequence: number,
  runId = "00000000-0000-4000-8000-000000000002",
): BatchProgress {
  return {
    batchId: "00000000-0000-4000-8000-000000000001",
    runId,
    sequence,
    status: sequence >= 2 ? "finished" : "running",
    items: [
      {
        id: "00000000-0000-4000-8000-000000000003",
        fileId: "00000000-0000-4000-8000-000000000010",
        name: "a.png",
        status: sequence >= 2 ? "done" : "queued",
        stage: sequence >= 2 ? "finished" : "waiting",
        outputs: [
          {
            format: "svg",
            status: sequence >= 2 ? "done" : "queued",
          },
        ],
      },
    ],
    completedCount: sequence >= 2 ? 1 : 0,
    doneCount: sequence >= 2 ? 1 : 0,
    partialCount: 0,
    failedCount: 0,
    cancelledCount: 0,
    total: 1,
  };
}

beforeEach(() => {
  vi.useFakeTimers();
  vi.clearAllMocks();
  serial = 0;
  vi.stubGlobal(
    "URL",
    Object.assign(URL, {
      createObjectURL: vi.fn(() => `blob:${++serial}`),
      revokeObjectURL: vi.fn(),
    }),
  );

  native.generatePreview.mockImplementation(async (request) => ({
    requestId: request.requestId,
    fileId: request.fileId,
    svg: "<svg/>",
    stats: {
      pathCount: 1,
      colorCount: 1,
      svgBytes: 6,
      width: 100,
      height: 100,
    },
    elapsedMs: 1,
  }));
  native.cancelPreview.mockResolvedValue(undefined);
  native.saveSettings.mockImplementation(async (settings) => settings);

  useProject.setState({
    files: [
      {
        id: "00000000-0000-4000-8000-000000000010",
        name: "a.png",
        width: 100,
        height: 100,
        bytes: 100,
        format: "png",
        hasAlpha: true,
        previewUrl:
          "http://vfsource.localhost/00000000-0000-4000-8000-000000000010",
        fingerprint: "a",
      },
    ],
    activeId: "00000000-0000-4000-8000-000000000010",
    params: { ...defaults },
    settings: { ...defaultSettings },
    ready: true,
    batchBusy: false,
    importing: false,
    notice: "",
  });
});

afterEach(() => {
  cleanup();
  vi.useRealTimers();
  vi.unstubAllGlobals();
});

describe("preview lifecycle", () => {
  it("hides stale results when parameters change and releases old URLs", async () => {
    const { result } = renderHook(() => usePreview(0));

    await act(async () => {
      await vi.advanceTimersByTimeAsync(301);
    });
    const previous = result.current.url;
    expect(previous).toMatch(/^blob:/);

    act(() =>
      useProject.setState({
        params: { ...defaults, colorPrecision: 3 },
      }),
    );

    expect(result.current.url).toBe("");
    expect(result.current.stale).toBe(true);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(301);
    });

    expect(native.cancelPreview).toHaveBeenCalled();
    expect(result.current.url).toMatch(/^blob:/);
    expect(result.current.url).not.toBe(previous);
    expect(result.current.stale).toBe(false);
    expect(URL.revokeObjectURL).toHaveBeenCalledWith(previous);
  });

  it("invalidates on preview resolution changes even when params stay the same", async () => {
    const { result } = renderHook(() => usePreview(0));
    await act(async () => {
      await vi.advanceTimersByTimeAsync(301);
    });
    const previous = result.current.url;

    act(() => {
      useProject.setState((state) => ({
        settings: { ...state.settings, previewMaxSide: 2048 },
      }));
    });
    expect(result.current.url).toBe("");
    expect(result.current.stale).toBe(true);

    await act(async () => {
      await vi.advanceTimersByTimeAsync(301);
    });
    expect(native.generatePreview).toHaveBeenLastCalledWith(
      expect.objectContaining({ maxSide: 2048 }),
    );
    expect(result.current.url).not.toBe(previous);
  });

  it("does not accept a mismatched native preview response", async () => {
    native.generatePreview.mockResolvedValueOnce({
      requestId: "00000000-0000-4000-8000-000000000099",
      fileId: useProject.getState().activeId,
      svg: "<svg/>",
    });
    const { result } = renderHook(() => usePreview(0));
    await act(async () => {
      await vi.advanceTimersByTimeAsync(301);
    });
    expect(result.current.url).toBe("");
    expect(result.current.error).toContain("Respons preview");
  });

  it("keeps manual preview stale until Refresh is requested", async () => {
    useProject.setState((state) => ({
      settings: { ...state.settings, autoPreview: false },
    }));
    const { result, rerender } = renderHook(
      ({ manual }) => usePreview(manual),
      { initialProps: { manual: 0 } },
    );
    await act(async () => {
      await vi.advanceTimersByTimeAsync(301);
    });
    expect(native.generatePreview).not.toHaveBeenCalled();
    expect(result.current.needsRefresh).toBe(true);

    rerender({ manual: 1 });
    await act(async () => {
      await vi.advanceTimersByTimeAsync(301);
    });
    expect(result.current.url).toMatch(/^blob:/);
    expect(result.current.needsRefresh).toBe(false);
  });
});

describe("AI Smooth preview integration", () => {
  it("submits the smoothing tolerance in a native preview request", async () => {
    const ai = builtIns.find((preset) => preset.id === "builtin-ai-smooth");
    if (!ai) throw new Error("AI Smooth preset missing");

    act(() => useProject.setState({
      presetId: ai.id,
      params: { ...ai.params },
    }));

    const { result } = renderHook(() => usePreview(0));
    await act(async () => {
      await vi.advanceTimersByTimeAsync(301);
    });
    expect(native.generatePreview).toHaveBeenCalledWith(
      expect.objectContaining({
        params: expect.objectContaining({
          smoothness: 1.5,
          mode: "spline",
        }),
      }),
    );
    expect(result.current.url).toMatch(/^blob:/);
  });
});

describe("professional segmentation preview integration", () => {
  it("passes the selected segmentation and palette to native tracing", async () => {
    const preset = builtIns.find(
      (candidate) => candidate.id === "builtin-professional-illustration",
    );
    if (!preset) throw new Error("professional illustration preset missing");

    act(() => {
      useProject.setState({ presetId: preset.id, params: { ...preset.params } });
    });
    const { result } = renderHook(() => usePreview(0));
    await act(async () => {
      await vi.advanceTimersByTimeAsync(301);
    });
    expect(native.generatePreview).toHaveBeenCalledWith(
      expect.objectContaining({
        params: expect.objectContaining({
          segmentationMode: "watershed",
          paletteLimit: 24,
          hierarchical: "cutout",
        }),
      }),
    );
    expect(result.current.url).toMatch(/^blob:/);
  });
});

describe("line-aware geometry preview integration", () => {
  it("includes geometryMode in the native preview request", async () => {
    const preset = builtIns.find(
      (candidate) => candidate.id === "builtin-professional-logo",
    );
    if (!preset) throw new Error("precision logo preset missing");
    act(() => {
      useProject.setState({ presetId: preset.id, params: { ...preset.params } });
    });
    const { result } = renderHook(() => usePreview(0));
    await act(async () => {
      await vi.advanceTimersByTimeAsync(301);
    });
    expect(native.generatePreview).toHaveBeenCalledWith(
      expect.objectContaining({
        params: expect.objectContaining({
          geometryMode: "line-aware",
          mode: "spline",
        }),
      }),
    );
    expect(result.current.url).toMatch(/^blob:/);
  });
});

describe("batch event lifecycle", () => {
  it("resyncs after start, filters stale sequence/run events, and cleans listeners", async () => {
    const handle = {
      batchId: "00000000-0000-4000-8000-000000000001",
      runId: "00000000-0000-4000-8000-000000000002",
    };
    native.startBatch.mockResolvedValue(handle);
    native.getBatch.mockResolvedValue(progress(1));

    const destination: Destination = {
      id: "00000000-0000-4000-8000-000000000020",
      displayPath: "C:\\output",
      kind: "directory",
    };

    const { result, unmount } = renderHook(useBatch);
    await act(async () => {
      await result.current.start(["svg"], destination);
    });

    expect(result.current.items[0].status).toBe("queued");
    expect(useProject.getState().batchBusy).toBe(true);

    act(() => native.progress(progress(2)));
    expect(result.current.items[0].status).toBe("done");
    expect(result.current.busy).toBe(false);

    act(() => native.progress(progress(1)));
    expect(result.current.items[0].status).toBe("done");

    act(() =>
      native.progress(
        progress(99, "00000000-0000-4000-8000-000000000099"),
      ),
    );
    expect(result.current.items[0].status).toBe("done");

    unmount();
    expect(native.unlisten).toHaveBeenCalledTimes(1);
    expect(useProject.getState().batchBusy).toBe(false);
  });
});

describe("preset reset contract", () => {
  it("resets the selected Logo & Flat preset without switching to Balanced", () => {
    const logo = builtIns.find((preset) => preset.id === "builtin-logo");
    if (!logo) throw new Error("Logo & Flat preset missing");
    act(() => {
      useProject.setState({
        presetId: logo.id,
        params: { ...logo.params, colorPrecision: 8 },
      });
    });
    render(<ParamPanel />);
    fireEvent.click(screen.getByRole("button", { name: "Reset Parameter" }));
    expect(useProject.getState().presetId).toBe(logo.id);
    expect(useProject.getState().params).toEqual(logo.params);
  });
});
