import { useCallback, useEffect, useRef, useState } from "react";

import { api } from "../../services/api";
import { appErrorMessage } from "../../services/errors";
import { useProject } from "../../stores/project.store";
import type {
  BatchHandle,
  BatchItem,
  BatchProgress,
  BatchSummary,
  Destination,
  ExportFormat,
} from "../../types/project";

export function useBatch() {
  const [items, setItems] = useState<BatchItem[]>([]);
  const [busy, setBusy] = useState(false);
  const [cancelling, setCancelling] = useState(false);
  const handle = useRef<BatchHandle | null>(null);
  const sequence = useRef(0);
  const alive = useRef(true);
  const listenerReady = useRef<Promise<void>>(Promise.resolve());
  const listenerError = useRef<unknown>(null);

  const finishRun = useCallback(() => {
    if (!alive.current) return;
    setBusy(false);
    setCancelling(false);
    useProject.setState({ batchBusy: false });
  }, []);

  const applyProgress = useCallback((progress: BatchProgress) => {
    const current = handle.current;
    if (
      !current ||
      progress.batchId !== current.batchId ||
      progress.runId !== current.runId ||
      progress.sequence <= sequence.current
    ) {
      return;
    }

    sequence.current = progress.sequence;
    if (!alive.current) return;
    setItems(progress.items);
    const running = progress.status !== "finished";
    setBusy(running);
    setCancelling(progress.status === "cancelling");
    useProject.setState({ batchBusy: running });
  }, []);

  const applyDone = useCallback((summary: BatchSummary) => {
    const current = handle.current;
    if (
      !current ||
      summary.batchId !== current.batchId ||
      summary.runId !== current.runId ||
      summary.sequence < sequence.current
    ) {
      return;
    }

    sequence.current = summary.sequence;
    finishRun();
  }, [finishRun]);

  useEffect(() => {
    alive.current = true;
    let disposed = false;
    let stop: (() => void) | undefined;

    const pending = api
      .subscribeBatchEvents(applyProgress, applyDone)
      .then((unlisten) => {
        if (disposed) unlisten();
        else stop = unlisten;
      })
      .catch((error) => {
        listenerError.current = error;
        if (!disposed) {
          useProject.setState({
            notice: appErrorMessage(
              error,
              useProject.getState().settings.language,
            ),
          });
        }
      });

    listenerError.current = null;
    listenerReady.current = pending;

    return () => {
      disposed = true;
      alive.current = false;
      stop?.();
      useProject.setState({ batchBusy: false });
    };
  }, [applyDone, applyProgress]);

  const resync = async (active: BatchHandle) => {
    const progress = await api.getBatch(active.batchId);
    applyProgress(progress);
  };

  return {
    items,
    busy,
    cancelling,

    start: async (formats: ExportFormat[], destination: Destination) => {
      const state = useProject.getState();
      if (state.batchBusy || !formats.length || !state.files.length) return;

      setBusy(true);
      useProject.setState({ batchBusy: true });

      try {
        await listenerReady.current;
        if (listenerError.current) throw listenerError.current;
        const active = await api.startBatch({
          fileIds: state.files.map((file) => file.id),
          params: { ...state.params },
          formats,
          destinationId: destination.id,
          overwrite: false,
        });
        handle.current = active;
        sequence.current = 0;
        await resync(active);
      } catch (error) {
        useProject.setState({
          notice: appErrorMessage(error, state.settings.language),
        });
        finishRun();
      }
    },

    cancel: async () => {
      const active = handle.current;
      if (!active || !busy || cancelling) return;

      setCancelling(true);
      try {
        await api.cancelBatch(active.batchId, active.runId);
        await resync(active);
      } catch (error) {
        setCancelling(false);
        useProject.setState({
          notice: appErrorMessage(
            error,
            useProject.getState().settings.language,
          ),
        });
      }
    },

    retry: async (itemId: string) => {
      const current = handle.current;
      if (!current || busy) return;

      setBusy(true);
      useProject.setState({ batchBusy: true });

      try {
        await listenerReady.current;
        if (listenerError.current) throw listenerError.current;
        const active = await api.retryBatchItem(current.batchId, itemId);
        handle.current = active;
        sequence.current = 0;
        await resync(active);
      } catch (error) {
        useProject.setState({
          notice: appErrorMessage(
            error,
            useProject.getState().settings.language,
          ),
        });
        finishRun();
      }
    },
  };
}
