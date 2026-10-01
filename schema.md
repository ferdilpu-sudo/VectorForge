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

SavePreset v1 membuat baru; tidak ada update built-in atau overwrite nama diam-diam. Built-in IDs stabil: builtin-balanced, builtin-logo, builtin-photo, builtin-poster. Seimbang default; Logo & Flat override colorPrecision=4/filterSpeckle=8/layerDifference=32; Foto Detail =8/2/8; Poster Halus colorPrecision=7/cornerThreshold=90/mode=spline. Field lainnya selalu default.

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
