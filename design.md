# Design — Studio Desktop VectorForge

Status: keputusan desain untuk implementasi, belum merupakan hasil usability testing.

## Tujuan dan susunan

Pengguna pemula membuka gambar, memilih preset dan mengekspor; pengguna mahir mengatur parameter dan batch dalam workspace yang sama. Preview adalah fokus visual. Bahasa Indonesia default, Inggris tersedia. Tidak ada dashboard, onboarding carousel, kartu statistik besar, login atau layar marketing.

| Area | Ukuran acuan CSS px | Tanggung jawab |
|---|---:|---|
| Title bar OS | native | Kontrol jendela Windows; terpisah dari toolbar aplikasi |
| Toolbar | 52 | Brand kecil, Buka Gambar, nama file aktif, Pengaturan, Ekspor… |
| Panel parameter kiri | 288, dapat dilipat | Preset dan pengaturan; scroll internal |
| Canvas | sisa ruang | Asli/Bandingkan/Hasil, zoom, pan, background |
| Antrean di bawah canvas | 144–240, dapat dilipat/resize | Thumbnail, status, pengaturan batch, aksi |
| Status bar | 28 | Dimensi, info preview, status singkat |

Panel kiri tidak tertimpa antrean. Antrean muncul saat ada lebih dari satu file, tanpa memindahkan pengguna ke halaman baru. Item aktif tidak berubah otomatis ketika worker batch selesai.

Target kenyamanan 1366×768 physical pixels pada Windows scale 100/125/150%. Uji juga viewport efektif yang mengecil akibat DPI dan tinggi title bar; jangan menganggap physical pixels sama dengan CSS pixels. Pada lebar konten <1000 px, panel dapat dilipat dan antrean awalnya ringkas. Pada tinggi <650 px, ringkas antrean, scroll panel; toolbar, aksi inti, dan canvas tetap dapat dijangkau. Hindari minimum window size besar yang membuat aplikasi keluar layar pada skala 150%.

## Tokens

| Token | Dark default | Light |
|---|---|---|
| bg | #17191D | #F3F4F6 |
| surface | #22252B | #FFFFFF |
| control | #2C3038 | #E8EBF0 |
| text | #F3F4F6 | #17191D |
| textMuted | #B7BEC9 | #505866 |
| primary | #245FCC | #245FCC |
| onPrimary | #FFFFFF | #FFFFFF |
| focus | #75A7FF | #245FCC |

Palet adalah nilai awal; uji kontras pasangan yang benar-benar dipakai, termasuk hover, disabled dan fokus sebelum menyatakan aksesibilitas lulus. Warna status harus disertai ikon dan label.

Font Segoe UI dengan fallback sans-serif lokal, tidak unduh font. Kontrol 13–14 px, judul panel 15–16 px, teks pendamping minimal 12 px. Line height sekitar 1.4. Spacing 4/8/12/16/24/32. Radius kontrol 6–8 px. Tinggi kontrol 34–36 px, focus ring terlihat. Panel solid, border halus; shadow untuk menu/dialog. Ikon satu gaya, label untuk aksi utama. Animasi transisi singkat 120–180 ms, hormati reduced motion.

## Panel parameter

1. Dropdown preset (empat bawaan + user), Simpan Preset.
2. Utama: Detail Warna, Buang Noda Kecil, Mode Kurva.
3. Accordion Lanjutan: Selisih Layer, Sudut Tajam, Panjang Segmen, Susunan Layer.
4. Reset Parameter, toggle Preview Otomatis. Bila otomatis mati, tombol Perbarui Preview tersedia.

Rentang dan default hanya dari schema. Slider dilengkapi input angka dan tooltip/helper text; nilai invalid memiliki pesan yang jelas. Preset diubah menampilkan “Nama · Diubah”; menyimpan menghasilkan preset user, tidak mengubah bawaan. Reset kembali ke default Seimbang; memilih ulang preset mengembalikan nilai preset tersebut. Nama preset 1–40 karakter setelah trim, unik case-insensitive. Hapus hanya preset user dengan konfirmasi.

## Canvas dan preview

- Default Bandingkan: kiri Asli, kanan Hasil; label tetap. Asli dan Hasil merupakan alternatif yang tidak perlu drag.
- Pembatas dapat dipindah keyboard, target hit area cukup besar meski garisnya tipis.
- Zoom 10–800%, tombol +/- dan input %, Pas ke Layar. Wheel zoom terpusat pada pointer; pan Space+drag dengan alternatif keyboard ketika canvas fokus. Shortcut tidak mengambil alih input teks.
- Transform asli dan hasil selalu sama. Memperbarui parameter tidak mereset zoom/pan; mengganti file memulai fit-to-view.
- Latar checkerboard/putih/gelap adalah tampilan saja, tidak mengubah output.
- Preview terakhir tetap terlihat saat processing dengan label Memperbarui preview…; hasil lama bertanda Belum sesuai parameter terbaru. Respons lama tidak boleh menimpa yang baru.
- Metadata berlabel Preview: jumlah path, warna, ukuran SVG preview, dimensi preview. Dimensi sumber ditampilkan terpisah. “Preview diperkecil. Ekspor menggunakan resolusi penuh.”
- Tidak ada hasil lama dari file A yang tampil sebagai hasil file B.

## State dan copy

| Kondisi | Tampilan / tindakan |
|---|---|
| Belum ada gambar | Tarik gambar ke sini; Pilih Gambar; format input; Diproses di perangkat Anda |
| File dibaca | Membaca gambar…; kontrol yang bergantung data nonaktif |
| Tracing | Memperbarui preview…; kontrol parameter tetap bisa diubah |
| File invalid | Nama file dan sebab spesifik; Pilih Gambar Lain |
| Preview gagal | Preview gagal dibuat; Coba Lagi; jangan tampilkan hasil gagal sebagai sukses |
| Export berjalan | Menyiapkan hasil resolusi penuh…; cegah submit ganda |
| Export sukses | File tersimpan; Buka Folder |
| Disk error | Tidak dapat menyimpan ke folder ini; Pilih Folder Lain |
| Membatalkan batch | Membatalkan… Menunggu proses aktif berhenti |

Phase mock wajib menampilkan banner Mode Demo — engine belum terhubung; contoh output diberi label ilustrasi. Jangan menamai posterisasi raster sebagai hasil tracing sungguhan. Mock ekspor/batch tidak mengklaim menyimpan output produksi.

## Export

Ekspor… membuka dialog format SVG/PDF/EPS untuk file aktif lalu Save As native. SVG default. Selalu trace sumber penuh dengan snapshot parameter saat klik. Input boleh tetap ditinjau, tetapi snapshot job tidak berubah. Perubahan parameter berikutnya tidak memengaruhi export yang sedang berjalan.

Peringatan SVG besar >50 MiB: tampilkan ukuran aktual dan minta lanjut/simpan atau batal sebelum commit. EPS memakai flatten transparansi ke putih dengan informasi eksplisit pada dialog (lihat keputusan). Konflik file meminta konfirmasi overwrite native untuk single. Setelah sukses, Buka Folder hanya menggunakan lokasi hasil yang sudah dikenal Rust.

## Batch

Satu parameter snapshot berlaku untuk semua file. Ringkasan jumlah file, preset/parameter, format dan folder terlihat sebelum Mulai Batch. Default tidak overwrite. Checkbox format wajib minimal satu. Multi-drop menambahkan file valid, menampilkan daftar penolakan invalid tanpa membuang file valid. Path duplikat tidak ditambah dua kali dalam antrean yang sama.

Status: Menunggu/Proses/Selesai/Sebagian gagal/Gagal/Dibatalkan. Progress total berupa item terminal dibagi total; ringkasan sukses dan gagal tetap terpisah. Progress item menggunakan tahap nyata Membaca/Menelusuri/Menyimpan; indeterminate ketika engine tidak menyediakan persen.

Saat aktif, konfigurasi dan anggota batch terkunci; pengguna masih boleh inspeksi hasil. Coba Lagi setelah batch terminal hanya memproses output gagal. Batalkan tidak menghapus output yang telah berhasil. Menghapus item antrean tidak menghapus file sumber/hasil. Menutup aplikasi saat pekerjaan aktif meminta konfirmasi berhenti atau tetap menunggu.

## Pengaturan dan aksesibilitas

Tema gelap/terang/sistem; bahasa ID/EN; worker 1–4; preview otomatis; sisi preview 512–2048; reset pengaturan dengan konfirmasi. Tidak ada pengaturan jaringan.

Tab order logis, nama aksesibel semua icon button, modal focus trap dan restore focus, Escape menutup modal non-blocking, slider dapat keyboard, focus tidak tersembunyi. Pembaruan status lewat live region yang tidak berbicara tiap slider tick. Error terkait field. Jangan mengandalkan warna atau hover saja.

Ctrl+O buka, Ctrl+Shift+E ekspor, Ctrl+0 fit, +/- zoom hanya saat canvas fokus. Shortcut selalu punya tombol/menu alternatif. Teks ID/EN panjang dan filename panjang tidak boleh mendorong aksi utama keluar layar; ellipsis dengan cara melihat nama penuh.
