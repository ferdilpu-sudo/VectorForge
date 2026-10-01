# B01 — Engine / Export Spike

Status: **DOING** • 1 Oktober 2026.

Dokumen ini mencatat pembuktian dependency sebelum VectorForge membuat shell Tauri atau adapter produksi. Isi folder `spikes/b01-engine` adalah harness karakterisasi, bukan backend aplikasi dan bukan izin menghubungkannya ke React.

## Kandidat versi

| Dependency | Kandidat B01 | Lisensi | Catatan |
|---|---:|---|---|
| Tauri | 2.12.0 | MIT / Apache-2.0 | Kandidat B02; tidak dikompilasi oleh harness B01. Release 2.12 menaikkan MSRV Tauri ke Rust 1.90 dan menghentikan dukungan resmi Windows 7. |
| vtracer | 1.0.0-alpha.4 | MIT / Apache-2.0 | **Pre-release**, dipilih hanya untuk spike karena API 1.0 menyediakan `CancelToken`, progress per fase dan `VectorDoc` publik. Belum menjadi dependency produksi. |
| visioncortex | 0.9.3 transitif | lihat upstream | Dibawa oleh vtracer alpha.4. Jangan ditambah sebagai dependency langsung sebelum ada kebutuhan API yang tidak tersedia lewat vtracer. |
| image | 0.25.10 | MIT / Apache-2.0 | Fitur yang dibutuhkan: PNG, JPEG, WebP, BMP. Decode hardening tetap tugas B02/B03. |
| svg2pdf | 0.13.0 | MIT / Apache-2.0 | Gunakan `svg2pdf::usvg` re-export agar versi parser cocok. Jangan pin `usvg` langsung pada B01. |
| usvg | 0.45.x transitif svg2pdf | MIT / Apache-2.0 | Versi kompatibel ditentukan oleh svg2pdf 0.13.0, bukan versi standalone terbaru. |

Sumber verifikasi:
- VTracer tag/API: https://github.com/visioncortex/vtracer/tree/1.0.0-alpha.4
- Tauri release: https://github.com/tauri-apps/tauri/releases/tag/tauri-v2.12.0
- image 0.25.10: https://docs.rs/image/0.25.10/image/
- svg2pdf 0.13.0: https://docs.rs/svg2pdf/0.13.0/svg2pdf/

Exact dependency graph dan lisensi transitif tetap harus dicatat dari hasil Cargo lokal sebelum B01 dinyatakan DONE.

## Mengapa bukan vtracer 0.6.5

Versi stabil 0.6.5 memiliki API tracing yang lebih lama dan tidak menyediakan kontrak cancellation/progress yang dibutuhkan architecture. Alpha 1.0.0-alpha.4 memiliki:
- `Pipeline::run_with_progress`;
- `CancelToken` yang dapat dibagikan lintas thread;
- progress bertahap `Segment`, `Compose`, `Optimize`;
- `VectorDoc` dan `PathCmd` publik sehingga writer EPS dapat dibuktikan tanpa mengurai SVG hasil sendiri;
- pipeline `segment` / `finish` yang menarik untuk preview interaktif.

Karena ini pre-release, keputusan ini masih **candidate**, bukan lock produksi. B01 wajib membuktikan build dan perilakunya di Windows pengguna.

## Harness

Dari root repository:

```powershell
cd spikes/b01-engine
cargo --version
rustc --version
cargo check
cargo run
cargo tree
```

Rust **1.90+** menjadi baseline spike agar cocok dengan kandidat Tauri 2.12.

Harness menghasilkan status tekstual untuk:
1. raster opaque → `VectorDoc` → SVG;
2. SVG → PDF melalui svg2pdf dengan `PageOptions { dpi: 96.0 }`, sesuai kebijakan 96 px = 72 pt;
3. `VectorDoc` → EPS solid path, nonzero fill rule, bounding box dan pembalikan sumbu Y;
4. progress fase + cooperative cancellation;
5. sumber alpha nol;
6. sumber alpha parsial;
7. boundary dengan RGB identik tetapi alpha berbeda, untuk membuktikan apakah segmentasi mempertahankan batas transparansi.

Harness tidak menulis output ke disk dan tidak menguji atomic replace Windows. Uji filesystem nyata tetap item B01 terpisah.

## Hasil run Windows — 1 Oktober 2026

Environment pengguna:
- `rustc 1.98.1 (48a229cea 2026-09-01)`;
- `cargo 1.98.1 (797e8a9bc 2026-08-05)`;
- host/target `x86_64-pc-windows-msvc`;
- MSVC Build Tools 2022 tersedia dan linker native berhasil dipakai.

Hasil pertama `cargo run` setelah harness dikompilasi:
- PASS raster opaque → `VectorDoc` → SVG;
- PASS SVG → PDF pada mapping 96 DPI;
- PASS `VectorDoc` → EPS solid path;
- PASS progress fase + cooperative cancellation;
- **BLOCKER** saat fixture alpha nol: proses panic di dependency `visioncortex 0.9.3`, `src/color_clusters/builder.rs:323:22`, dengan pesan `attempt to divide by zero`.

Panic dependency ini bukan acceptance failure yang boleh disembunyikan. Harness kini menangkap panic khusus pada fixture alpha agar karakterisasi berikutnya dapat terus berjalan dan fixture alpha parsial tetap diuji. Production code nantinya tidak boleh bergantung pada `catch_unwind` sebagai strategi normal; input alpha nol harus ditangani sebelum memasuki engine atau dependency harus diganti/diperbaiki.

## Temuan sebelum run lokal

### Cancellation/progress

Source vtracer 1.0.0-alpha.4 menyediakan cancellation dan progress native. Ini lebih cocok dari asumsi architecture awal yang menyiapkan fallback “tunggu call synchronous selesai lalu discard”. Scheduler VectorForge tetap tidak boleh menjanjikan preemption instan, tetapi adapter dapat menggunakan `CancelToken` bila hasil lokal membuktikan stabil.

### PDF

svg2pdf 0.13.0 menerima tree dari `svg2pdf::usvg`. Untuk ukuran fisik VectorForge, harness memakai DPI 96 sehingga skala PDF menjadi 72/96 point per pixel. VTracer menghasilkan path/fill sederhana tanpa filter SVG, sehingga jalur spike tidak membutuhkan fallback raster.

### EPS

IR alpha.4 memiliki `MoveTo`, `LineTo`, `CubicTo`, `Close` dan paint solid. Itu cukup untuk writer EPS v1 tanpa parser SVG tambahan. EPS VectorForge tetap mengikuti keputusan D11: input transparan harus dikomposit putih **sebelum trace**, sehingga writer hanya menerima dokumen opaque.

Writer di harness hanya karakterisasi geometri. Atomic write, race nama, file target terbuka dan overwrite aman belum diuji.

### BLOCKER — alpha nol dapat memicu panic dependency

Run Windows membuktikan VTracer/visioncortex tidak aman menerima fixture yang seluruh pixel-nya alpha 0 pada konfigurasi default spike: `visioncortex 0.9.3` membagi dengan nol di color-cluster builder.

Implikasi untuk desain:
- gambar yang seluruhnya transparan harus dideteksi pada boundary decode/normalize dan menghasilkan dokumen vektor kosong tanpa menjalankan cluster engine; atau
- dependency/engine harus diperbaiki sebelum input tersebut diteruskan.

Ini cocok dengan D10 bahwa alpha nol harus tetap kosong pada SVG/PDF. Pre-check alpha nol adalah validasi semantik, bukan fallback raster atau flatten.

### Investigasi alpha parsial SVG/PDF

Stock SVG writer vtracer 1.0.0-alpha.4 saat ini menulis `Paint::Solid(Color)` sebagai fill RGB dan tidak menulis `opacity` atau `fill-opacity`. Namun `visioncortex::Color` sendiri memiliki channel `a`, `ColorSum` menghitung rata-rata alpha, dan `Paint::Solid(Color)` membawa objek Color tersebut ke `VectorDoc`.

Harness berikutnya karena itu tidak langsung menyimpulkan IR kehilangan alpha. Ia memeriksa alpha pada `VectorDoc`, lalu memakai writer SVG karakterisasi milik VectorForge yang menyerialisasi `fill-opacity` bila alpha 1–254 dan menguji SVG itu melalui svg2pdf.

Kontrak D10 tetap belum terbukti sampai dua hal lolos: channel alpha bertahan sampai `VectorDoc`, dan segmentasi tidak menggabungkan area yang RGB-nya sama tetapi alpha-nya berbeda. Kasus kedua penting karena clustering VTracer terutama berbasis RGB. B01 tidak boleh mengubah D10 diam-diam.

Pilihan yang nanti memerlukan keputusan eksplisit bila run lokal mengonfirmasi blocker:
1. pertahankan D10 dan buat strategi alpha-aware di atas/di sekitar tracer;
2. pilih engine/pipeline lain yang mempertahankan alpha parsial;
3. ubah kontrak v1 agar alpha parsial di-flatten dengan aturan eksplisit.

Belum ada pilihan yang diambil.

## Kriteria keluar B01 yang masih terbuka

- harness berhasil compile/run pada Windows; empat jalur utama sudah PASS, tetapi run keseluruhan belum dapat dinyatakan lulus sampai karakterisasi alpha selesai tanpa process abort;
- simpan output diagnostik dan `cargo tree`;
- lengkapi lisensi transitif;
- konfirmasi hasil fixture alpha nol dan alpha parsial;
- uji atomic replace/race/file-locked pada filesystem Windows;
- putuskan blocker alpha bila masih ada;
- setelah itu baru lock versi dan izinkan B02.

Jangan membuat `src-tauri` produksi atau menghubungkan adapter frontend sebelum daftar ini selesai.


### Karakterisasi lanjutan alpha

Commit spike berikutnya menambahkan:
- pre-check seluruh alpha=0 → SVG kosong tanpa memanggil tracer, sesuai D10 sekaligus menghindari panic dependency;
- inspeksi nilai alpha di `VectorDoc` untuk fixture alpha parsial;
- writer SVG karakterisasi yang menulis `fill-opacity` dari `Paint::Solid(Color).a`;
- konversi SVG alpha-aware tersebut ke PDF;
- fixture dua bidang dengan RGB identik dan alpha 128/255. Fixture terakhir menentukan apakah alpha bisa dipertahankan hanya dengan writer sendiri atau membutuhkan frontend/segmentasi alpha-aware.


### Kandidat strategi tanpa fork engine: split mask berdasarkan alpha

Hasil Windows menunjukkan stock VTracer menggabungkan dua area yang RGB-nya identik tetapi alpha-nya 128 dan 255 menjadi satu paint dengan alpha rata-rata 191. Karena `Segmentation`, `Layer`, dan `RegionMask` public, spike berikutnya menguji strategi yang lebih terarah:

1. jalankan color clustering VTracer satu kali;
2. untuk setiap layer/mask hasil clustering, baca alpha asli pada pixel sumber;
3. pecah mask menjadi sub-mask per nilai alpha yang hadir;
4. pertahankan RGB paint VTracer, tetapi set alpha paint sesuai sub-mask;
5. baru jalankan `Pipeline::finish` untuk curve fitting/compose/optimize;
6. serialize dengan writer SVG alpha-aware VectorForge.

Spike menambah dependency langsung `visioncortex = 0.9.3` hanya karena `BinaryImage` diperlukan untuk membangun sub-mask. Ini penggunaan API nyata, bukan dependency transitif yang ditambahkan tanpa alasan.

Versi karakterisasi memakai nilai alpha exact agar correctness mudah dibuktikan. Itu **belum** keputusan produksi: gambar dengan banyak level alpha dapat memperbanyak layer/mask dan perlu benchmark/resource budget sebelum B01 ditutup.
