# PRD — VectorForge v1

Revisi 1.1. Spesifikasi target; seluruh kemampuan perlu dibuktikan lewat implementasi dan tes.

## Produk dan pengguna

Aplikasi Windows offline untuk desainer, pembuat logo/stiker/aset game dan pengguna umum yang ingin raster → vektor berwarna tanpa upload. Alur inti: buka gambar, pilih preset, periksa before/after, ekspor. Bukan editor node/path. Tidak ada server, akun atau telemetry.

## Fitur dan acceptance criteria

| ID | Fitur | Kriteria penerimaan |
|---|---|---|
| AC01 | Import PNG/JPG/JPEG/WebP/BMP | Dialog dan drop native; validasi header/dimensi; sumber corrupt/tidak didukung ditolak dengan sebab; multi-import tidak membuang file valid |
| AC02 | Compare | Asli/Bandingkan/Hasil; posisi dan zoom identik; zoom 10–800%; pan dan alternatif keyboard; tidak tersendat saat drag karena tracing |
| AC03 | Parameter | Seluruh field/rentang schema; debounce 300 ms; default preview maxSide 1024; reset, preset built-in/user; latest-result-wins |
| AC04 | Export | SVG, PDF, EPS vektor nyata pada resolusi penuh; Save As; nama dasar sumber; sukses hanya setelah file committed; Buka Folder |
| AC05 | Batch | Multi-format, snapshot parameter, worker maksimum min(setting, 4, logical cores) dan dapat dikurangi budget memori; cancel; retry output gagal; item gagal tidak menghentikan lainnya |
| AC06 | Settings | ID/EN, tema, worker, autoPreview, previewMaxSide, lastOutDir/preset; persistence lokal versioned dan aman |
| AC07 | Privacy | Tidak ada request jaringan runtime; semua pemrosesan lokal; permissions terbatas |
| AC08 | Recovery | File hilang/berubah, disk penuh, folder readonly, collision dan partial output ditangani tanpa merusak sumber |
| AC09 | Accessibility | Alur utama keyboard, fokus jelas, label/status tekstual, perbandingan tanpa drag, DPI/teks panjang tetap dapat dipakai |

## Parameter

Sumber angka/default kanonis: `schema.md`. UI tidak menyalin default secara terpisah di banyak komponen. Semua format memakai parameter yang sama; EPS memiliki kebijakan transparansi yang dijelaskan di bawah.

## Output dan transparansi

SVG/PDF: area alpha nol harus tetap transparan; alpha parsial wajib diverifikasi pada B01. Jika tidak dapat dipertahankan, ini blocker kemampuan, bukan izin flatten diam-diam. Hasil trace merupakan aproksimasi bentuk/warna sumber, bukan janji reproduksi foto lossless.

EPS v1: solid fill, tidak mendukung opacity/gradient sebagai fitur output. Sumber transparan dikomposit ke putih sebelum tracing khusus EPS, sehingga output tetap path vektor. Dialog menjelaskan latar putih; jangan sekadar mengganti ekstensi SVG. PDF mengikuti ukuran gambar dengan pemetaan 96 px = 72 pt dan orientation yang konsisten. SVG viewBox mengikuti dimensi sumber penuh.

## Batas eksplisit

- Sumber lebih dari 30.000.000 pixel ditolak dengan saran resize di luar aplikasi; tidak auto-downscale ekspor.
- Input animasi/multi-frame ditolak untuk v1; jangan diam-diam mengambil frame pertama.
- Warna kerja sRGB; metadata warna/orientasi diproses konsisten pada preview dan ekspor. Kebijakan profil warna yang tidak didukung dijelaskan, bukan klaim fidelity cetak CMYK.
- Preview SVG di atas 50 MiB tidak dirender otomatis: tampilkan peringatan dan saran kurangi detail. Export SVG di atas 50 MiB membutuhkan konfirmasi sebelum commit. Batch menandai format tersebut gagal OUTPUT_TOO_LARGE agar tidak memunculkan puluhan dialog.
- Batch tidak dipulihkan otomatis setelah restart. Hasil yang sudah tersimpan tetap ada.
- Progress persen tracing hanya jika API engine benar-benar menyediakannya; baseline UI memakai tahap + indikator tidak tentu.

## Target nonfungsional (belum terukur)

Installer <25 MB di luar runtime WebView2; startup <2 s; preview tipikal sisi 1024 <1,5 s; memori skenario single 20 MP <500 MB; 50 gambar 2 MP selesai tanpa UI freeze. Lihat testing untuk metode. Windows 10 (target awal 1809+)/11 x64, minimum nyata wajib sesuai dependency dan uji distribusi. Offline berlaku pada aplikasi; instalasi dengan bootstrapper dapat memerlukan internet.

## Selesai v1

AC01–AC09 diverifikasi, hasil terbuka di browser (SVG), Inkscape dan Illustrator sesuai format dukungannya, error kritis tidak tersisa, Windows installer smoke test lulus. Jika aplikasi eksternal tidak tersedia, tes itu berstatus belum diuji dan tidak boleh dilaporkan lulus. Target UX: pengguna baru menyelesaikan single drop → preview → export <30 detik pada fixture ringan; pisahkan waktu interaksi dan waktu tracing.
