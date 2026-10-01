# Rules — Batas Implementasi

## Struktur kode

- **Tidak ada hard limit baris. Satu file satu tanggung jawab.** Ukuran bukan alasan tunggal untuk memecah file. Pisahkan bila file memiliki alasan berubah yang berbeda, ketergantungan yang tidak berkaitan, atau sulit diuji karena mencampur domain.
- UI, orchestration, tracing, export, persistence dan validasi adalah tanggung jawab terpisah. App entry point hanya komposisi.
- Hindari abstraksi spekulatif, utility serba guna, barrel berlebihan dan file tipis yang hanya meneruskan argumen tanpa manfaat.
- Edit bug di sumbernya. Jangan menambah stylesheet/fungsi `fix`, `patch`, `override`, `v2` yang menimpa implementasi lama.
- TypeScript strict; hindari `any` dan non-null assertion tanpa alasan yang dapat dibuktikan. Rust: tidak ada `unwrap/expect` untuk input, disk, decoding atau operasi pengguna yang bisa gagal.
- `schema.md` menjadi sumber kontrak; JSON camelCase, enum sesuai dokumen. Kontrak TS/Rust diuji dengan fixture serialisasi.

## Scope dan dependency

Jangan upgrade mayor, migrasi framework, memasang SDK jaringan, updater, telemetry, auth, database server, AI API, payment, editor node, plugin system atau dukungan OS lain tanpa keputusan user. Dependency dipilih untuk kebutuhan nyata, diverifikasi dokumentasi resminya, dikunci dalam lockfile, dan dicatat lisensinya. Jangan menulis versi “latest” sebagai spesifikasi reproduksibel.

Tidak ada pembatasan file 150/200/300 baris dari pedoman lama yang berlaku. Seluruh dokumentasi harus konsisten dengan aturan ini.

## Perilaku dan keamanan

- Runtime offline. Development boleh memperoleh dependency/dokumentasi. WebView2 bootstrapper merupakan kebutuhan instalasi terpisah dan harus dijelaskan, bukan disamarkan sebagai offline installer penuh.
- Rust memvalidasi parameter, header gambar, dimensi dan akses file. UI validation hanya untuk feedback cepat.
- Tolak angka non-finite dan out-of-range; **jangan clamp diam-diam** nilai IPC yang salah.
- Jangan inject SVG mentah dengan innerHTML. Preview hasil melalui URL blob ke img; revoke URL saat tidak digunakan.
- Akses file melalui dialog/drop yang diizinkan dan registry file internal; jangan menerima path arbitrer dari JavaScript tanpa otorisasi.
- Permission plugin dan CSP seminimal mungkin; tidak ada wildcard filesystem/global shell atau remote content. CSP mengizinkan img blob/data bila dibutuhkan, hanya dev yang boleh memakai koneksi dev server.
- Worker berat tidak berjalan pada UI thread. Tidak ada persentase/progress, waktu, ukuran file atau jumlah path buatan pada produksi.
- Tidak overwrite output diam-diam; tidak merusak input; output lewat temporary file pada folder tujuan lalu commit/rename yang aman di Windows.
- Log terstruktur lokal dengan rotasi. Tidak log isi gambar, SVG lengkap, credential; kurangi path absolut pada log normal. Pesan pengguna ramah dan detail teknis tetap lokal.

## Definisi selesai

Acceptance criteria tugas terpenuhi; empty/loading/error/success dan keyboard relevan tersedia; build/typecheck/lint sesuai fase lulus; tes risiko yang relevan lulus atau blocker dinyatakan; kontrak/dokumen selaras; tidak ada debug logging atau perubahan di luar scope. Kegagalan tes tidak diatasi dengan menonaktifkan tes atau melonggarkan validasi.
