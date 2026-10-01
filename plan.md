# Plan — Urutan Kerja dan Kendali Scope

Status 1 Oktober 2026: P0, frontend P1/G1, backend B01–B06, dan boundary hardening pra-G2 selesai. Backend final lulus `cargo check`, 74/74 Rust tests, Clippy `-D warnings`, rustfmt, serta smoke test Windows untuk preview → release → re-import file yang sama. Preview source kini memakai custom local protocol `vfsource` berbasis opaque `fileId`, sehingga URL lama mati setelah release tanpa mematahkan re-import. Backend siap checkpoint G2; integrasi frontend produksi tetap belum diizinkan.

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
| 2026-10-01 | B01 | DONE | Engine/export/alpha/filesystem Windows PASS; Cargo.lock committed; Cargo metadata menunjukkan seluruh dependency eksternal/transitif memiliki deklarasi license | User kemudian memerintahkan lanjut |
| 2026-10-01 | B02 | DONE | Tauri shell/native boundary compile bersih di Windows; 11/11 Rust tests PASS; Clippy -D warnings PASS; rustfmt PASS; smoke-run PASS; Cargo.lock committed; audit lisensi eksternal/transitif PASS; working tree clean | User kemudian mengizinkan B03 |
| 2026-10-01 | B03 | DONE | Core + fixture preview nyata lulus; 28/28 Rust tests PASS; Clippy -D warnings PASS; rustfmt PASS; latest-wins/cancel, alpha preservation, SOURCE_CHANGED, 30 MP dan 50 MiB guards terbukti; Cargo.lock committed | User kemudian mengizinkan B04 |
| 2026-10-01 | B04 | DONE | Export SVG/PDF/EPS produksi + atomic Windows commit lulus; 44/44 Rust tests PASS; Clippy -D warnings PASS; rustfmt PASS; Cargo.lock committed; audit lisensi eksternal/transitif PASS | User kemudian mengizinkan B05; sebelum G2 verifikasi/fix asset-protocol scope revocation pada release_files |
| 2026-10-01 | B05 | DONE | Settings/preset storage produksi lulus; 58/58 Rust tests PASS; Clippy -D warnings PASS; rustfmt PASS; Cargo.lock committed; tidak ada package/version baru di luar graph dependency yang sudah diaudit | User kemudian mengizinkan B06; sebelum G2 verifikasi/fix asset-protocol scope revocation pada release_files |
| 2026-10-01 | B06 | DONE | Batch scheduler produksi lulus; 70/70 Rust tests PASS; Clippy -D warnings PASS; rustfmt PASS; partial/cancel/retry/collision fixtures lulus; working tree clean | User memerintahkan bereskan boundary asset-scope sebelum G2 |
| 2026-10-01 | B02 boundary hardening | DONE | Custom local protocol `vfsource` berbasis opaque `fileId` menggantikan revocation asset-scope one-way; handler registry-gated memverifikasi fingerprint sebelum/sesudah read; `cargo check` PASS, 74/74 tests PASS, Clippy PASS, rustfmt PASS, smoke preview → release → re-import PASS | Backend siap checkpoint G2; I01 tetap menunggu persetujuan G2 |

## Catatan izin fase

- 1 Oktober 2026: user memerintahkan eksekusi ke repository ferdilpu-sudo/VectorForge; izin meliputi memasukkan pedoman dan memulai frontend P1.
- 1 Oktober 2026: setelah review screenshot Windows, polish, dan verifikasi lokal sukses, user memerintahkan **lanjut**. Ini dicatat sebagai persetujuan melewati G1 dan izin mengerjakan **B01 saja**. Tidak ada izin melewati G2 atau menganggap B01 otomatis mengizinkan B02.
- 1 Oktober 2026: setelah B01 resmi DONE, user memerintahkan **lanjut**. Ini menjadi izin mengerjakan **B02 saja**. B03/B04/B05/B06 dan integrasi frontend produksi belum otomatis diizinkan.
- 1 Oktober 2026: setelah B02 resmi DONE, user memerintahkan **lanjut b03**. Ini menjadi izin mengerjakan **B03 saja**. B04/B05/B06 dan integrasi frontend produksi belum otomatis diizinkan.
- 1 Oktober 2026: setelah B03 resmi DONE, user memerintahkan **lanjut**. Ini menjadi izin mengerjakan **B04 saja**. B05/B06 dan integrasi frontend produksi belum otomatis diizinkan.
- 1 Oktober 2026: setelah B04 resmi DONE, user memerintahkan **lanjut**. Ini menjadi izin mengerjakan **B05 saja**. B06 dan integrasi frontend produksi belum otomatis diizinkan.
- 1 Oktober 2026: setelah B05 resmi DONE, user memerintahkan **lanjut**. Ini menjadi izin mengerjakan **B06 saja**. Integrasi frontend produksi dan G2 belum otomatis diizinkan.
- 1 Oktober 2026: user mengonfirmasi smoke test preview → release → re-import file yang sama **normal** setelah custom protocol `vfsource` diterapkan. Ini menutup blocker boundary pra-G2, tetapi bukan persetujuan G2 atau izin I01.
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


### B01 ditutup — 1 Oktober 2026

Audit `cargo metadata --locked` menunjukkan satu-satunya package tanpa field `license` adalah package lokal `vectorforge-b01-engine-spike 0.0.0`. Semua dependency eksternal/transitif memiliki deklarasi license. Spike lokal `publish = false`; lisensi aplikasi tetap keputusan pemilik dan tidak diisi secara asumtif.

Dengan build/run release Windows, alpha split-mask, SVG/PDF/EPS, cancellation, resource characterization, filesystem safety, Cargo.lock, dependency tree dan audit lisensi selesai, **B01 = DONE**.

B02 belum dimulai. Catatan izin fase tetap berlaku: persetujuan sebelumnya hanya untuk B01 dan tidak otomatis memberi izin membuat backend produksi.


### B02 bootstrap icon

Build Windows pertama berhenti di `tauri-build` sebelum compile Rust karena `src-tauri/icons/icon.ico` belum ada. B02 menambahkan bootstrap icon teknis minimal hanya agar Windows resource generation dapat berjalan. Ini **bukan** keputusan branding final; icon aplikasi/installer final tetap pekerjaan H02.


### B02 verifikasi Rust awal

Verifikasi lokal Windows setelah cleanup scope B02:
- `cargo check`: PASS;
- `cargo test`: PASS, 11 test / 0 gagal;
- `cargo clippy --all-targets -- -D warnings`: PASS.

Cleanup sebelum hasil ini menghapus model tracing B03 yang terlalu dini dan resolver registry yang belum dipakai, lalu menggantinya dengan invariant validation pada registrasi source/destination. Tidak ada `allow(dead_code)` atau pelonggaran lint. B02 masih DOING sampai rustfmt, smoke-run Tauri Windows, dan lockfile/dependency audit selesai.


### B02 smoke-run Windows

Pengguna menjalankan shell Tauri di Windows dan mengonfirmasi window VectorForge muncul serta UI frontend tampil setelah `npm run dev` dijalankan. Ini sesuai konfigurasi development karena `tauri.conf.json` memakai `devUrl = http://127.0.0.1:5173`. Startup shell tidak crash pada smoke-run ini.

Smoke-run dev **PASS**. Ini bukan klaim installer production atau frontend production adapter sudah terintegrasi.


### B02 lockfile dan lisensi

`src-tauri/Cargo.lock` dikomit pengguna pada commit `1034cb9`. Audit `cargo metadata --locked` menunjukkan satu-satunya package tanpa field `license` adalah package lokal `vectorforge 0.1.0`; seluruh dependency eksternal/transitif memiliki deklarasi license. Lisensi aplikasi VectorForge belum diputuskan dan tidak diisi secara asumtif.

Folder `src-tauri/gen/` adalah output generated Tauri dan di-ignore dari repository; source capability tetap berada di `src-tauri/capabilities/`.


### B02 ditutup — 1 Oktober 2026

Verifikasi final pengguna:
- `cargo check`: PASS;
- `cargo test`: PASS, 11/11;
- `cargo clippy --all-targets -- -D warnings`: PASS;
- `cargo fmt -- --check`: PASS tanpa output;
- smoke-run Windows: window native VectorForge muncul, frontend tampil dengan Vite dev server aktif, tidak crash saat startup;
- `src-tauri/Cargo.lock` committed pada `1034cb9`;
- audit `cargo metadata --locked`: seluruh dependency eksternal/transitif memiliki deklarasi license; hanya package lokal `vectorforge 0.1.0` tanpa field license;
- working tree bersih dan sinkron dengan `origin/main`.

Dengan bukti ini, **B02 = DONE**.

B03 belum dimulai. Scope berikutnya adalah decode + preview scheduler + adapter vtracer, tetapi tetap menunggu instruksi user untuk lanjut fase berikutnya.


### B03 verifikasi core preview

Verifikasi lokal Windows setelah implementasi awal B03:
- `cargo check`: PASS;
- `cargo test`: PASS, 24/24;
- `cargo clippy --all-targets -- -D warnings`: PASS;
- `cargo fmt -- --check`: PASS tanpa output.

Bukti unit yang sudah lulus meliputi decode tanpa upscale, resize aspect-ratio, orientation-before-resize, alpha split 128/255, alpha mask budget, fully-transparent bypass, schema-to-VTracer mapping, production adapter partial alpha, PreviewRequest validation, dan scheduler latest-wins. B03 tetap DOING sampai fixture preview nyata end-to-end dan resource-limit verification selesai.


### B03 ditutup — 1 Oktober 2026

Verifikasi final pengguna:
- `cargo check`: PASS;
- `cargo test`: PASS, 28/28;
- `cargo clippy --all-targets -- -D warnings`: PASS;
- `cargo fmt -- --check`: PASS tanpa output;
- fixture PNG nyata melewati jalur production `probe/fingerprint → decode/orientation → resize → VTracer → SVG`;
- hasil preview terbukti menghasilkan path vektor nyata dan tidak menyisipkan raster `<image>`;
- alpha 128/255 tetap dipertahankan melalui adapter split-mask produksi;
- fully-transparent source melewati tracer dengan hasil SVG kosong yang valid;
- scheduler terbukti latest-wins: active lama dibatalkan/stale, pending lama diganti, request terbaru diproses;
- source yang berubah sejak import menghasilkan `SOURCE_CHANGED`;
- batas 30 MP diuji tepat di boundary dan di atas boundary;
- batas SVG preview 50 MiB diuji tepat di boundary dan +1 byte;
- `src-tauri/Cargo.lock` produksi dikomit pada `3e8ebcd`;
- seluruh 16 dependency baru B03 pada lockfile produksi identik dengan package/version yang sudah ada pada lockfile spike B01 yang sebelumnya lolos audit lisensi transitif. Tidak ada dependency delta B03 di luar graph ter-audit tersebut;
- working tree pengguna dilaporkan clean dan sinkron dengan `origin/main`.

Dengan bukti ini, **B03 = DONE**.

B04 belum dimulai. Scope berikutnya adalah export SVG/PDF/EPS produksi, tetapi tetap menunggu instruksi user untuk lanjut fase berikutnya.


### B04 ditutup — 1 Oktober 2026

Verifikasi final pengguna:
- `cargo check`: PASS;
- `cargo test`: PASS, 44/44;
- `cargo clippy --all-targets -- -D warnings`: PASS;
- `cargo fmt -- --check`: PASS tanpa output;
- SVG export memakai trace resolusi penuh dan mempertahankan alpha parsial;
- PDF dihasilkan dari SVG vektor melalui `svg2pdf 0.13.0`, bukan raster fallback;
- EPS tetap vektor dan transparansi dikomposit ke putih sebelum tracing sesuai D11;
- output bounds/statistik mengikuti sumber penuh;
- source fingerprint diverifikasi sebelum dan setelah decode;
- source file tidak dapat dipilih sebagai target overwrite;
- output baru memakai temporary file same-directory lalu commit aman;
- overwrite terkonfirmasi memakai `ReplaceFileW` di Windows;
- collision tanpa izin dan locked target gagal aman tanpa merusak output lama;
- SVG >50 MiB membutuhkan `allowLargeOutput=true`;
- `outputId` diregistrasikan secara opaque di Rust;
- `src-tauri/Cargo.lock` dikomit pada `d2df21f`;
- audit `cargo metadata --locked` menunjukkan satu-satunya package tanpa field license adalah package lokal `vectorforge 0.1.0`;
- dependency export utama terverifikasi: `svg2pdf 0.13.0`, `usvg 0.45.1`, `pdf-writer 0.12.1`, `image 0.25.10`, `visioncortex 0.9.3`, `vtracer 1.0.0-alpha.4`, semuanya memiliki deklarasi license dan seluruh versi tersebut ada pada graph spike B01 yang telah diaudit;
- working tree pengguna clean dan sinkron dengan `origin/main`.

Dengan bukti ini, **B04 = DONE**.

Catatan lintas fase sebelum G2: `release_files` saat ini menghapus source dari registry, tetapi pencabutan grant pada Tauri asset-protocol scope perlu diverifikasi terhadap API Tauri 2.12 dan diperbaiki bila belum dicabut. Ini technical debt boundary B02, bukan blocker acceptance B04, namun wajib diselesaikan sebelum backend review G2.


### B05 ditutup — 1 Oktober 2026

Verifikasi final pengguna:
- `cargo check`: PASS pada rangkaian verifikasi B05;
- `cargo test`: PASS, 58/58;
- `cargo clippy --all-targets -- -D warnings`: PASS;
- `cargo fmt -- --check`: PASS tanpa output;
- settings memakai wrapper versioned `SettingsFile { version: 1, settings }`;
- preset user memakai wrapper versioned `PresetsFile { version: 1, presets }`;
- file tidak ada menghasilkan default/daftar built-in tanpa menulis file baru secara diam-diam;
- empat preset built-in memakai ID stabil dan tidak pernah ditulis sebagai user preset;
- nama preset user divalidasi 1–40 Unicode code points dan unik case-insensitive terhadap built-in maupun user preset;
- built-in preset read-only; delete user preset memakai ID UUID opaque;
- `createdAt` user preset dibuat sebagai RFC3339 UTC;
- settings dan preset memakai app data dir dari resolver Tauri, bukan hardcode path OS;
- seluruh I/O storage command dijalankan melalui `spawn_blocking` dan satu writer mutex;
- write memakai temporary file same-directory lalu atomic commit/replace;
- JSON corrupt menghasilkan `DATA_CORRUPT`, byte file asli dipertahankan dan mutation diblokir;
- versi data yang tidak didukung menghasilkan `DATA_VERSION_UNSUPPORTED`, file asli dipertahankan dan mutation diblokir;
- tidak ada auto-reset destructive tanpa persetujuan eksplisit;
- command native `list_presets`, `save_preset`, `delete_preset`, `get_settings`, dan `save_settings` terdaftar;
- `src-tauri/Cargo.lock` dikomit pada `81c31ef`;
- `serde_json 1.0.149` dan `time 0.3.55` sudah ada pada graph dependency sebelum B05 dan B05 tidak menambah package/version baru di luar graph yang telah diaudit;
- working tree pengguna clean dan sinkron dengan `origin/main`.

Dengan bukti ini, **B05 = DONE**.

B06 belum dimulai. Catatan lintas fase tetap berlaku: sebelum G2, `release_files` wajib diverifikasi terhadap Tauri 2.12 agar grant asset-protocol scope benar-benar dicabut ketika referensi sumber terakhir dilepas.


### Boundary pra-G2 ditutup — 1 Oktober 2026

Masalah awal: `release_files` hanya menghapus source dari registry, sementara preview masih memakai Tauri asset protocol. Inspeksi source Tauri 2.12 menunjukkan `forbid_file()` bersifat one-way selama sesi karena forbidden path selalu mengalahkan allowed path; pendekatan itu akan mematahkan re-import file yang sama.

Solusi final:
- preview source dipindahkan ke custom local protocol `vfsource`;
- URL preview hanya membawa opaque `fileId`, bukan path filesystem;
- handler Rust resolve `fileId` melalui registry;
- fingerprint diverifikasi sebelum dan sesudah membaca bytes;
- MIME berasal dari format hasil probe, bukan ekstensi;
- response memakai `Cache-Control: no-store`;
- setelah `release_files`, ID lama hilang dari registry sehingga URL lama tidak lagi dapat melayani file;
- file yang sama dapat di-import ulang dan mendapat ID/URL baru;
- CSP production dan development hanya mengizinkan origin lokal `http://vfsource.localhost` untuk preview source.

Bukti Windows final:
- `cargo check`: PASS;
- `cargo test`: PASS, 74/74;
- `cargo clippy --all-targets -- -D warnings`: PASS;
- `cargo fmt -- --check`: PASS;
- smoke test native WebView2: import gambar → preview tampil → release → import file yang sama → preview tampil normal.

Dengan bukti ini, technical debt boundary B02 pra-G2 **DONE**. Backend P2 siap checkpoint G2. I01/P3 belum diizinkan.
