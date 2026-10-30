# Testing — Bukti Kualitas dan Rilis

Dokumen ini adalah metodologi dan gate pengujian. Hasil aktual tidak disimpulkan dari checklist ini; sumber status dan evidence nyata berada di `plan.md`. Per 2 Oktober 2026, implementasi telah melewati H01/H02 dan sedang diaudit pada G3.

## Dataset wajib

Logo flat 2–8 warna; logo dengan hole; ilustrasi detail; foto; sumber alpha nol; antialias alpha parsial; semua format input; EXIF rotation; non-square; 1×1; sumber di bawah ukuran preview; 20 MP; >30 MP; file animasi; file corrupt; ekstensi palsu; nama Unicode/spasi/panjang; dua sumber bernama sama dari folder berbeda. Tambahkan file gradient untuk mengamati kualitas tracing, bukan mengharapkan gradient vector output yang belum dijanjikan.

## Regresi kualitas logo — QV-001 (26 Oktober 2026)

Synthetic fixture dengan delapan bentuk gelap terpisah adalah pengganti aman untuk menguji kehilangan area foreground, bukan bukti bahwa seluruh glyph font dunia nyata aman. Tes regresi opsional `gallery_computer_wordmark_retains_all_seven_dark_letters` memeriksa tujuh huruf GALLERY dan delapan huruf COMPUTER; tes memerlukan PNG milik pengguna pada `VECTORFORGE_QUALITY_IMAGE` (1536×1024); fixture itu tidak dicommit. Jika tes gagal, bandingkan output segmentasi, fit, dan serialisasi sebelum mengubah engine. Jalankan juga matriks maxSide 1024 vs 2048 dan ekspor resolusi penuh; `path_count > 0` saja tidak cukup untuk mengklaim bentuk lengkap. Status QA Windows 29 Oktober 2026: pengguna melaporkan `cargo test` **81 PASS, 0 FAIL, 2 ignored**; `cargo clippy --all-targets -- -D warnings` **PASS** setelah perubahan `as_chunks`. `cargo fmt -- --check` melaporkan sembilan diff format pada tiga file Rust; patch berikutnya telah menyesuaikan seluruh diff dari output pengguna. Rerun `cargo fmt -- --check` setelah `git pull --ff-only` ke `0f4a710` selesai tanpa output menurut laporan pengguna (**PASS**). Gate Rust: 81 PASS / 0 FAIL / 2 ignored, Clippy PASS, rustfmt PASS. Tes frontend dan evaluasi visual belum dilaporkan. Regresi Gallery Computer yang biasanya ignored telah dijalankan eksplisit pada Windows 29 Oktober 2026: file PNG lokal tersedia (`Test-Path True`); **1 PASS, 0 FAIL, 82 filtered out**, 1,68 detik. Tes hanya memeriksa keberadaan command path berwarna tertentu dalam area setiap huruf; ini bukan pixel-level comparison atau konfirmasi visual bahwa seluruh glyph benar. Tes ignored lainnya tidak otomatis ikut dijalankan.

Inspeksi visual dilaporkan 29 Oktober 2026 berdasarkan screenshot dan file SVG yang diunggah: **24 path, viewBox 0 0 1536 1024, tanpa raster embedded, ikon dan kedua wordmark utuh**; hasil SSIM ~0,985 saat SVG dirender pada 1536×1024 dan dibandingkan dengan PNG. Ini adalah verifikasi untuk fixture Gallery Computer pada hasil yang diunggah, bukan konfirmasi semua skenario dan bukan pengganti perbandingan pada beberapa resolusi preview. Catatan produk: path background krem penuh masih ada; mode latar checkerboard pada kanvas tidak otomatis membuat ekspor transparan.

## QV-005 — AI Smooth Illustration (belum diuji di Windows)

| ID | Kasus uji | Kriteria |
|---|---|---|
| TEST-QV005-01 | Default, logo, foto, poster lama | `smoothness=0`, tracing legacy tidak berubah |
| TEST-QV005-02 | JSON preset versi 1 tanpa smoothness | Rust deserialize memberi 0, preset dapat dipakai |
| TEST-QV005-03 | Slider/input 0–3 dan nilai invalid | Batas valid, nonfinite ditolak oleh Rust dan TS |
| TEST-QV005-04 | Mapping pipeline VTracer | Spline 1.5 → `simplify=Some(1.5)`; Polygon → `None` |
| TEST-QV005-05 | Preset AI via frontend preview | IPC `generatePreview` mengandung `smoothness=1.5` |
| TEST-QV005-06 | Dua PNG AI detail tinggi berbeda | Compare 100%/200%, contour legibility, hilangnya elemen, path count, ukuran SVG, perubahan warna |
| TEST-QV005-07 | PNG alpha parsial + logo huruf kecil | Tidak ada hole hilang, alpha tetap benar, tidak menggabungkan huruf |
| TEST-QV005-08 | SVG/PDF/EPS dan batch | Punya output vektor valid, semantics EPS flatten tetap sama |

Metrik visual perlu hasil pengukuran nyata; jangan menandai smoothness meningkatkan fidelitas hanya dari PASS tes yang memverifikasi parameter mapping. Tidak ada jaringan atau AI SDK baru. Tahap berikutnya (preprocessing/gradient) belum diimplementasikan.


**QA Windows 29 Oktober 2026 (dilaporkan pengguna):** `cargo test`: **86 PASS / 0 FAIL / 2 ignored**, `cargo clippy --all-targets -- -D warnings`: **PASS**, `cargo fmt -- --check`: **FAIL** dengan empat perubahan format pada `preview.rs` dan `presets.rs`. Patch formatting diterapkan dalam commit berikutnya; rerun `cargo fmt -- --check` **belum dilaporkan**. Gate frontend `npm run lint`, `npm test`, `npm run build` dan evaluasi visual gambar AI **belum dilaporkan**.

## QV-006 — Professional Segmentation (QA belum dijalankan)

| ID | Fixture & gate | Acceptance |
|---|---|---|
| TEST-QV006-01 | DTO TS/Rust v1 tanpa field baru | `segmentationMode=color-cluster`, `paletteLimit=0`, semua valid |
| TEST-QV006-02 | Nilai invalid palet dan enum | Rust dan TypeScript tolak >64, fractional / enum invalid |
| TEST-QV006-03 | Config mapping ColorCluster vs Watershed | Default klasik, mode profesional, `max_colors=24`, cutout sesuai preset |
| TEST-QV006-04 | Preset profesional dan nama preset lama | Builtin ID stabil, preset lama tetap dapat dibaca |
| TEST-QV006-05 | Preview IPC dan perubahan parameter | Segmentation dan palette diteruskan, hasil lama diinvalidasi saat setting berubah |
| TEST-QV006-06 | Synthetic multi-object segmented input | Tidak ada objek gelap terpisah yang hilang akibat watershed/cutout/palette |
| TEST-QV006-07 | Logo asli, dua AI illustration, alpha parsial | Uji warna, outline, ruang negatif, tepi, dan bentuk kecil secara visual |
| TEST-QV006-08 | Ekspor full-res SVG/PDF/EPS dan batch | File benar-benar vektor, transparansi tidak berubah, ukuran memori/latensi dicatat |

Tes QV-006 belum diuji lokal karena toolchain Rust dan environment aplikasi Windows tidak tersedia dalam sesi assistant. Pengguna harus menjalankan gate Windows; PR tetap Draft hingga bukti hasil. Akurasi kontur profesional, straight-edge fitting, primitive fitting, dan quality scoring **belum diterapkan** oleh milestone ini.

## QV-007 — Line-aware Geometry (belum diuji Windows)

| ID | Tes | Acceptance |
|---|---|---|
| TEST-QV007-01 | FittedGeom cubic hampir lurus | Handle ditransformasikan jadi 1/3 dan 2/3 chord, endpoint tetap |
| TEST-QV007-02 | Kurva organik, tepi pendek, handle overshoot, NaN | Dilewati tanpa perubahan, tidak panic |
| TEST-QV007-03 | Closed ring dan sudut | Titik sudut dan seam tidak berpindah |
| TEST-QV007-04 | Polygon | Tidak menambahkan curve pass; geometri polyline tidak berubah |
| TEST-QV007-05 | Segmented rectangles cutout | Seluruh objek tetap ditemukan setelah mode line-aware |
| TEST-QV007-06 | JSON preset lama dan enum baru | `geometryMode=classic` default; `line-aware` serialized kebab-case |
| TEST-QV007-07 | Frontend preset dan preview request | Logo Presisi mengirim `geometryMode=line-aware`; preset lama classic |
| TEST-QV007-08 | File asli + gambar organik dan alpha | Bandingkan dengan mode classic, straight-edge quality dan topology di preview 1024/2048 serta SVG full-res |

Belum ada hasil QA Windows untuk QV-007; tes ini sudah ditulis tetapi belum dijalankan. Perbandingan kualitas tidak boleh diganti dengan angka path-count saja. PR tetap Draft; QV-008 quality guard serta full manual-grade vector reconstruction masih deferred.

## QV-008 — Contour Quality Guard (Windows QA belum dilakukan)

| ID | Tes | Kriteria |
|---|---|---|
| TEST-QV008-01 | Identical vs shifted Bézier geometry | Kontur identik diterima, bergeser >0,6 px ditolak |
| TEST-QV008-02 | Nonfinite geometry dan area ring berbeda | Ditolak; arah/orientasi tidak dibalik |
| TEST-QV008-03 | Strict guard pada kurva hampir lurus | Straightening yang melanggar batas dikembalikan ke geometri awal |
| TEST-QV008-04 | Polygon dan kontur >96 kubik | Tidak dimodifikasi, waktu komputasi dibatasi |
| TEST-QV008-05 | Preset/serde compatibility | Preset v1 default false; preset professional true |
| TEST-QV008-06 | Preview parameter forwarding | `qualityGuard=true` mencapai IPC untuk preset professional |
| TEST-QV008-07 | Gallery Computer PNG + beberapa gambar AI | Sampel 0/0,5/1/2,5 dan baseline non-guard: keutuhan glyph, ruang negatif, tepian dan SVG rendering |
| TEST-QV008-08 | Source alpha parsial, ekspor SVG/PDF/EPS, benchmark | Tidak regress alpha, tidak membuat output raster palsu, waktu/memori dilaporkan |

**Status:** tes tertulis, belum ada hasil `cargo test`, lint, rustfmt, ataupun pengujian visual di Windows untuk branch QV-008. Ini guard pada kontur hasil tracing, bukan pengganti raster feedback atau metrik SSIM.

## QV-009 — Raster Feedback (Windows QA belum dilakukan)

| ID | Tes | Kriteria |
|---|---|---|
| TEST-QV009-01 | JSON lama tanpa rasterFeedback | False secara default; preset dan ekspor lama tetap kompatibel |
| TEST-QV009-02 | TS parameter dan native preview IPC | Opsi boolean valid, diteruskan dari panel ke request |
| TEST-QV009-03 | SVG hitam vs putih terhadap raster hitam | Candidate yang cocok dipilih, mismatch ditolak |
| TEST-QV009-04 | Opaque dan alpha=0 | Hidden RGB transparan tidak dihitung; alpha yang salah mendapat penalti |
| TEST-QV009-05 | Segmentasi sama (color/palette/alpha), dua finish | Baseline vs candidate berbagi segmentasi tanpa pemrosesan ulang front-end |
| TEST-QV009-06 | Input >8MP dan SVG evaluasi >8MiB | Error eksplisit saat opt-in, output tidak di-downscale diam-diam |
| TEST-QV009-07 | Parameter polygon atau perubahan nol | Guard tidak mengubah output karena tidak ada perubahan geometri yang perlu dibandingkan |
| TEST-QV009-08 | Gallery Computer + dua ilustrasi AI | Compare guard ON/OFF dengan smoothness 0,5/1/2,5; kontur, area huruf, alpha, path count, warna, latency dan peak memory |
| TEST-QV009-09 | Ekspor SVG/PDF/EPS dan batch | Seluruh format memakai keputusan yang sama pada file full-resolution, EPS tetap flatten putih |

QA Windows belum dijalankan. Fitur ini merupakan reference-based comparison terhadap dua kandidat VectorDoc *dengan segmentasi identik*, bukan classifier kualitas desain yang komprehensif. Nilai error dan ambang masih membutuhkan verifikasi empiris pada dataset representatif, bukan diklaim sebagai ukuran universal.

## QV-010 — Raster Quality Diagnostics (Windows QA belum dilakukan)

| ID | Pemeriksaan | Bukti |
|---|---|---|
| TEST-QV010-01 | Buffer raster panjang berbeda, width=0 | Error eksplisit, tidak ada zip partial scoring |
| TEST-QV010-02 | Ukuran buffer RGBA/dimensi sumber salah | Diagnostic score menolak input invalid |
| TEST-QV010-03 | Fixture real PNG lokal | `scripts/qv010-quality-qa.ps1 -ImagePath <path>`: keluaran `QV010 label=...` |
| TEST-QV010-04 | Smoothing 0 vs 1 vs 2,5 dan feedback OFF/ON | Laporan skor error (lebih kecil lebih baik), count path/kubik, SVG bytes, elapsedMs |
| TEST-QV010-05 | Vector authenticity | SVG hasil tracing tidak mengandung `<image>` |
| TEST-QV010-06 | Gallery Computer dan dua AI images | Kontur/huruf/hole utuh dan penilaian manual 200–400% |
| TEST-QV010-07 | Windows frontend/native gates | npm lint/test/build, cargo check/test/clippy/rustfmt --locked sesuai instruksi |
| TEST-QV010-08 | Benchmark waktu/memori | Bandingkan 1/4/8 MP, single vs batch; laporkan peak proses dan overhead ON/OFF |

Status: **kode tes ditulis, tidak ada bukti Windows run** untuk PR #7 maupun PR sebelumnya. Benchmark & skor objektif adalah alat diagnosis, bukan bukti mutu profesional secara otomatis. Laporan score menggunakan rumus QV-009 (RGB premultiplied + edge-weighted), bukan SSIM.

## QV-011 — Acceptance Matrix (Windows execution pending)

| Test | Kriteria | Bukti |
|---|---|---|
| TEST-QV011-01 | Synthetic logo negative-space ring | Render 160×160: ring gelap, lubang terang, background terang; `<path>` nyata tanpa `<image>` |
| TEST-QV011-02 | Raster Feedback ON, smooth 2.5 + line-aware | Mode aktif harus dapat dirender; tidak boleh menghapus ruang negatif |
| TEST-QV011-03 | 8 automated gates | `npm ci`, lint, test, build, `cargo check/test/clippy/fmt` merekam PASS/FAIL |
| TEST-QV011-04 | QA images lokal | Setiap fixture pilihan pengguna menjalankan QV-010 dan menyimpan log lokal |
| TEST-QV011-05 | Summary integrity | Satu `qa-summary.csv`, satu file log per gate, exit code nonzero jika gagal |
| TEST-QV011-06 | Gallery Computer + minimal 2 AI source | Audit visual 200–400%, negative space, kontur, latensi dan memori; belum ada hasil Windows |
| TEST-QV011-07 | Thin-stroke disappearance | Native-resolution SVG candidate with 3 px stroke must score better than white-only vector; missing-stroke candidate rejected |
| TEST-QV011-08 | Alpha disagreement | Identical RGB with different alpha must produce strictly higher RGBA error |
| TEST-QV011-09 | Real-image guard/feedback isolation | QV010 compares baseline_0, smooth_05, smooth_1, smooth_1_guard, smooth_1_feedback, smooth_25, smooth_25_feedback, line_aware_feedback |
| TEST-QV011-10 | Raster Feedback decision invariant | For feedback=true, measured error must not exceed baseline + 0.35 + 1% baseline (floating tolerance 1e-6) |

Contoh penggunaan dari root repo:

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\qv011-acceptance.ps1 -ImagePaths @("C:\\Images\\gallery.png", "C:\\Images\\ai-illustration-1.png")
```

Log disimpan di `%TEMP%\VectorForge-QA\QV011-<timestamp>\` bukan di repo, sehingga tidak mengotori working tree. Tidak ada data gambar disalin ke log; yang dicatat adalah metrik dan gate. Skrip tidak menggantikan visual QA. **STATUS: NOT RUN**.

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
