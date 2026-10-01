# Testing — Bukti Kualitas dan Rilis

Status awal: seluruh tes implementasi belum dijalankan. Checklist ini adalah rencana, bukan laporan lulus. Simpan hasil aktual di plan task log; fixture provenance/lisensi dicatat di tests/fixtures saat dibuat.

## Dataset wajib

Logo flat 2–8 warna; logo dengan hole; ilustrasi detail; foto; sumber alpha nol; antialias alpha parsial; semua format input; EXIF rotation; non-square; 1×1; sumber di bawah ukuran preview; 20 MP; >30 MP; file animasi; file corrupt; ekstensi palsu; nama Unicode/spasi/panjang; dua sumber bernama sama dari folder berbeda. Tambahkan file gradient untuk mengamati kualitas tracing, bukan mengharapkan gradient vector output yang belum dijanjikan.

## Matriks pengujian

| Tes | Bukti yang dicari | Fase |
|---|---|---|
| Params boundary/non-finite/enum invalid | Validasi TS/Rust setara, tidak clamp diam-diam | F04/B02 |
| Serialization DTO fixture | camelCase/enums/optionals benar, command wrapper konsisten | B02/I01 |
| Import mixed valid/invalid | Valid tetap masuk, error per file, path grant ditegakkan | F03/B02 |
| Latest preview A lambat/B cepat | A tidak menimpa B, hanya pending terbaru diproses, zoom tidak reset | F04/B03 |
| Switch source saat preview | Tidak ada hasil/metadata silang antar file | B03/I02 |
| Raster decode normalization | Orientasi, dimensi, profil warna kebijakan konsisten | B03 |
| SVG vector authenticity | Path nyata, tidak menyisipkan image raster untuk berpura-pura vektor | B04 |
| SVG/PDF alpha | Area transparan dan tepi semi-transparent sesuai kebijakan; blocker jika tidak | B01/B04 |
| EPS correctness | Hole/fill rule, transform, Y-axis, bounds, putih untuk alpha, vektor nyata | B01/B04 |
| Full resolution | Export stats/viewBox/page dimensions cocok sumber, bukan preview 1024 | B04 |
| File overwrite/collision | Nama reserve aman, suffix tidak race, input tidak berubah | B04/B06 |
| Failure recovery | Disk penuh, readonly, file terkunci, source hilang/berubah, temp cleanup | B04/I02 |
| Partial batch output | SVG sukses/PDF gagal → partial, retry hanya gagal | B06 |
| Cancel dan retry | Tidak mulai queued baru, tunggu aktif aman, output done bertahan, runId baru | B06/I02 |
| Events lifecycle | Subscribe/resync, sequence filter, stale run ignored, no duplicate listener | I01 |
| Storage | Corrupt/newer version/built-in deletion/duplicate preset/atomic failure | B05 |
| UI states | Empty/loading/error/success/disabled dan teks panjang dua locale | F02–F05 |
| Accessibility | Keyboard full flow, alternative drag, focus modal/slider, kontras aktual | H01 |
| Packaging | Windows install, start, import, export, uninstall; user files tidak dihapus | H02 |

Tes logika murni unit; tes engine/export dengan fixture integration. Screenshot/golden jangan terlalu rapuh terhadap metadata/path ordering; periksa struktur vector, bounds dan render visual dengan toleransi yang dicatat. Golden update hanya bila perubahan disengaja, tidak untuk menutupi regresi.

## Visual/UX checks

1366×768 pada Windows 100/125/150%; 1920×1080; jendela diperkecil; tema dark/light/system; panel kiri folded; batch kosong/1/50 item; nama panjang; modal dan keyboard. Drag split nyaman, tidak menyebabkan trace baru; sumber dan hasil sejajar saat zoom/pan. Ukur kontras teks normal target 4.5:1, teks besar 3:1, indikator kontrol/fokus relevan 3:1. Angka ini target desain; klaim compliance memerlukan audit implementasi sesuai standar resmi saat rilis.

Lakukan walkthrough pemula: buka logo → pilih preset → ekspor SVG tanpa bantuan; catat titik kebingungan, waktu interaksi dan waktu engine terpisah. Bandingkan contoh output di browser (SVG), Inkscape, Illustrator untuk format yang didukung; jangan mengklaim uji aplikasi yang tidak tersedia.

## Benchmark reproducible

Build release pada Windows, catat CPU/core/RAM/OS/WebView2/versi dependency. Startup: definisikan start process → UI siap input. Preview: catat decode/resize, trace, end-to-end; cold dan warm terpisah. Minimal 10 pengukuran per fixture representatif, laporkan median dan p95 serta dimensi. Memory: catat peak process tree single 20 MP dan batch 50×2 MP terpisah. Installer size tanpa dan dengan kebutuhan WebView2 dijelaskan. Catat responsive UI selama batch dan apakah scheduler menurunkan concurrency.

Target PRD tidak menjadi alasan mematikan validasi atau menurunkan resolusi ekspor. Jika target gagal, laporkan fixture/hardware dan bottleneck; optimasi hanya setelah ada hasil ukur.

## Gate rilis

- Semua acceptance PRD terpetakan ke bukti; blocker alpha/EPS/OS terselesaikan.
- Frontend build/typecheck/lint serta tes relevan lulus.
- Rust fmt/test/clippy lulus, tidak ada panic pada input buruk.
- Tidak ada mock adapter/fake stats di mode production.
- Tidak ada network runtime, CSP/capabilities terverifikasi.
- Output input untouched, cancellation safe, error recovery benar.
- Installer Windows diuji; status signing dan prasyarat WebView2 jelas.
- Dependency licenses diperiksa; lisensi aplikasi ditentukan pemilik sebelum distribusi, jangan mengarang lisensi.
- README implementasi berisi versi/prasyarat/perintah yang benar-benar dijalankan, known limitations, cara memakai app.

Laporan akhir membedakan PASS, FAIL, NOT RUN. Release readiness tidak sama dengan izin mempublikasikan atau mengunggah installer ke layanan eksternal.
