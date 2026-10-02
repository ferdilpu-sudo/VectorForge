# VectorForge

VectorForge adalah aplikasi desktop Windows offline untuk mengubah PNG, JPEG, WebP, dan BMP menjadi output vektor berwarna SVG, PDF, dan EPS. Frontend React/TypeScript berjalan di shell Tauri 2 dan engine tracing/export berada di Rust. Tidak ada upload gambar atau layanan jaringan runtime.

## Status implementasi

Per 2 Oktober 2026:
- frontend P1/G1: DONE;
- backend P2 B01–B06: DONE;
- integrasi native I01: DONE;
- walkthrough desktop I02: DONE;
- H01 hardening/benchmark/accessibility: DONE;
- installer H02 dan gate rilis G3: belum dimulai.

I02 telah diuji pada Windows untuk native import, drag/drop, preview tracing nyata, perubahan parameter, pergantian source saat preview, export SVG/PDF/EPS, membuka folder output, batch, cancel, serta failure → retry.

## Prasyarat pengembangan

- Windows 10/11;
- Node.js 22.12+ atau 24;
- npm;
- Rust stable + Cargo;
- Microsoft WebView2 Runtime;
- untuk H02 packaging: `tauri-cli 2.12.0`.

Versi dependency frontend dikunci di `package-lock.json`; dependency Rust dikunci di `src-tauri/Cargo.lock`.

## Menjalankan aplikasi development

Tauri memakai Vite development server. Jalankan dari root repository:

```powershell
npm ci
npm run dev
```

Di terminal kedua:

```powershell
cd src-tauri
cargo run
```

## Gate source

Frontend:

```powershell
npm run lint
npm test
npm run h01:contrast
npm run build
```

Rust:

```powershell
cd src-tauri
cargo check
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt -- --check
```

Verifikasi proyek dilakukan lokal pada Windows, bukan melalui GitHub Actions.

## Benchmark H01

Buat frontend production dan executable release terlebih dahulu:

```powershell
npm run build
cd src-tauri
cargo build --release
cd ..
```

Lalu ukur startup-to-main-window dan idle working set sebanyak 10 run:

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\h01-startup-benchmark.ps1
```

Script melaporkan OS, CPU, jumlah logical processor, RAM, ukuran executable, startup median/p95, dan idle working-set median/p95.

Benchmark preview production pipeline dengan file nyata:

```powershell
$env:VECTORFORGE_BENCH_IMAGE="C:\path\to\representative-image.png"
$env:VECTORFORGE_BENCH_ITERATIONS="10"
cd src-tauri
cargo test --release h01_real_image_preview_benchmark -- --ignored --nocapture
cd ..
```

Untuk memory process-tree, jalankan executable release, lalu di terminal lain:

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\h01-memory-watch.ps1 -Seconds 60
```

Selama watcher aktif, lakukan skenario yang sedang diukur, misalnya satu source sekitar 20 MP atau batch besar. Angka benchmark hanya dianggap bukti setelah dijalankan pada Windows nyata dan dicatat di `plan.md`.

## H02 — Build installer Windows

H02 memakai Tauri CLI `2.12.0` agar toolchain bundler sesuai dengan runtime Tauri `2.12.0`. Instal sekali pada Windows bila belum tersedia:

```powershell
cargo install tauri-cli --version 2.12.0 --locked
```

Dari root repository, jalankan build reproducible:

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\h02-build-windows.ps1
```

Script menjalankan source gates frontend/Rust terlebih dahulu, lalu membangun NSIS dan MSI secara terpisah. Setelah selesai script melaporkan path artefak, ukuran MiB, apakah memenuhi target installer <25 MiB, SHA-256, dan status Authenticode.

Konfigurasi Windows H02:
- target bundle: NSIS dan MSI;
- NSIS install mode: current user, tanpa meminta hak Administrator untuk jalur normal;
- WebView2: `downloadBootstrapper`, sehingga runtime tidak disertakan dalam ukuran installer; pada mesin yang belum memiliki WebView2, installer dapat membutuhkan internet;
- icon bundle saat ini memakai `src-tauri/icons/icon.ico`; file ini masih icon bootstrap teknis dan belum dianggap keputusan branding final.

MSI memakai WiX dan pada Windows dapat membutuhkan optional feature VBSCRIPT. Jika build MSI gagal pada `light.exe`, aktifkan VBSCRIPT melalui Windows Optional Features lalu ulangi build.

### Smoke test installer

Uji NSIS terlebih dahulu, lalu MSI bila build MSI tersedia:

1. install VectorForge;
2. jalankan aplikasi dari hasil instalasi, bukan executable di `target\release`;
3. import satu gambar nyata;
4. buat preview;
5. export SVG ke folder pengguna di luar folder instalasi, misalnya Documents;
6. tutup aplikasi;
7. uninstall VectorForge dari Windows;
8. pastikan aplikasi terhapus tetapi file SVG hasil export pengguna tetap ada;
9. ulangi alur dengan installer MSI bila tersedia.

H02 selesai pada 2 Oktober 2026: NSIS dan MSI berhasil dibangun, keduanya di bawah target 25 MiB, smoke test install/run/import/preview/export/uninstall lulus, dan file hasil pengguna tetap ada setelah uninstall. Kedua installer masih `NotSigned`; signing tetap menjadi status rilis yang harus dinyatakan jujur.

## Struktur dan sumber kebenaran

Agent membaca `AGENTS.md`, lalu `rules.md`, lalu `plan.md`. Dokumen domain:
- `prd.md`: acceptance produk;
- `design.md`: UI/UX dan accessibility;
- `architecture.md`: modul dan boundary;
- `schema.md`: kontrak IPC/data;
- `testing.md`: matriks verifikasi;
- `decisions.md`: keputusan teknis;
- `plan.md`: status task dan bukti aktual.

Kode frontend berada di `src/`, backend native di `src-tauri/`, dan spike dependency historis B01 berada di `spikes/b01-engine/`.

## Kebijakan penting

- runtime offline;
- path filesystem tidak dipercaya dari JavaScript tanpa native grant/registry;
- preview source memakai local protocol registry-gated berbasis opaque `fileId`;
- SVG/PDF harus tetap vektor, bukan raster yang dibungkus;
- EPS flatten transparansi ke putih sesuai keputusan v1;
- output memakai commit/replace aman dan tidak boleh merusak input;
- mock/demo tidak menjadi fallback produksi.

## Status release

H01 dan H02 selesai. G3 release-readiness audit sedang berjalan.

Yang sudah terbukti pada Windows nyata: alur desktop produksi, hardening/performance/accessibility, NSIS+MSI build, install/run/import/preview/export/uninstall, serta user-file retention setelah uninstall.

Yang masih harus ditutup sebelum klaim release-ready penuh:
- final verification setelah dua fix G3 (last output directory dan logical-core worker cap), lalu rebuild installer;
- lisensi aplikasi VectorForge belum ditentukan pemilik;
- installer masih Authenticode `NotSigned`;
- Windows 10 target belum memiliki distribution smoke evidence; Windows 11 sudah diuji;
- compatibility output browser/Inkscape/Illustrator belum seluruhnya memiliki bukti final;
- icon bundle saat ini masih bootstrap teknis, bukan branding final.

Dependency license audit sudah lulus untuk Rust yang diaudit pada fase backend dan untuk 362 package eksternal NPM pada G3 (0 package tanpa deklarasi license).
