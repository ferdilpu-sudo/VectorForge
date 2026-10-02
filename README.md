# VectorForge

VectorForge adalah aplikasi desktop Windows offline untuk mengubah PNG, JPEG, WebP, dan BMP menjadi output vektor berwarna SVG, PDF, dan EPS. Frontend React/TypeScript berjalan di shell Tauri 2 dan engine tracing/export berada di Rust. Tidak ada upload gambar atau layanan jaringan runtime.

## Status implementasi

Per 2 Oktober 2026:
- frontend P1/G1: DONE;
- backend P2 B01–B06: DONE;
- integrasi native I01: DONE;
- walkthrough desktop I02: DONE;
- H01 hardening/benchmark/accessibility: DOING;
- installer H02 dan gate rilis G3: belum dimulai.

I02 telah diuji pada Windows untuk native import, drag/drop, preview tracing nyata, perubahan parameter, pergantian source saat preview, export SVG/PDF/EPS, membuka folder output, batch, cancel, serta failure → retry.

## Prasyarat pengembangan

- Windows 10/11;
- Node.js 22.12+ atau 24;
- npm;
- Rust stable + Cargo;
- Microsoft WebView2 Runtime.

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

Script melaporkan OS, CPU, jumlah logical processor, RAM, ukuran executable, startup median/p95, dan idle working-set median/p95. Angka benchmark hanya dianggap bukti setelah dijalankan pada Windows nyata dan dicatat di `plan.md`.

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

## Belum release-ready

VectorForge belum dinyatakan release-ready sampai H01, H02, dan G3 selesai. Installer production, signing status, benchmark final, accessibility audit, serta packaging smoke test masih harus dibuktikan.
