# VectorForge — Pedoman Proyek

Revisi dokumen: 1.1 • 30 September 2026 • Bahasa: Indonesia.

VectorForge adalah aplikasi desktop Windows offline untuk mengubah PNG, JPEG, WebP, dan BMP menjadi vektor berwarna SVG, PDF, dan EPS. Paket ini adalah spesifikasi implementasi, **bukan aplikasi yang sudah dibuat**. Tidak ada klaim build, tes, benchmark, maupun installer lulus.

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

Pedoman fase selanjutnya tetap berlaku. `npm run tauri dev/build` belum tersedia. Engine, grant native file, full-resolution export, worker paralel dan penyimpanan JSON Rust belum dibuat.

## Verifikasi

Build TypeScript/Vite dan lint lulus. Delapan tes validasi dan lifecycle preview/batch lulus pada environment Linux/Node 24. Tes ini tidak membuktikan aplikasi native Windows berjalan.

Percobaan browser automation terblokir: agent-browser daemon gagal start, Chromium lokal tidak tersedia. Visual, DPI Windows, keyboard walkthrough dan pengujian aplikasi grafis eksternal berstatus NOT RUN. Gate G1 menunggu review UI, belum menjadi izin backend.

## Batas demo yang perlu ditutup sebelum G1 final

- Browser decode memeriksa signature dan batas 30 MP setelah decode; pengecekan dimensi pra-decode, format animasi, profil warna dan grant file di Rust tetap B02/B03.
- Teks label utama tersedia ID/EN; sebagian pesan validasi masih bilingual dan nama preset bawaan tetap Indonesia.
- Kontrak desktop memakai fileId/destinationId. Browser adapter memakai File dan object URL secara eksplisit; native adapter belum dibuat.
- Batch demo berjalan sekuensial dengan delay simulasi, bukan benchmark workerCount/engine.
- Preview illustrative tidak memiliki statistik tracing riil; jangan memakai demo untuk penilaian kualitas vektor.
- Reset pengaturan dengan konfirmasi dan kontrol tinggi/collapse antrean tersedia.

## Struktur kode

`src/core` komposisi; `src/features` UI per fitur; `src/services` adapter demo/import/preferences; `src/stores` state proyek; `src/types` model; `src/shared` modal dan locale; `src/styles` tema/layout; `src/tests` tes risiko. Satu file satu tanggung jawab tanpa limit baris.

## Langkah berikutnya

Review frontend dan tutup item G1 yang belum terverifikasi. Setelah persetujuan frontend, lanjut B01 untuk validasi dependency/engine/transparansi/EPS, bukan langsung mengklaim dukungan seluruh format. Lisensi aplikasi belum ditentukan pemilik; belum ada rilis publik atau installer.
