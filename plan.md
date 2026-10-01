# Plan — Urutan Kerja dan Kendali Scope

Status 1 Oktober 2026: P0 selesai, implementasi awal P1 tersedia. F01 selesai dengan build/lint; F02–F05 terimplementasi sebagian dan menunggu visual/manual review serta item backlog README. G1 belum lulus. Backend P2 dan seterusnya TODO.

## Backlog dan gate

| ID | Fase / task | Dependensi | Bukti selesai |
|---|---|---|---|
| P0 | Selaraskan paket pedoman | Dokumen asli + arahan user | Dokumen, referensi internal, kontrak dan keputusan konsisten |
| F01 | Scaffold React/TS, scripts, tokens, types | Izin mulai P1 | Dev frontend, lint/typecheck/build berjalan |
| F02 | Shell, toolbar, panel, canvas states | F01 | Layout sesuai design, keyboard dan DPI diperiksa |
| F03 | Import mock/browser, validation, compare zoom/pan | F02 | Single/multiple, invalid, transform bersama, URL cleanup |
| F04 | Parameter, preset dan settings demo | F03 | Input range, debounce, stale result, reset, nama duplikat |
| F05 | Export dialog, batch dan error simulation | F04 | Semua state, retry/cancel/partial, label demo jujur |
| G1 | Review frontend | F01–F05 | Demo/screenshot, checklist, keterbatasan; persetujuan user |
| B01 | Spike dependency/engine/export | G1 disetujui | Versi + lisensi; uji alpha, cancellation, PDF/EPS; keputusan terdokumentasi |
| B02 | Tauri shell, file registry, models, errors | B01 | IPC deserialize/validation, otorisasi file, capabilities |
| B03 | Decode + preview scheduler + vtracer adapter | B02 | Fixture visual dan resource limits, latest-wins |
| B04 | Export SVG/PDF/EPS | B03 | Output vektor nyata, transparansi, bounds, disk safety |
| B05 | Settings/preset storage | B02 | Atomic update, version mismatch, corrupt file recovery |
| B06 | Batch scheduler, cancel/retry, events | B04 | IDs stabil, progress benar, partial output, collision handling |
| G2 | Review backend | B03–B06 | Hasil fixture, kontrak, cargo checks; persetujuan user |
| I01 | Adapter IPC produksi + event lifecycle | G2 disetujui | Mock bukan fallback diam-diam, listeners cleanup |
| I02 | Uji alur desktop lengkap | I01 | Import → parameter → preview → export; batch recovery |
| H01 | Hardening, benchmark, accessibility | I02 | Matriks testing, angka aktual dan issue tersisa |
| H02 | Build installer Windows dan smoke test | H01 | NSIS/MSI bila toolchain tersedia, install/run/uninstall |
| G3 | Release readiness | H02 | Bukti gate rilis; signing/lisensi/OS dicatat |

## Batas fase

P1 menggunakan mock engine deterministic, bukan tracing Rust. Browser File API untuk demo boleh, tetapi file browser tidak dianggap memiliki path desktop yang valid. Siapkan antarmuka adapter sesuai architecture agar fase integrasi tidak merombak komponen.

P2 boleh menguji Rust melalui harness/tes tanpa mengintegrasikan UI produksi. Spike B01 adalah kewajiban verifikasi, bukan izin mengganti fitur/stack. Jika alpha penuh atau EPS tidak dapat memenuhi kontrak, dokumentasikan reproduksi dan usulan, minta keputusan perubahan; jangan menurunkan acceptance diam-diam.

P3 menghubungkan alur lengkap. P4 hanya hardening dalam scope; ide baru masuk deferred list di decisions. Jangan mengerjakan deferred karena “mudah sekalian”.

## Definition of Ready sebuah task

Tujuan pengguna jelas, acceptance test jelas, kontrak tersedia, izin fase valid, dependency task selesai atau ada alasan bisa berjalan independen. Mulai task terkecil yang memenuhi ini; tidak perlu menunggu keputusan kosmetik rutin.

## Log pelaksanaan (diisi agent)

| Tanggal | Task | Status TODO/DOING/BLOCKED/DONE | Perubahan & bukti | Blocker / next task |
|---|---|---|---|---|
| 2026-09-30 | P0 | DONE | Paket pedoman revisi 1.1 disusun; belum ada implementasi | Berikutnya F01 setelah izin mulai frontend |

## Catatan izin fase

- 1 Oktober 2026: user memerintahkan eksekusi ke repository ferdilpu-sudo/VectorForge; izin meliputi memasukkan pedoman dan memulai frontend P1. Tidak ada persetujuan melampaui checkpoint G1/G2.
- Tidak ada izin tercatat melewati G1/G2.
- Isi tanggal, pesan persetujuan dan cakupan nyata ketika izin diterima. Jangan mengisi asumsi sebagai persetujuan.

## Target performa

Target PRD adalah sasaran, bukan fakta: startup <2 s, preview tipikal 1024 px <1,5 s, installer <25 MB tanpa WebView2 runtime, memori <500 MB untuk skenario satu sumber 20 MP. Ukur dengan hardware, dataset, build release dan metode di testing. Batch paralel tidak dianggap otomatis memenuhi target memori single. Kebenaran output dan stabilitas lebih tinggi prioritas daripada angka target; kegagalan target harus dilaporkan, bukan disembunyikan dengan menurunkan kualitas.

## Hasil eksekusi P1 awal — 1 Oktober 2026

- F01 DONE: scaffold React 18/TypeScript/Tailwind/Zustand; package-lock; dev/build/lint/test.
- F02–F05 DOING: UI dan demo workflow tersedia; delapan unit/hook tests lulus.
- Teruji: boundary parameter, nama preset, cancel request, stale preview cleanup, manual preview, partial-output retry dan cancel batch.
- Visual/DPI/browser walkthrough BLOCKED: daemon agent-browser gagal start; Chromium tidak terpasang. Tidak mengklaim screenshot atau manual Windows pass.
- TODO sebelum G1 final: penyelesaian locale error, visual dan keyboard manual review. Reset settings, tinggi antrean dan penolakan PNG/WebP animasi sudah ditambahkan. Native pre-decode validation tetap P2.
- Berikutnya: review frontend di Windows dengan npm ci / npm run dev. Backend tidak dimulai.
