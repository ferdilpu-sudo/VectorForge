// @vitest-environment jsdom
import { afterEach, beforeEach, describe, it, expect, vi } from "vitest";
import { act, cleanup, renderHook } from "@testing-library/react";
import { useProject } from "../stores/project.store";
import { defaults } from "../types/params";
import { defaultSettings } from "../services/preferences";
import { usePreview } from "../features/compare/usePreview";
import { useDemoBatch } from "../features/batch/useDemoBatch";
let serial = 0;
beforeEach(() => {
  vi.useFakeTimers();
  vi.stubGlobal(
    "URL",
    Object.assign(URL, {
      createObjectURL: vi.fn(() => `blob:${++serial}`),
      revokeObjectURL: vi.fn(),
    }),
  );
  useProject.setState({
    files: [
      {
        id: "a",
        name: "a.png",
        width: 100,
        height: 100,
        bytes: 100,
        previewUrl: "blob:source",
        fingerprint: "a",
      },
    ],
    activeId: "a",
    params: { ...defaults },
    settings: { ...defaultSettings },
    batchBusy: false,
  });
});
afterEach(() => {
  cleanup();
  vi.useRealTimers();
  vi.unstubAllGlobals();
});
describe("preview lifecycle", () => {
  it("discards obsolete work and holds prior image while replacement is pending", async () => {
    const { result } = renderHook(() => usePreview(0));
    await act(async () => {
      await vi.advanceTimersByTimeAsync(800);
    });
    const previous = result.current.url;
    expect(previous).toMatch(/^blob:/);
    act(() =>
      useProject.setState({ params: { ...defaults, colorPrecision: 3 } }),
    );
    expect(result.current.url).toBe(previous);
    expect(URL.revokeObjectURL).not.toHaveBeenCalledWith(previous);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(800);
    });
    expect(result.current.url).not.toBe(previous);
    expect(URL.revokeObjectURL).toHaveBeenCalledWith(previous);
  });
  it("does not update on parameter changes with auto preview disabled after a manual update", async () => {
    useProject.setState({
      settings: { ...defaultSettings, autoPreview: false },
    });
    const { result, rerender } = renderHook(({ tick }) => usePreview(tick), {
      initialProps: { tick: 0 },
    });
    await act(async () => {
      await vi.advanceTimersByTimeAsync(800);
    });
    expect(result.current.url).toBe("");
    rerender({ tick: 1 });
    await act(async () => {
      await vi.advanceTimersByTimeAsync(800);
    });
    const url = result.current.url;
    expect(url).not.toBe("");
    act(() =>
      useProject.setState({ params: { ...defaults, colorPrecision: 2 } }),
    );
    await act(async () => {
      await vi.advanceTimersByTimeAsync(800);
    });
    expect(result.current.url).toBe(url);
    expect(result.current.stale).toBe(true);
  });
});
describe("batch lifecycle", () => {
  it("preserves successful formats while retrying a partial failure", async () => {
    const { result } = renderHook(useDemoBatch);
    act(() => result.current.start(["svg", "pdf"], true));
    await act(async () => {
      await vi.advanceTimersByTimeAsync(800);
    });
    expect(result.current.items[0].status).toBe("partial");
    const id = result.current.items[0].id;
    expect(result.current.items[0].outputs[0].status).toBe("done");
    act(() => result.current.retry(id));
    await act(async () => {
      await vi.advanceTimersByTimeAsync(400);
    });
    expect(result.current.items[0].status).toBe("done");
    expect(result.current.items[0].id).toBe(id);
    expect(useProject.getState().batchBusy).toBe(false);
  });
  it("cancels outstanding output and releases batch lock", async () => {
    const { result } = renderHook(useDemoBatch);
    act(() => result.current.start(["svg", "pdf"], false));
    act(() => result.current.cancel());
    await act(async () => {
      await vi.advanceTimersByTimeAsync(800);
    });
    expect(result.current.items[0].status).toBe("cancelled");
    expect(result.current.busy).toBe(false);
    expect(useProject.getState().batchBusy).toBe(false);
  });
});
