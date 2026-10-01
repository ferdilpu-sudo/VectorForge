// @vitest-environment jsdom
import { act, cleanup, renderHook } from "@testing-library/react";
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
    subscribeBatchEvents: native.subscribeBatchEvents,
    startBatch: native.startBatch,
    getBatch: native.getBatch,
    cancelBatch: native.cancelBatch,
    retryBatchItem: native.retryBatchItem,
  },
}));

import { useBatch } from "../features/batch/useBatch";
import { usePreview } from "../features/compare/usePreview";
import { defaultSettings } from "../services/preferences";
import { useProject } from "../stores/project.store";
import { defaults } from "../types/params";
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
  it("cancels obsolete native work and retains the prior image until replacement arrives", async () => {
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

    expect(result.current.url).toBe(previous);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(301);
    });

    expect(native.cancelPreview).toHaveBeenCalled();
    expect(result.current.url).not.toBe(previous);
    expect(URL.revokeObjectURL).toHaveBeenCalledWith(previous);
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
