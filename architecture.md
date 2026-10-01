# Architecture — VectorForge

## Stack dan batas modul

Tauri 2/Rust menangani file, decode, tracing, export, queue dan persistence. React 18/TypeScript strict/Tailwind/Zustand menangani tampilan dan state interaksi. vtracer (+visioncortex bila diperlukan langsung) untuk tracing, image untuk decode, usvg/svg2pdf untuk PDF, writer EPS khusus output solid path. Exact versions dan API diverifikasi B01 lalu dikunci; jangan menambah dependency transitif langsung tanpa kebutuhan.

Frontend menggunakan `services/api.ts` sebagai facade typed. Implementasi `mock-engine.ts` pada P1 dan `tauri-adapter.ts` pada P3 memenuhi interface yang sama. Komponen tidak memanggil invoke atau filesystem langsung. Mock dipilih konfigurasi eksplisit dan dilabeli; kegagalan Rust tidak boleh fallback ke mock.

## Struktur tanggung jawab yang ditargetkan

| Lokasi | Pemilik tanggung jawab |
|---|---|
| src/core | Komposisi aplikasi, providers, locale |
| src/features/dropzone | Import UI dan feedback file |
| src/features/compare | Canvas, transform, pembatas, result info |
| src/features/params | Parameter/preset UI |
| src/features/batch | Antrean, status dan kendali batch |
| src/features/export | Dialog single export dan notifikasi |
| src/features/settings | Pengaturan aplikasi |
| src/services | Facade, mock, Tauri adapter dan event lifecycle |
| src/stores | Project state, batch state; tanpa engine/I/O langsung |
| src/types | Kontrak yang sesuai schema |
| src/shared | Komponen/pure utility yang benar-benar lintas fitur |
| src/styles | Tokens, global base; styles fitur dekat pemiliknya |
| src-tauri/src/commands | IPC tipis, validasi boundary, delegasi service |
| src-tauri/src/files | Registry fileId, grant akses, fingerprint metadata |
| src-tauri/src/engine | Decode/normalize, cache preview, adapter tracer, scheduling |
| src-tauri/src/export | SVG/PDF/EPS writer, output reservation, atomic commit |
| src-tauri/src/batch | Queue lifecycle, worker orchestration, event snapshots |
| src-tauri/src/store | Settings/presets versioned persistence |
| src-tauri/src/models | DTO, enums, errors |
| tests/fixtures | Dataset kecil deterministik, asal/lisensi fixture dicatat |

File dipecah berdasar tanggung jawab; tabel bukan perintah membuat file placeholder untuk semua kemungkinan.

## Import dan keamanan path

Dialog/drop native menjadi sumber otorisasi path. `import_files` memverifikasi path dalam grant dialog/drop yang diperoleh Rust/Tauri integration, canonicalize, header, dimensi dan format. Simpan path canonical di registry Rust, kembalikan fileId opaque plus metadata UI. Path pengguna tidak menjadi capability hanya karena dikirim lewat invoke. Mekanisme grant aktual wajib dipastikan sesuai API Tauri terpilih pada B02; custom dialog command dapat menjaga path tetap di Rust.

Frontend menerima thumbnail/URL lokal yang scoped, bukan akses seluruh disk. Release fileId menghapus registry/cache/object URLs terkait setelah tidak dirujuk job. Export folder/file dipilih native, direpresentasikan destinationId opaque dengan capability tulis terbatas. Buka folder memakai hasil job yang dikenal, tidak generic shell execute.

## Preview

Import → normalized thumbnail asli → parameter debounce 300 ms → generate_preview → bounded blocking scheduler → decode/resize/trace → SVG string dan stats → blob img.

Satu sesi preview aktif, satu pekerjaan pending terbaru; request baru mengganti pending dan menandai aktif obsolete. RequestId dicek sebelum render. Cache key: fileId + fingerprint + orientation/color policy + maxSide. Cache terbatas satu gambar decoded preview dan hasil terbaru, bukan kumpulan tanpa batas. Jangan decode full-res berulang tiap slider tick.

Jika engine synchronous tidak menyediakan cancel callback, biarkan pemanggilan aktif kembali, abaikan hasilnya dan baru proses pending terbaru. `spawn_blocking` sendiri tidak memberi kemampuan membunuh pekerjaan. Jangan menjanjikan instant cancellation. Preview mendapat prioritas pada slot worker berikutnya; bukan preemption worker yang sudah aktif.

## Export

Snapshot source fingerprint + params + destination + format → validate/decode sumber penuh → trace → SVG. SVG langsung ditulis; PDF dari parsed SVG; EPS solid paths dari hasil trace sumber yang sudah dikomposit putih bila perlu. Terapkan transform/fill rule/orientation/bounds dengan benar. Jangan render raster ke PDF sebagai fallback.

96 px = 72 pt untuk halaman PDF; EPS bounding box konsisten. Unsupported node/style menghasilkan error. Statistik trace pada ExportResult adalah hasil tracing penuh; bytes adalah ukuran format yang disimpan.

Periksa fingerprint sebelum membaca dan setelah decode; file yang berubah menghasilkan SOURCE_CHANGED, bukan hasil cache lama. Temporary file unik di direktori output, commit aman setelah lengkap. Reserve nama secara eksklusif dan retry suffix saat race; overwrite hanya setelah izin, tidak unlink target lama sebelum replacement siap. Cancel/error membersihkan temporary file milik job saja.

## Batch dan resource budget

Satu batch aktif. Snapshot file IDs/fingerprint, params, formats, output destination, overwrite policy dan batas ukuran. Worker pool terpusat; jangan menumpuk rayon tak terbatas di atas spawn_blocking. Maks concurrent pekerjaan berat min(workerCount, 4, cores), dapat turun karena estimasi memori decoded buffer/tracing. Preview/export single berbagi scheduler yang sama.

Dalam satu item, trace SVG sekali untuk SVG/PDF jika input policy sama; EPS transparan memerlukan trace terpisah. Format diproses berurutan untuk membatasi memori. Simpan hasil per format sehingga sukses SVG tetap dicatat bila PDF gagal. Status partial berarti paling sedikit satu format berhasil dan satu gagal.

Cancellation menghentikan job baru, menandai queued cancelled dan menunggu aktif sampai titik aman; cek sebelum decode/trace/export/commit. File sukses tidak dihapus. Item aktif cancelled dapat memiliki outputs done yang sudah committed. Retry hanya saat batch terminal, menggunakan snapshot awal dan output gagal; bila sumber berubah, minta import/batch baru. Retry memulai runId baru agar event terminal lama tidak mengakhiri run baru.

Events full snapshot dengan sequence monoton per run, dibatasi frekuensi (misalnya maksimal 10/s, terminal selalu dikirim). Frontend subscribe sebelum start, filter batchId/runId/sequence, dan dapat resync lewat get_batch. Done tidak dipancarkan sebelum semua worker terminal dan output committed/cleaned.

## Storage

JSON settings/presets di app data dir yang diresolve Tauri untuk identifier com.vectorforge.app. Jangan hardcode asumsi APPDATA. Serialize atomic temp → replace Windows, satu writer lock. Baca sebelum write; versi lebih baru tidak boleh ditimpa, tampilkan incompatibility. File corrupt dipertahankan/backup sebelum reset dengan persetujuan. Preset built-in di kode; hanya user presets disimpan.

## Runtime keamanan dan build

CSP production membatasi self dan image blob/data seperlunya; tanpa remote scripts/fonts. Capabilities dialog/drop/scoped asset/open-folder saja, tidak fs wildcard/shell arbitrary. Log Rust terstruktur dan rotasi. Frontend menampilkan AppError manusiawi, details hanya log.

Build Windows menghasilkan NSIS exe dan MSI bila toolchain tersedia. Signing opsional belum diputuskan; tanpa signing jangan menjanjikan bebas warning. WebView2 downloadBootstrapper dapat membutuhkan internet saat instalasi. CI/typecheck Linux tidak membuktikan Windows installer berjalan.

## Command/event registry

Nama dan payload kanonis ada di schema. Command: import_files, release_files, choose_destination, generate_preview, cancel_preview, export_file, start_batch, get_batch, cancel_batch, retry_batch_item, list_presets, save_preset, delete_preset, get_settings, save_settings, open_output_folder. Events: batch://progress, batch://done. Command tambahan membutuhkan perubahan schema dan architecture sebelum implementasi.
