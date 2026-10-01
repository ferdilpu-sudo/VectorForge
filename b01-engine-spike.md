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
6. sumber alpha parsial.

Harness tidak menulis output ke disk dan tidak menguji atomic replace Windows. Uji filesystem nyata tetap item B01 terpisah.

## Temuan sebelum run lokal

### Cancellation/progress

Source vtracer 1.0.0-alpha.4 menyediakan cancellation dan progress native. Ini lebih cocok dari asumsi architecture awal yang menyiapkan fallback “tunggu call synchronous selesai lalu discard”. Scheduler VectorForge tetap tidak boleh menjanjikan preemption instan, tetapi adapter dapat menggunakan `CancelToken` bila hasil lokal membuktikan stabil.

### PDF

svg2pdf 0.13.0 menerima tree dari `svg2pdf::usvg`. Untuk ukuran fisik VectorForge, harness memakai DPI 96 sehingga skala PDF menjadi 72/96 point per pixel. VTracer menghasilkan path/fill sederhana tanpa filter SVG, sehingga jalur spike tidak membutuhkan fallback raster.

### EPS

IR alpha.4 memiliki `MoveTo`, `LineTo`, `CubicTo`, `Close` dan paint solid. Itu cukup untuk writer EPS v1 tanpa parser SVG tambahan. EPS VectorForge tetap mengikuti keputusan D11: input transparan harus dikomposit putih **sebelum trace**, sehingga writer hanya menerima dokumen opaque.

Writer di harness hanya karakterisasi geometri. Atomic write, race nama, file target terbuka dan overwrite aman belum diuji.

### BLOCKER — alpha parsial SVG/PDF

Stock SVG writer vtracer 1.0.0-alpha.4 saat ini menulis `Paint::Solid(Color)` sebagai fill RGB. IR `Paint` juga hanya memiliki varian solid; writer tidak menulis `opacity` atau `fill-opacity`.

Artinya kontrak D10 untuk **alpha parsial pada SVG/PDF belum terbukti dan secara source inspection tampak tidak dipertahankan oleh stock writer**. B01 tidak boleh mengubah D10 diam-diam.

Pilihan yang nanti memerlukan keputusan eksplisit bila run lokal mengonfirmasi blocker:
1. pertahankan D10 dan buat strategi alpha-aware di atas/di sekitar tracer;
2. pilih engine/pipeline lain yang mempertahankan alpha parsial;
3. ubah kontrak v1 agar alpha parsial di-flatten dengan aturan eksplisit.

Belum ada pilihan yang diambil.

## Kriteria keluar B01 yang masih terbuka

- `cargo check` dan `cargo run` lulus pada Windows pengguna;
- simpan output diagnostik dan `cargo tree`;
- lengkapi lisensi transitif;
- konfirmasi hasil fixture alpha nol dan alpha parsial;
- uji atomic replace/race/file-locked pada filesystem Windows;
- putuskan blocker alpha bila masih ada;
- setelah itu baru lock versi dan izinkan B02.

Jangan membuat `src-tauri` produksi atau menghubungkan adapter frontend sebelum daftar ini selesai.
