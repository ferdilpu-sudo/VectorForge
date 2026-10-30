# Schema — Kontrak Data dan IPC

Revisi 1.1. JSON camelCase; DTO TS dan serde Rust harus setara. Contoh TS berikut adalah spesifikasi, bukan kode aplikasi yang sudah dikompilasi. ID opaque UUID, waktu ISO8601 UTC, bytes integer non-negatif, elapsedMs milidetik. Semua command async mengembalikan DTO atau menolak dengan AppError serializable. Nilai tidak valid ditolak, bukan clamp diam-diam.

## Parameter dan preset

```ts
interface TraceParams {
  colorPrecision: number; // integer 1–8, default 6
  filterSpeckle: number; // integer 0–128, default 4
  layerDifference: number; // integer 0–128, default 16
  cornerThreshold: number; // integer 0–180, default 60
  lengthThreshold: number; // finite 3.5–10, default 4; UI step 0.1
  smoothness: number; // finite 0–3 px, default 0; UI step 0.25
  segmentationMode: 'color-cluster' | 'watershed'; // default color-cluster
  paletteLimit: number; // integer 0–64, default 0 (= no quantization cap)
  geometryMode: 'classic' | 'line-aware'; // default classic; spline only
  qualityGuard: boolean; // default false; guard applied only in spline mode
  rasterFeedback: boolean; // default false; opt-in source comparison, max input 8MP
  mode: 'spline' | 'polygon'; // default spline
  hierarchical: 'stacked' | 'cutout'; // default stacked
}
interface Preset {
  id: string;
  name: string; // trim, 1–40 Unicode code points, unik case-insensitive
  params: TraceParams;
  createdAt: string;
  builtIn: boolean;
}
interface SavePresetRequest { name: string; params: TraceParams }
```

SavePreset v1 membuat baru; tidak ada update built-in atau overwrite nama diam-diam. Built-in IDs stabil: builtin-balanced, builtin-logo, builtin-photo, builtin-poster, builtin-ai-smooth. Seimbang default; Logo & Flat override colorPrecision=4/filterSpeckle=8/layerDifference=32; Foto Detail =8/2/8; Poster Halus colorPrecision=7/cornerThreshold=90/mode=spline; AI Smooth Illustration colorPrecision=6/filterSpeckle=5/layerDifference=20/cornerThreshold=75/smoothness=1.5/mode=spline. Field lainnya selalu default.

Kontrak tambahan QV-005: `smoothness` mengontrol toleransi simplify kurva VTracer dalam piksel input tracing (bukan skala layar), hanya aktif pada mode `spline`. `smoothness=0` setara konfigurasi engine lama (`simplify=None`). Backend menerima JSON preset lama yang belum memiliki field tersebut dengan nilai default 0 (`#[serde(default)]`). Field negatif, di atas 3, NaN dan Infinity ditolak; versi `PresetsFile` tetap 1 karena perubahan bersifat aditif dan kompatibel. Jika nama preset user versi 1 secara kebetulan sama dengan nama preset AI Smooth yang baru, file tetap dibaca (nama baru tidak dianggap reserved historis), namun penyimpanan preset baru tetap menolak duplikasi nama. Preview dan ekspor harus memakai nilai yang sama; kualitas visual belum dijamin tanpa evaluasi fixture.

## Kontrak tambahan QV-006 (Professional Segmentation)

Mode eksperimen, **opt-in**: `segmentationMode='color-cluster'` memakai segmentasi VTracer klasik; `'watershed'` memakai frontend watershed default VTracer (detail 128 pada versi dikunci). `paletteLimit=0` tidak memakai quantization cap; nilai positif (1–64) dipetakan ke `Config.max_colors` agar VTracer mengurangi palet setelah segmentasi. Kombinasi `hierarchical='cutout'` memakai mosaic shared-boundary untuk mengurangi celah antar area warna. Tidak otomatis menambah `cutout` saat user memilih watershed; pengguna dapat menguji setiap kombinasi.

Preset baru:
- `builtin-professional-logo` = colorPrecision 5, filterSpeckle 3, layerDifference 20, cornerThreshold 45, smoothness 0, cutout, color-cluster, paletteLimit 0.
- `builtin-professional-illustration` = colorPrecision 6, filterSpeckle 5, layerDifference 18, cornerThreshold 75, smoothness 0.5, cutout, watershed, paletteLimit 24.

Seluruh field baru memiliki default serde yang kompatibel dengan JSON preset lama: `SegmentationMode::ColorCluster` dan `palette_limit=0`. Nilai paletteLimit >64 ditolak di Rust dan TypeScript; field terpisah tidak mengubah kebijakan alpha/EPS. PresetsFile tetap version 1. Nama custom lama yang sama dengan built-in baru tetap dapat dibaca untuk mencegah regressi data. Perubahan kualitas wajib dinilai dengan kasus nyata; nama `Professional` adalah pilihan UX **eksperimental**, bukan jaminan hasil seperti gambar manual.

## Kontrak tambahan QV-007 (Line-aware Geometry)

`geometryMode='classic'` (default) mengabaikan pass baru agar output lama tidak berubah. `'line-aware'` menambahkan `PreserveStraightEdges` sebagai `CurvePass` setelah `Config.simplify` dan sebelum compositing; hanya berlaku untuk `mode='spline'`. Pass mendeteksi kubik dengan kedua handle mendekati chord (maksimum deviasi tegak lurus 0,6 px, panjang chord minimal 8 px, proyeksi handle berurutan tanpa overshoot). Hanya handle yang diganti dengan titik 1/3 dan 2/3 pada chord, sehingga anchor/junction dan kontur tetap. Pada `cutout`, pass ditempatkan pada batas bersama sebelum shape dibuat, untuk menghindari pengeditan dua salinan path secara terpisah.

`builtin-professional-logo` mengaktifkan `geometryMode='line-aware'`; semua preset sebelumnya dan `builtin-professional-illustration` menggunakan `classic`. Field `geometry_mode` Rust memiliki `#[serde(default)]`, sehingga file preset v1 lama tetap bisa dibaca tanpa migrasi versi. Ini adalah **perbaikan terbatas pada handle kubik yang hampir lurus**, bukan pendeteksi lingkaran/garis penuh, pemulihan simetri, atau mesin vector redraw manual. Tes visual terhadap logo dan ilustrasi wajib membandingkan bentuk serta kontur, bukan hanya jumlah kurva.

## Kontrak tambahan QV-008 (Contour Quality Guard)

`qualityGuard=false` adalah default kompatibilitas preset lama; jika true dan mode `spline`, VTracer `Config.simplify` dimatikan dan digantikan oleh satu `GuardedContourPass`. Pass ini menerapkan `SimplifyCurves` secara terkontrol dan opsional `PreserveStraightEdges`, lalu menilai kandidat terhadap kontur yang belum diubah. Kandidat ditolak jika endpoint berpindah, koordinat tak finite, sampled bidirectional nearest-polyline deviation >0,6 piksel, rata-rata jarak >0,2 piksel, orientasi area berubah, atau selisih area cincin >max(0,5 px², 1,5%). Bila simplifikasi gagal, pass mencoba perapian garis saja jika `geometryMode=line-aware`, lalu kembali ke kontur asli bila kandidat tetap gagal.

Untuk membatasi kompleksitas kuadrat, kontur dengan lebih dari 96 kubik tidak diubah oleh pass guard. Ini fallback konservatif, **bukan** pengukuran kualitas raster global. Sumber input, clustering, color quantization, dan kesalahan besar area sejak sebelum pass tidak dideteksi. Mode `polygon` tetap tidak mengaktifkan pass. `builtin-professional-logo` dan `builtin-professional-illustration` opt-in menggunakan guard, sedangkan preset lain tetap non-guard. PresetsFile v1 tetap dibaca dengan `qualityGuard=false` bila field belum ada. Jangan mengklaim SSIM atau pixel-fidelity meningkat tanpa pengujian render nyata.

## Kontrak QV-009 (Raster Feedback Gate)

`rasterFeedback=false` (default) tidak mengubah perilaku tracing, preview, batch atau ekspor. Saat true pada input hingga **8.000.000 piksel**, `spline` dan geometri yang akan berubah (`smoothness>0` atau `geometryMode=line-aware`), kedua kandidat menjalankan **segmentasi yang sama** melalui cache hasil `segment_with_progress` lalu dua `finish_with_progress`. Baseline: `smoothness=0`, `geometryMode=classic`, `qualityGuard=false`; kandidat: parameter user dengan guard QV-008 bila aktif. Pengaturan warna/alpha/cluster tetap identik.

Setiap VectorDoc diserialisasi ke SVG path asli, dirender kembali menggunakan `resvg=0.45.1` + `tiny-skia=0.11.4` (resvg sudah merupakan dependency transitif svg2pdf v0.13). Resolusi evaluasi: maksimal 384 piksel sisi panjang, **tidak** mengganti resolusi output ekspor. Error: rerender/parse gagal, SVG evaluasi >8 MiB, atau input >8MP → `AppError` eksplisit; tidak ada fallback tersembunyi. Metrik: rata-rata perbedaan absolut premultiplied RGBA (termasuk alpha), bobot 50% seluruh piksel + 50% area tepi sumber (kontras >=20). Kandidat diterima bila error ≤ error baseline + 0,35 + 1% error baseline; jika lebih buruk dipilih baseline. Pemeriksaan ini berbasis aproksimasi sampel source 2×2 dan raster SVG; **bukan** SSIM, perceptual QA sempurna, atau pengganti inspeksi kontur beresolusi penuh. Ini hanya menguji perubahan pasca-segmentasi; cacat segmentasi yang sama pada kedua kandidat tidak terdeteksi.

`rasterFeedback` tetap false di semua preset bawaan, termasuk dua preset profesional, karena overhead dua compositing + dua rendering harus diuji CPU/memori. JSON preset versi 1 lama tanpa field baru dimuat sebagai false melalui `#[serde(default)]`, tanpa migrasi format. Jika user memilih raster feedback pada PNG >8 MP, aplikasi memberi error dan meminta mode dimatikan; tidak mengecilkan hasil ekspor diam-diam.

## Import dan destination

```ts
interface SourceFile {
  id: string; name: string; width: number; height: number; bytes: number;
  format: 'png' | 'jpeg' | 'webp' | 'bmp';
  hasAlpha: boolean;
  fingerprint: string; // opaque dari Rust; bukan dipercaya dari frontend
  previewUrl: string; // scoped local asset/thumbnail URL
}
interface ImportRequest { paths: string[] } // hanya path ber-grant native yang valid
interface ImportRejection { name: string; error: AppError }
interface ImportResult { files: SourceFile[]; rejected: ImportRejection[] }
type ExportFormat = 'svg' | 'pdf' | 'eps';
interface DestinationRequest {
  kind: 'file' | 'directory';
  suggestedName?: string;
  format?: ExportFormat; // wajib bila kind=file
}
interface Destination { id: string; displayPath: string; kind: 'file' | 'directory' }
```

choose_destination membuka native dialog; cancel mengembalikan null dan bukan error toast. Destination file menyimpan keputusan overwrite native di Rust. Permission/grant tetap diperiksa meski DTO memiliki path/id yang tampak valid. Source dimensions setelah orientasi diterapkan. Sumber animasi, >30 MP, malformed header atau format palsu ditolak.

## Preview dan export

```ts
interface TraceStats {
  pathCount: number; colorCount: number; svgBytes: number;
  width: number; height: number;
}
interface PreviewRequest {
  fileId: string; params: TraceParams; maxSide: number; requestId: string;
}
interface PreviewResult {
  requestId: string; fileId: string; svg: string;
  stats: TraceStats; elapsedMs: number;
}
interface ExportRequest {
  fileId: string; params: TraceParams; format: ExportFormat;
  destinationId: string;
  allowLargeOutput: boolean; // false default; explicit confirmation untuk SVG >50 MiB
}
interface ExportResult {
  outputId: string; outPath: string; format: ExportFormat;
  bytes: number; stats: TraceStats; elapsedMs: number;
}
```

maxSide integer 512–2048, default 1024. Ukuran hasil mengikuti aspek sumber; tidak upscale sumber kecil. `outPath` hanya display, open folder memakai outputId. EPS flatten putih adalah kebijakan tetap v1, bukan parameter tersembunyi yang berubah per sesi.

## Batch

```ts
type ItemStatus = 'queued' | 'processing' | 'done' | 'partial' | 'failed' | 'cancelled';
type OutputStatus = 'queued' | 'processing' | 'done' | 'failed' | 'cancelled';
type JobStage = 'waiting' | 'decoding' | 'tracing' | 'exporting' | 'finished';
interface BatchRequest {
  fileIds: string[]; params: TraceParams; formats: ExportFormat[];
  destinationId: string; overwrite: boolean;
}
interface BatchHandle { batchId: string; runId: string }
interface BatchOutput {
  format: ExportFormat; status: OutputStatus;
  outputId?: string; outPath?: string; bytes?: number; error?: AppError;
}
interface BatchItem {
  id: string; fileId: string; name: string; status: ItemStatus;
  stage: JobStage; outputs: BatchOutput[]; error?: AppError; elapsedMs?: number;
}
interface BatchProgress {
  batchId: string; runId: string; sequence: number;
  status: 'running' | 'cancelling' | 'finished';
  items: BatchItem[]; completedCount: number; doneCount: number;
  partialCount: number; failedCount: number; cancelledCount: number; total: number;
}
interface BatchSummary {
  batchId: string; runId: string; sequence: number;
  succeeded: number; partial: number; failed: number; cancelledCount: number;
  cancelRequested: boolean; total: number; totalElapsedMs: number;
}
```

Semua format unik dan minimal satu; fileIds valid unik dan minimal satu. `done` jika seluruh output done; `partial` bila ada done dan failed; `failed` bila tak ada done dan ada failed; `cancelled` bila pekerjaan tersisa dibatalkan, termasuk item yang telah punya output done. Output done tetap disimpan. `completedCount` = done+partial+failed+cancelled. Sum terminal counters = total saat finished. Persentase total = completedCount/total, tidak sama dengan persentase sukses.

Saat retry terminal failed/partial, reset hanya output failed, pertahankan done; count snapshot dihitung ulang. Retry item cancelled tidak termasuk v1: buat batch baru secara eksplisit. runId baru, sequence mulai 1; item ID stabil. get_batch memberi snapshot terakhir run aktif/terakhir. Summary elapsed mengukur run saat ini, bukan kumulatif seluruh retry.

## Settings dan disk

```ts
interface AppSettings {
  language: 'id' | 'en'; // id
  theme: 'dark' | 'light' | 'system'; // dark
  lastOutDir?: string;
  workerCount: number; // integer 1–4, default min(4, cores)
  autoPreview: boolean; // true
  previewMaxSide: number; // integer 512–2048, default 1024
  lastPresetId?: string;
}
interface SettingsFile { version: 1; settings: AppSettings }
interface PresetsFile { version: 1; presets: Preset[] } // user only
```

Format settings revisi ini memakai wrapper `settings`, menggantikan contoh flat dalam dokumen awal sebelum implementasi. Tidak ada data pengguna produksi yang diketahui perlu migrasi. Jika ditemukan file flat nyata, jangan mengasumsikan boleh menimpa: implementasikan migrasi eksplisit yang diuji. Versi file lebih baru: jangan write; error DATA_VERSION_UNSUPPORTED. JSON corrupt: simpan file asal, tawarkan reset. Path terakhir hanya convenience, bukan grant akses. Preset ID hilang fallback Seimbang dengan informasi non-blocking.

## Registry command

| Command | Input | Output |
|---|---|---|
| import_files | ImportRequest | ImportResult |
| release_files | {fileIds: string[]} | void; tunda release yang masih direferensikan job |
| choose_destination | DestinationRequest | Destination atau null |
| generate_preview | PreviewRequest | PreviewResult |
| cancel_preview | {requestId: string} | void; obsolete/cancel requested |
| export_file | ExportRequest | ExportResult |
| start_batch | BatchRequest | BatchHandle |
| get_batch | {batchId: string} | BatchProgress |
| cancel_batch | {batchId: string, runId: string} | void; permintaan diterima, bukan jaminan selesai |
| retry_batch_item | {batchId: string, itemId: string} | BatchHandle |
| list_presets | tanpa argumen | Preset[] |
| save_preset | SavePresetRequest | Preset |
| delete_preset | {id: string} | void |
| get_settings | tanpa argumen | AppSettings |
| save_settings | {settings: AppSettings} | AppSettings normalized setelah validasi |
| open_output_folder | {outputId: string} | void |

Events `batch://progress`: BatchProgress, `batch://done`: BatchSummary. Tauri adapter membungkus DTO ke argumen command secara konsisten; nama argumen aktual dikunci dalam contract test. Error serde/native dipetakan AppError, tidak bocor sebagai string teknis mentah.

## Errors

```ts
interface AppError { code: ErrorCode; message: string; details?: string }
type ErrorCode =
  | 'FILE_NOT_FOUND' | 'UNSUPPORTED_FORMAT' | 'IMAGE_CORRUPT' | 'IMAGE_TOO_LARGE'
  | 'INVALID_PARAMS' | 'TRACE_FAILED' | 'EXPORT_FAILED' | 'WRITE_FAILED'
  | 'CANCELLED' | 'PRESET_DUPLICATE' | 'PRESET_READ_ONLY' | 'NOT_FOUND'
  | 'ACCESS_DENIED' | 'SOURCE_CHANGED' | 'BATCH_BUSY' | 'INVALID_STATE'
  | 'OUTPUT_TOO_LARGE' | 'DATA_VERSION_UNSUPPORTED' | 'DATA_CORRUPT';
```

Message default Indonesia; frontend dapat memetakan code ke locale aktif dan tetap menyertakan konteks aman. details hanya log. Cancel dialog tidak ditampilkan sebagai kegagalan. Error menunjukkan file/format yang gagal, bukan menggagalkan semua output tanpa penjelasan.
