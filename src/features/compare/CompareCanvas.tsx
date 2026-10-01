import { useEffect, useRef, useState } from "react";
import type { SourceFile } from "../../types/project";
import { useLanguage } from "../../shared/useLanguage";
import { usePreview } from "./usePreview";
export function CompareCanvas({
  file,
  onOpen,
}: {
  file?: SourceFile;
  onOpen: () => void;
}) {
  const t = useLanguage();
  const [manual, setManual] = useState(0);
  const preview = usePreview(manual);
  const [mode, setMode] = useState("compare");
  const [split, setSplit] = useState(50);
  const [zoom, setZoom] = useState(100);
  const [pan, setPan] = useState({ x: 0, y: 0 });
  const [bg, setBg] = useState("checker");
  const canvasRef = useRef<HTMLDivElement>(null);
  const drag = useRef<{ x: number; y: number } | null>(null);
  const [space, setSpace] = useState(false);
  useEffect(() => {
    setManual(0);
    setPan({ x: 0, y: 0 });
    const el = canvasRef.current;
    if (!el || !file) return;
    const observer = new ResizeObserver(() => {
      setZoom(
        Math.max(
          10,
          Math.min(
            800,
            100 *
              Math.min(
                (el.clientWidth * 0.75) / file.width,
                (el.clientHeight * 0.75) / file.height,
              ),
          ),
        ),
      );
    });
    observer.observe(el);
    return () => observer.disconnect();
  }, [file]);
  const zoomTo = (n: number) => setZoom(Math.max(10, Math.min(800, n)));
  const fit = () => {
    const el = canvasRef.current;
    if (el && file)
      zoomTo(
        100 *
          Math.min(
            (el.clientWidth * 0.75) / file.width,
            (el.clientHeight * 0.75) / file.height,
          ),
      );
    setPan({ x: 0, y: 0 });
  };
  return (
    <section className="canvas-section">
      <div className="canvas-toolbar">
        <div className="segmented">
          {[
            ["original", t("Asli", "Original")],
            ["compare", t("Bandingkan", "Compare")],
            ["result", t("Hasil", "Result")],
          ].map(([id, label]) => (
            <button
              key={id}
              aria-pressed={mode === id}
              onClick={() => setMode(id)}
            >
              {label}
            </button>
          ))}
        </div>
        <div className="canvas-actions">
          <button disabled={!file} onClick={() => setManual((n) => n + 1)}>
            {t("Perbarui", "Refresh")}
          </button>
          <select
            aria-label={t("Latar canvas", "Canvas background")}
            value={bg}
            onChange={(e) => setBg(e.target.value)}
          >
            <option value="checker">{t("Transparan", "Transparent")}</option>
            <option value="white">{t("Putih", "White")}</option>
            <option value="dark">{t("Gelap", "Dark")}</option>
          </select>
        </div>
      </div>
      <div
        ref={canvasRef}
        onWheel={(e) => {
          if (!file) return;
          const rect = e.currentTarget.getBoundingClientRect();
          const next = Math.max(
            10,
            Math.min(800, zoom * (e.deltaY < 0 ? 1.1 : 1 / 1.1)),
          );
          const ratio = next / zoom;
          const x = e.clientX - rect.left - rect.width / 2;
          const y = e.clientY - rect.top - rect.height / 2;
          setPan((p) => ({
            x: x - (x - p.x) * ratio,
            y: y - (y - p.y) * ratio,
          }));
          setZoom(next);
        }}
        className={`canvas ${file ? bg : "empty-canvas"}`}
        tabIndex={0}
        aria-label={t(
          "Canvas gambar. Pan dengan Space dan drag atau tombol arah.",
          "Image canvas. Pan with Space and drag or arrow keys.",
        )}
        onKeyDown={(e) => {
          if (e.target !== e.currentTarget) return;
          if (e.key === " ") {
            e.preventDefault();
            setSpace(true);
          }
          if (e.key.startsWith("Arrow")) {
            e.preventDefault();
            setPan((p) => ({
              x:
                p.x +
                (e.key === "ArrowLeft" ? -20 : e.key === "ArrowRight" ? 20 : 0),
              y:
                p.y +
                (e.key === "ArrowUp" ? -20 : e.key === "ArrowDown" ? 20 : 0),
            }));
          }
          if (e.key === "+") zoomTo(zoom + 10);
          if (e.key === "-") zoomTo(zoom - 10);
          if (e.ctrlKey && e.key === "0") {
            e.preventDefault();
            fit();
          }
        }}
        onKeyUp={(e) => {
          if (e.key === " ") setSpace(false);
        }}
        onBlur={() => {
          setSpace(false);
          drag.current = null;
        }}
        onPointerDown={(e) => {
          if (!space) return;
          e.currentTarget.setPointerCapture(e.pointerId);
          drag.current = { x: e.clientX - pan.x, y: e.clientY - pan.y };
        }}
        onPointerMove={(e) => {
          if (drag.current)
            setPan({
              x: e.clientX - drag.current.x,
              y: e.clientY - drag.current.y,
            });
        }}
        onPointerUp={() => {
          drag.current = null;
        }}
        onPointerCancel={() => {
          drag.current = null;
        }}
      >
        {!file ? (
          <div className="empty-state">
            <div className="empty-symbol">◇</div>
            <span className="section-eyebrow">RASTER → VECTOR</span>
            <h1>
              {t(
                "Ubah gambar menjadi vektor",
                "Turn images into vectors",
              )}
            </h1>
            <p>
              {t(
                "Tarik gambar ke sini untuk mulai.",
                "Drop an image here to get started.",
              )}
            </p>
            <button className="primary" onClick={onOpen}>
              {t("Pilih Gambar", "Choose image")} <span>↗</span>
            </button>
            <small>PNG · JPG · WEBP · BMP</small>
            <small>
              {t("Diproses di perangkat Anda", "Processed on your device")}
            </small>
          </div>
        ) : (
          <>
            <div
              className="artboard"
              style={{
                transform: `translate(${pan.x}px, ${pan.y}px)`,
                width: (file.width * zoom) / 100,
                height: (file.height * zoom) / 100,
                flexShrink: 0,
              }}
            >
              <img
                draggable={false}
                src={file.previewUrl}
                alt={t("Gambar asli", "Original image")}
                style={{ visibility: mode === "result" ? "hidden" : "visible" }}
              />
              {mode !== "original" && preview.url && (
                <img
                  draggable={false}
                  className="result-image"
                  src={preview.url}
                  alt="Demo illustration — not a trace"
                  style={{
                    clipPath:
                      mode === "compare" ? `inset(0 0 0 ${split}%)` : undefined,
                  }}
                />
              )}
              {mode === "compare" && (
                <div className="split-line" style={{ left: `${split}%` }} />
              )}
            </div>
            <div className="canvas-label left-label">
              {mode === "result" ? "DEMO" : t("ASLI", "ORIGINAL")}
            </div>
            {mode === "compare" && (
              <div className="canvas-label right-label">DEMO</div>
            )}
            <div className="preview-caption">
              {t(
                "Ilustrasi demo • bukan hasil tracing gambar Anda",
                "Demo illustration • not a trace of your image",
              )}
            </div>
            {mode === "compare" && (
              <input
                className="compare-range"
                type="range"
                min="0"
                max="100"
                value={split}
                aria-label={t("Pembatas perbandingan", "Comparison divider")}
                onChange={(e) => setSplit(Number(e.target.value))}
              />
            )}
          </>
        )}
      </div>
      <div className="canvas-footer">
        <span role="status">
          {preview.busy
            ? t("Memperbarui preview demo…", "Updating demo preview…")
            : preview.error ||
              (file
                ? preview.stale
                  ? t("Preview belum diperbarui", "Preview not updated")
                  : t("Preview demo siap", "Demo preview ready")
                : t("Siap menerima gambar", "Ready for an image"))}
        </span>
        <div className="zoom-controls">
          <button
            disabled={!file}
            aria-label="Zoom out"
            onClick={() => zoomTo(zoom - 10)}
          >
            −
          </button>
          <input
            aria-label="Zoom %"
            type="number"
            min="10"
            max="800"
            value={Math.round(zoom)}
            onChange={(e) => {
              if (e.target.value) zoomTo(Number(e.target.value));
            }}
          />
          <span>%</span>
          <button
            disabled={!file}
            aria-label="Zoom in"
            onClick={() => zoomTo(zoom + 10)}
          >
            +
          </button>
          <button disabled={!file} onClick={fit}>
            {t("Pas ke Layar", "Fit to view")}
          </button>
        </div>
      </div>
    </section>
  );
}
