# Plan — Urutan Kerja dan Kendali Scope

Status 1 Oktober 2026: P0 dan frontend P1 selesai untuk gate G1. UI terbaru telah ditinjau pengguna di Windows; lint, 10 tes dan build frontend lulus lokal. Pengguna kemudian memerintahkan lanjut, sehingga G1 dicatat DONE dan B01 mulai sebagai spike terisolasi. B02 dan backend produksi belum diizinkan sampai B01 selesai.

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
| 2026-10-01 | G1 | DONE | Screenshot UI Windows ditinjau pengguna; polish panel/canvas/batch diterapkan; lint, 10 tes dan build lulus lokal | Pengguna menyatakan sukses lalu memerintahkan lanjut |
| 2026-10-01 | B01 | DOING | Engine/export/alpha/filesystem Windows PASS; dependency tree terkunci dan Cargo.lock dikomit pada 2bf29cf | Tinggal audit lisensi transitif sebelum B01 DONE |

## Catatan izin fase

- 1 Oktober 2026: user memerintahkan eksekusi ke repository ferdilpu-sudo/VectorForge; izin meliputi memasukkan pedoman dan memulai frontend P1.
- 1 Oktober 2026: setelah review screenshot Windows, polish, dan verifikasi lokal sukses, user memerintahkan **lanjut**. Ini dicatat sebagai persetujuan melewati G1 dan izin mengerjakan **B01 saja**. Tidak ada izin melewati G2 atau menganggap B01 otomatis mengizinkan B02.
- G2 belum disetujui.
- Isi tanggal, pesan persetujuan dan cakupan nyata ketika izin diterima. Jangan mengisi asumsi sebagai persetujuan.

## Target performa

Target PRD adalah sasaran, bukan fakta: startup <2 s, preview tipikal 1024 px <1,5 s, installer <25 MB tanpa WebView2 runtime, memori <500 MB untuk skenario satu sumber 20 MP. Ukur dengan hardware, dataset, build release dan metode di testing. Batch paralel tidak dianggap otomatis memenuhi target memori single. Kebenaran output dan stabilitas lebih tinggi prioritas daripada angka target; kegagalan target harus dilaporkan, bukan disembunyikan dengan menurunkan kualitas.

## Hasil eksekusi P1 — 1 Oktober 2026

- F01–F05 selesai untuk scope demo G1: React 18/TypeScript/Tailwind/Zustand, import browser, parameter/preset/settings, compare/zoom/pan, export demo dan batch demo.
- Verifikasi lokal pengguna: `npm run lint` lulus setelah perbaikan binding catch, 3 test files / 10 tests lulus, dan `npm run build` lulus.
- Teruji otomatis: boundary parameter, nama preset, cancellation mock, stale preview cleanup, manual preview, partial-output retry/cancel batch, recovery preference corrupt dan locale error import.
- Screenshot terbaru ditinjau pengguna pada Windows dan dipoles lagi pada panel batch. Ini memenuhi bukti screenshot + persetujuan untuk G1.
- Tidak mengklaim matriks DPI/keyboard lengkap atau native Windows shell sudah diuji; keduanya tetap item verifikasi berikutnya, bukan alasan menyamarkan demo sebagai aplikasi native.
- Native pre-decode validation, filesystem grant dan tracing nyata tetap fase B02/B03.

## Polish F02 — 1 Oktober 2026

User meminta polish setelah meninjau screenshot dan memastikan tema/latar bisa diganti. Perubahan: panel lebih padat, header lebih singkat, empty canvas solid adaptif, banner demo ringkas, scrollbar bertema. Pilihan dark/light/system dan latar putih/checker/gelap dipertahankan. Tidak mengubah engine atau kontrak IPC. Setelah polish lanjutan batch, pengguna meninjau hasil terbaru di Windows dan verifikasi lokal berakhir dengan lint, 10 tes dan build lulus. G1 kemudian disetujui dan ditutup.


## B01 spike — 1 Oktober 2026

- Kandidat tracing: `vtracer = 1.0.0-alpha.4` untuk spike, karena menyediakan `CancelToken`, progress fase, `VectorDoc` dan pipeline segment/finish. Status pre-release berarti belum menjadi lock produksi.
- Kandidat decode: `image = 0.25.10` dengan PNG/JPEG/WebP/BMP.
- Kandidat PDF: `svg2pdf = 0.13.0`; gunakan `svg2pdf::usvg` agar tidak membuat version skew dengan usvg standalone.
- Harness terisolasi berada di `spikes/b01-engine`; tidak boleh diimpor frontend atau dianggap backend produksi.
- Run Windows membuktikan opaque SVG, PDF, EPS dan cooperative cancellation bekerja. Fixture alpha nol memicu panic `attempt to divide by zero` di `visioncortex 0.9.3`; harness menangkap panic fixture agar pengujian alpha parsial dapat lanjut. Source inspection juga menunjukkan stock writer memakai paint/fill RGB solid dan tidak menyerialisasi alpha parsial.
- Detail versi, lisensi awal, langkah run, dan kriteria keluar ada di `b01-engine-spike.md`.


### B01 karakterisasi alpha lanjutan

Setelah run Windows membuktikan panic pada gambar alpha nol dan stock writer menghilangkan opacity, inspeksi upstream menunjukkan `visioncortex::Color` tetap membawa channel alpha sampai tipe paint. Spike diperluas untuk memeriksa alpha pada `VectorDoc`, menulis `fill-opacity` sendiri, menguji PDF dari SVG tersebut, serta menguji boundary RGB-sama/alpha-berbeda. Pre-check gambar seluruhnya transparan diperlakukan sebagai solusi semantik yang valid: hasilnya dokumen kosong dan tracer tidak dipanggil.


### B01 split-mask alpha spike

Stock segmentation meratakan fixture RGB-sama dengan alpha 128/255 menjadi alpha 191. Karena IR segmentation VTracer public, B01 menguji split-mask per alpha setelah clustering namun sebelum curve fitting. Tujuannya mempertahankan engine VTracer tanpa fork dan tanpa multi-pass tracing. Dependency `visioncortex 0.9.3` ditambahkan langsung pada spike hanya untuk konstruksi `BinaryImage`. Correctness diuji dengan alpha exact; biaya layer/memory pada gambar nyata belum dianggap lulus.


### B01 alpha resource characterization

Correctness split-mask lulus pada Windows untuk alpha parsial dan boundary RGB-sama alpha 128/255. Spike berikutnya meng-crop sub-mask per alpha dan mengukur gradient coherent vs alpha terfragmentasi. Lower-bound mask dihitung dari 1 bit/pixel; benchmark keputusan harus memakai release build. Jika input alpha patologis menyebabkan ledakan layer/mask, kebijakan produksi adalah complexity guard + error eksplisit, bukan silent flatten.


### B01 hasil alpha release dan filesystem

Release benchmark Windows: smooth 512×256 dengan 255 level menghasilkan 16 KiB lower-bound mask (8 ms segment, 6 ms split); fragmented 256×256 dengan 255 level menghasilkan 1913 KiB (2 ms segment, 4 ms split). Guard tidak boleh berbasis jumlah alpha level. Candidate split-mask budget ditetapkan 128 MiB berdasarkan bounding-box mask sebelum alokasi, dengan error eksplisit jika terlampaui.

Spike berikutnya menguji filesystem Windows nyata: create_new collision, same-directory commit target baru, ReplaceFileW overwrite, dan kegagalan aman saat target dikunci deny-share.


### B01 Windows release gate — engine/filesystem PASS

Run pengguna pada commit `2ce516d` lulus seluruh karakterisasi engine, alpha dan filesystem. Stock VTracer tetap tercatat merata-ratakan fixture RGB-sama menjadi alpha 191, tetapi adapter split-mask VectorForge mempertahankan 128/255. Filesystem Windows lulus reservation collision, same-directory new-file commit, `ReplaceFileW` overwrite, dan locked-target safe failure.

Status B01 tetap DOING hanya untuk penutupan dependency graph/lisensi dan lockfile spike. Tidak ada blocker engine/export/filesystem tersisa dari hasil run ini.


### B01 lockfile

`spikes/b01-engine/Cargo.lock` telah dikomit pada `2bf29cf`; working tree pengguna clean dan branch lokal sinkron dengan origin. Exact dependency graph kini terkunci. B01 tinggal menutup audit lisensi transitif.
