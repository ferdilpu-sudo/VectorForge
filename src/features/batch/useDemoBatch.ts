import { useEffect, useRef, useState } from "react";
import { useProject } from "../../stores/project.store";
import type { DemoItem, ExportFormat } from "../../types/project";
export function useDemoBatch() {
  const [items, setItems] = useState<DemoItem[]>([]);
  const [busy, setBusy] = useState(false);
  const [cancelling, setCancelling] = useState(false);
  const abort = useRef(false);
  const alive = useRef(true);
  const snapshot = useRef("");
  useEffect(() => {
    alive.current = true;
    return () => {
      alive.current = false;
      abort.current = true;
      useProject.setState({ batchBusy: false });
    };
  }, []);
  const run = async (initial: DemoItem[], fail: boolean, targetId?: string) => {
    abort.current = false;
    setBusy(true);
    setCancelling(false);
    useProject.setState({ batchBusy: true });
    const jobs = structuredClone(initial);
    const publish = () => {
      if (alive.current) setItems(structuredClone(jobs));
    };
    publish();
    try {
      for (const item of jobs) {
        if (item.status === "done" || (targetId && item.id !== targetId))
          continue;
        for (const output of item.outputs) {
          if (output.status === "done") continue;
          if (abort.current) {
            output.status = "cancelled";
            continue;
          }
          item.status = "processing";
          item.stage = "Simulasi / Simulation";
          output.status = "processing";
          publish();
          await new Promise((r) => setTimeout(r, 350));
          output.status = abort.current
            ? "cancelled"
            : fail && output.format === "pdf"
              ? "failed"
              : "done";
          publish();
        }
        item.status = item.outputs.some((o) => o.status === "cancelled")
          ? "cancelled"
          : item.outputs.every((o) => o.status === "done")
            ? "done"
            : item.outputs.some((o) => o.status === "done")
              ? "partial"
              : "failed";
        item.stage = "";
        publish();
      }
    } finally {
      if (alive.current) {
        setBusy(false);
        setCancelling(false);
      }
      useProject.setState({ batchBusy: false });
    }
  };
  return {
    items,
    busy,
    cancelling,
    snapshot: snapshot.current,
    start: (formats: ExportFormat[], fail: boolean) => {
      const state = useProject.getState();
      if (state.batchBusy || !formats.length) return;
      snapshot.current = JSON.stringify(state.params);
      void run(
        state.files.map((f) => ({
          id: crypto.randomUUID(),
          fileId: f.id,
          name: f.name,
          status: "queued",
          stage: "",
          outputs: formats.map((format) => ({ format, status: "queued" })),
        })),
        fail,
      );
    },
    cancel: () => {
      abort.current = true;
      setCancelling(true);
    },
    retry: (id: string) => {
      if (busy) return;
      void run(
        items.map((item) =>
          item.id === id
            ? {
                ...item,
                status: "queued",
                outputs: item.outputs.map((o) =>
                  o.status === "failed" ? { ...o, status: "queued" } : o,
                ),
              }
            : item,
        ),
        false,
        id,
      );
    },
  };
}
