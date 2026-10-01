# AGENTS.md — Instruksi Agent VectorForge

Baca file ini dahulu. Berlaku untuk seluruh proyek. **Satu file satu tanggung jawab; tidak ada batas jumlah baris.** Nama kanonis file ini adalah `AGENTS.md`, bukan duplikat `agents.md`.

## Tujuan tetap

Bangun aplikasi Windows offline raster → vektor full color, sesuai PRD. Prioritas: hasil benar, UI responsif, mudah digunakan, lalu optimasi yang terukur. Jangan memperluas menjadi editor desain, layanan cloud atau aplikasi AI generatif.

## Urutan membaca

1. `rules.md` untuk batas kerja.
2. `plan.md` untuk fase aktif, izin dan next task.
3. `prd.md` untuk acceptance criteria.
4. `design.md`, `architecture.md`, `schema.md`, atau `testing.md` sesuai file yang akan disentuh.
5. `decisions.md` sebelum mengganti keputusan teknis.

Instruksi eksplisit user terbaru menjadi prioritas atas dokumen ini. Setiap dokumen hanya otoritatif di domainnya (lihat README). Bila dua dokumen proyek bertentangan, jangan diam-diam memilih: catat konflik, selesaikan berdasarkan keputusan user terakhir, lalu sinkronkan. Jangan menciptakan persetujuan user.

## Sebelum kode

Nyatakan fase dan task ID, acceptance criteria, file yang akan berubah dan bukti verifikasi yang relevan. Periksa struktur nyata dan implementasi yang sudah ada. Bila scope sudah jelas, kerjakan tanpa pertanyaan tambahan untuk keputusan rutin. Update dokumen kontrak dahulu bila perubahan sah membutuhkan kontrak baru.

## Fase wajib

P0 dokumen → P1 frontend mock → checkpoint review user → P2 backend → checkpoint review user → P3 integrasi → P4 hardening/rilis.

Jangan membuat engine Rust sambil mengerjakan frontend mock. Jangan mengganti mock menjadi engine produksi sebelum backend disetujui. User dapat memberi izin eksplisit beberapa fase sekaligus; catat izin tersebut di plan dan tidak perlu meminta ulang.

## Batas otonomi

Boleh: detail implementasi dalam scope, perbaikan bug tugas aktif, refactor lokal yang diperlukan agar tugas selesai, penambahan tes berisiko nyata, melengkapi dokumentasi yang terdampak.

Perlu keputusan user: mengganti stack/engine, menambah jaringan atau server, menurunkan fitur v1, mengubah semantik output, fitur baru, menjalankan fase yang belum diizinkan. Laporkan bug di luar scope dan dampaknya; jangan sekaligus merombak modul lain.

Jika terblokir dependency/OS/akses: tulis bukti dan batasan, lanjutkan pekerjaan independen yang aman, jangan membuat klaim pengujian palsu. Jangan memakai fallback raster dalam file SVG/PDF dan menyebutnya vektor.

## Laporan akhir tiap task

- Task ID dan hasil perilaku pengguna.
- File utama yang berubah dan alasan.
- Tes yang dijalankan serta hasil; bedakan belum diuji dan gagal.
- Risiko/blocker tersisa, status fase, next task.
- Apakah checkpoint membutuhkan persetujuan; kutip instruksi pemicunya bila berhenti.

Update status hanya setelah ada bukti. “Ditulis” tidak sama dengan “teruji”. Jangan mengarang hash commit, benchmark, persetujuan, atau screenshot. Tidak perlu membuat file laporan per tugas; gunakan `plan.md` agar riwayat tidak tersebar.

## Status repository saat bootstrap

Frontend demo sudah tersedia; baca status terkini di plan.md dan README.md. Jangan menganggap seluruh F02–F05 DONE hanya karena build lulus. Kerjakan backlog P1/G1 sebelum meminta izin backend.
