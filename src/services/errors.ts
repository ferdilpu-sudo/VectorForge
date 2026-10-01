import type { AppError } from "../types/project";

function errorCode(error: unknown): AppError["code"] | undefined {
  if (
    typeof error === "object" &&
    error !== null &&
    "code" in error &&
    typeof error.code === "string"
  ) {
    return error.code as AppError["code"];
  }
  return undefined;
}

export function appErrorMessage(
  error: unknown,
  language: "id" | "en",
): string {
  const code = errorCode(error);
  const messages: Partial<Record<AppError["code"], [string, string]>> = {
    FILE_NOT_FOUND: ["File tidak ditemukan", "File not found"],
    UNSUPPORTED_FORMAT: ["Format tidak didukung", "Unsupported format"],
    IMAGE_CORRUPT: ["Gambar rusak atau gagal dibaca", "Corrupt or unreadable image"],
    IMAGE_TOO_LARGE: ["Gambar melebihi 30 MP", "Image exceeds 30 MP"],
    INVALID_PARAMS: ["Parameter tidak valid", "Invalid parameters"],
    TRACE_FAILED: ["Tracing gagal", "Tracing failed"],
    EXPORT_FAILED: ["Ekspor gagal", "Export failed"],
    WRITE_FAILED: ["File output gagal ditulis", "Could not write output file"],
    CANCELLED: ["Operasi dibatalkan", "Operation cancelled"],
    PRESET_DUPLICATE: ["Nama preset sudah digunakan", "Preset name is already used"],
    PRESET_READ_ONLY: ["Preset bawaan tidak dapat dihapus", "Built-in preset is read-only"],
    NOT_FOUND: ["Data tidak ditemukan", "Data not found"],
    ACCESS_DENIED: ["Akses file ditolak", "File access denied"],
    SOURCE_CHANGED: ["File sumber telah berubah", "Source file has changed"],
    BATCH_BUSY: ["Batch lain masih berjalan", "Another batch is still running"],
    INVALID_STATE: ["Status aplikasi tidak valid", "Invalid application state"],
    OUTPUT_TOO_LARGE: [
      "SVG melebihi batas ukuran dan perlu konfirmasi",
      "SVG exceeds the size limit and needs confirmation",
    ],
    DATA_VERSION_UNSUPPORTED: [
      "Versi data lebih baru tidak didukung",
      "Newer data version is unsupported",
    ],
    DATA_CORRUPT: ["Data lokal rusak", "Local data is corrupt"],
  };

  if (code && messages[code]) {
    return messages[code]![language === "id" ? 0 : 1];
  }

  if (
    typeof error === "object" &&
    error !== null &&
    "message" in error &&
    typeof error.message === "string" &&
    error.message.trim()
  ) {
    return error.message;
  }

  return language === "id"
    ? "Operasi VectorForge gagal."
    : "VectorForge operation failed.";
}

export function hasAppErrorCode(
  error: unknown,
  code: AppError["code"],
): boolean {
  return errorCode(error) === code;
}
