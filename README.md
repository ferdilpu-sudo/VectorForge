# VectorForge — Pedoman Proyek

Revisi dokumen: 1.1 • 30 September 2026 • Bahasa: Indonesia.

VectorForge ditujukan sebagai aplikasi desktop Windows offline untuk mengubah PNG, JPEG, WebP, dan BMP menjadi vektor berwarna SVG, PDF, dan EPS. Saat ini repository sudah memiliki **frontend demo yang berjalan di browser**, sedangkan shell Tauri, engine tracing Rust, ekspor native, dan installer Windows belum dibuat. Dokumen proyek membedakan kemampuan demo dari kemampuan produksi agar status implementasi tidak terkesan lebih maju daripada kenyataannya.

## Mulai di sini

Agent membaca [AGENTS.md](AGENTS.md), [rules.md](rules.md), lalu [plan.md](plan.md). Baca dokumen domain sesuai tugas sebelum mengubah kode. Jangan membaca pedoman umum lama sebagai alasan untuk menimpa keputusan proyek ini.

| Dokumen | Tanggung jawab / sumber kebenaran |
|---|---|
| [AGENTS.md](AGENTS.md) | Protokol kerja agent dan checkpoint |
| [rules.md](rules.md) | Aturan kode, scope, perubahan dan keamanan |
| [prd.md](prd.md) | Produk, batas v1, acceptance criteria |
| [design.md](design.md) | Tampilan, interaksi, state, aksesibilitas |
| [architecture.md](architecture.md) | Modul, alur data, batas frontend/Rust |
| [schema.md](schema.md) | Kontrak IPC dan data lokal |
| [plan.md](plan.md) | Fase, dependensi, kriteria keluar dan status |
| [testing.md](testing.md) | Skenario verifikasi dan rilis |
| [decisions.md](decisions.md) | Keputusan tetap, alasan, risiko terbuka |

## Menjalankan frontend demo

Prasyarat: Node.js 22.12+ atau 24 dan npm. Versi dependency dikunci dalam package-lock.json.

```bash
npm ci
npm run dev
```

Buka URL lokal dari Vite. Demo berjalan di browser; belum ada shell Tauri atau installer. Gambar dipilih memakai File API browser. Tidak ada gambar diunggah ke server.

```bash
npm run lint
npm run test
npm run build
```

## Status implementasi — 1 Oktober 2026

Frontend demo tersedia: import gambar, panel parameter/preset, perbandingan dan zoom/pan, pengaturan tema/bahasa, dialog ekspor simulasi dan batch simulasi dengan cancel/retry per format. Preview sengaja menampilkan ilustrasi berlabel DEMO, **bukan hasil tracing gambar input**. Simulasi tidak menulis file hasil. Preferences demo memakai localStorage, bukan app data Windows.

Gate frontend G1 sudah disetujui setelah review visual Windows dan verifikasi lokal. B01 kini berjalan sebagai spike Rust **terisolasi**; `npm run tauri dev/build` tetap belum tersedia. Engine produksi, grant native file, full-resolution export, worker paralel dan penyimpanan JSON Rust belum dibuat.

## Verifikasi

Verifikasi proyek dilakukan **secara lokal**, bukan melalui GitHub Actions. Pada Windows pengguna, lint lulus, 3 test files / 10 tests lulus, dan build Vite produksi lulus setelah polish terbaru. Screenshot light-mode dengan gambar aktif dan batch queue juga ditinjau langsung pengguna. Tes frontend ini tetap tidak membuktikan shell native Windows berjalan.

Percobaan browser automation pada environment agent sebelumnya terblokir, sehingga tidak ada klaim matriks DPI/keyboard otomatis. Keterbatasan itu dicatat, tetapi pengguna menyetujui hasil G1 dan memerintahkan lanjut ke B01.

## Batas demo yang dibawa ke fase native

- Browser decode memeriksa signature, menolak APNG/animated WebP, dan menerapkan batas 30 MP setelah decode; pengecekan dimensi pra-decode, profil warna, dan grant file di Rust tetap B02/B03.
- Teks UI utama, error import demo, dan label preset bawaan mengikuti pilihan ID/EN. Kontrak native tetap harus menangani locale/error code secara terstruktur saat adapter Rust dibuat.
- Kontrak desktop memakai fileId/destinationId. Browser adapter memakai File dan object URL secara eksplisit; native adapter belum dibuat.
- Batch demo berjalan sekuensial dengan delay simulasi, bukan benchmark workerCount/engine.
- Preview illustrative tidak memiliki statistik tracing riil; jangan memakai demo untuk penilaian kualitas vektor.
- Reset pengaturan dengan konfirmasi dan kontrol tinggi/collapse antrean tersedia.

## Struktur kode

`src/core` komposisi; `src/features` UI per fitur; `src/services` adapter demo/import/preferences; `src/stores` state proyek; `src/types` model; `src/shared` modal dan locale; `src/styles` tema/layout; `src/tests` tes risiko. Satu file satu tanggung jawab tanpa limit baris.

## B01 engine/export spike

Harness Rust ada di `spikes/b01-engine` dan dijelaskan di [b01-engine-spike.md](b01-engine-spike.md). Harness ini menguji kandidat VTracer, cancellation/progress, SVG→PDF, writer EPS dan karakterisasi alpha. Ia **bukan** `src-tauri` produksi dan tidak dihubungkan ke React.

```powershell
cd spikes/b01-engine
cargo check
cargo run
cargo tree
```

Source inspection awal menemukan bahwa stock SVG writer VTracer 1.0.0-alpha.4 masih memakai fill RGB solid, sehingga alpha parsial SVG/PDF adalah blocker D10 sampai dibuktikan dan diputuskan secara eksplisit. Jangan flatten diam-diam.

## Langkah berikutnya

Jalankan harness B01 pada Windows, catat dependency tree/lisensi transitif, uji filesystem atomic replace, lalu tutup keputusan alpha. B02 baru boleh dimulai setelah B01 benar-benar DONE. Lisensi aplikasi VectorForge sendiri belum ditentukan pemilik; belum ada rilis publik atau installer.


## Native backend status

- G1 frontend demo: **DONE**
- B01 engine/export dependency spike: **DONE**
- B02 production Tauri/backend shell: **DONE**

B01 passed Windows release characterization for SVG/PDF/EPS, cancellation, alpha-preserving split-mask, resource guard characterization, and safe Windows file replacement. Its Cargo.lock is committed and Cargo metadata reported no external/transitive package missing a license declaration. The local non-published spike package intentionally has no application license declaration yet.


### B02 scope aktif

B02 membuat shell Tauri 2 dan boundary file native saja: DTO/error Rust, file/destination registry opaque, validasi metadata input, scope authorization dan capability minimum. Frontend tetap memakai adapter demo sampai fase integrasi; tracing/preview produksi belum dimulai.
