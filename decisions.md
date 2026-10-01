# Decisions — Keputusan Tetap dan Risiko

## Keputusan revisi 1.1

| ID | Keputusan | Alasan / konsekuensi |
|---|---|---|
| D01 | Windows desktop, lokal, tanpa login/server | Scope awal dan privasi |
| D02 | Tauri 2 + React 18 + TS + Tailwind + Zustand + Rust | Pertahankan stack dokumen awal; versi konkret diverifikasi sebelum implementasi |
| D03 | Satu file satu tanggung jawab, tanpa limit baris | Arahan eksplisit user menggantikan aturan lama |
| D04 | Workspace studio, parameter kiri, batch bawah | Canvas luas dan alur single/batch konsisten |
| D05 | Native Windows title bar; dark default, opsi light/system | Kontrol jendela familiar dan tematization terencana |
| D06 | Preview kecil, export sumber penuh | Kinerja editing dan kualitas hasil dipisahkan |
| D07 | Progress berbasis tahap nyata | Engine belum dibuktikan memberi persen per path |
| D08 | Cancellation kooperatif, discard stale result | Tidak menjanjikan interupsi instan pada engine synchronous |
| D09 | IPC gunakan fileId dari registry Rust | Mengurangi akses path arbitrer dari frontend |
| D10 | Alpha nol tetap kosong pada SVG/PDF; alpha parsial diuji | Tidak boleh mengklaim dukungan alpha yang belum dibuktikan |
| D11 | EPS flatten transparansi ke putih sebelum trace | EPS v1 solid fill; tetap vektor, berbeda secara eksplisit dari SVG/PDF |
| D12 | Satu batch aktif; retry memakai snapshot asal | Konsistensi output dan resource budget |
| D13 | Sumber >30 MP ditolak; tidak auto resize | Pilih cabang paling sederhana dari opsi PRD lama; kualitas export tidak berubah diam-diam |
| D14 | Engine/output limitations menjadi blocker, bukan silent fallback | Hindari file raster berbungkus SVG/PDF |

## Pertanyaan teknis yang wajib diselesaikan B01

1. API versi kompatibel vtracer/visioncortex/image/usvg/svg2pdf; lisensi dan kebutuhan distribusi. Catat URL dokumentasi resmi, tanggal, versi dan hasil spike. Jangan mengandalkan contoh API versi lama.
2. Seberapa jauh alpha parsial dapat dipertahankan oleh pipeline SVG/PDF? Fixture semi-transparent wajib. Jika engine tidak mendukung, laporkan blocker dan minta keputusan alternatif (misalnya flatten eksplisit). Alpha parsial tidak boleh diam-diam dihapus.
3. Apakah tracing menyediakan cancellation/progress callback? Baseline aman: scheduler tidak memulai job baru, menunggu panggilan aktif kembali, lalu membuang output cancelled. Jangan fork engine hanya untuk cancellation tanpa persetujuan.
4. EPS writer: dukungan path, fill rule, affine transform, orientation dan bounding box pada hasil engine. Fitur unsupported mengembalikan error, tidak menghasilkan file rusak.
5. Atomic replace Windows, antisipasi race nama output dan file target terbuka. Uji di filesystem Windows nyata.
6. Minimum OS/WebView2/toolchain yang didukung versi terpilih, ukuran installer, DPI dan konsumsi memori.

## Di luar v1

Editor node/path, Potrace khusus B/W, cloud sync, AI generation/upscale/remove background, akun/subscription, auto updater, shell Explorer integration, CLI publik, macOS/Linux, multi-window, project format, resume batch setelah restart. Jangan implementasi tanpa arahan baru.

## Pengendalian perubahan

Untuk perubahan keputusan: tulis masalah, bukti, opsi dan konsekuensi; minta persetujuan hanya bila melampaui scope/kontrak yang disetujui. Setelah disetujui, ubah sumber kebenaran dan dokumen yang bergantung padanya dalam perubahan yang sama. Catat keputusan baru dengan ID; jangan menyimpan dua versi aturan aktif.
