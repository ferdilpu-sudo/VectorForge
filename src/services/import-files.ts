import type { SourceFile } from "../types/project";

export type ImportFileErrorCode =
  | "UNSUPPORTED_FORMAT"
  | "INVALID_HEADER"
  | "ANIMATED_UNSUPPORTED"
  | "IMAGE_TOO_LARGE"
  | "CORRUPT_IMAGE";

export class ImportFileError extends Error {
  constructor(readonly code: ImportFileErrorCode) {
    super(code);
    this.name = "ImportFileError";
  }
}

export function importFileErrorMessage(
  error: unknown,
  language: "id" | "en",
): string {
  const code =
    error instanceof ImportFileError ? error.code : "CORRUPT_IMAGE";
  const messages: Record<ImportFileErrorCode, [string, string]> = {
    UNSUPPORTED_FORMAT: ["Format tidak didukung", "Unsupported format"],
    INVALID_HEADER: ["Header gambar tidak valid", "Invalid image header"],
    ANIMATED_UNSUPPORTED: [
      "Gambar animasi tidak didukung",
      "Animated images are unsupported",
    ],
    IMAGE_TOO_LARGE: ["Gambar melebihi 30 MP", "Image exceeds 30 MP"],
    CORRUPT_IMAGE: ["Gambar rusak atau gagal dibaca", "Corrupt or unreadable image"],
  };
  return messages[code][language === "id" ? 0 : 1];
}

async function sampledContentFingerprint(file: File): Promise<string> {
  const chunkSize = 64 * 1024;
  const middleStart = Math.max(
    0,
    Math.floor(file.size / 2) - Math.floor(chunkSize / 2),
  );
  const tailStart = Math.max(0, file.size - chunkSize);
  const chunks = await Promise.all([
    file.slice(0, chunkSize).arrayBuffer(),
    file.slice(middleStart, middleStart + chunkSize).arrayBuffer(),
    file.slice(tailStart).arrayBuffer(),
  ]);
  const metadata = new TextEncoder().encode(`${file.size}:${file.type}`);
  const byteLength =
    metadata.byteLength + chunks.reduce((sum, chunk) => sum + chunk.byteLength, 0);
  const sample = new Uint8Array(byteLength);
  let offset = 0;
  sample.set(metadata, offset);
  offset += metadata.byteLength;
  for (const chunk of chunks) {
    const bytes = new Uint8Array(chunk);
    sample.set(bytes, offset);
    offset += bytes.byteLength;
  }
  const digest = new Uint8Array(await crypto.subtle.digest("SHA-256", sample));
  return Array.from(digest, (byte) => byte.toString(16).padStart(2, "0")).join("");
}

export async function readBrowserFile(file: File): Promise<SourceFile> {
  if (!/\.(png|jpe?g|webp|bmp)$/i.test(file.name))
    throw new ImportFileError("UNSUPPORTED_FORMAT");
  const b = new Uint8Array(await file.slice(0, 32).arrayBuffer());
  const signature =
    (b[0] === 137 && b[1] === 80 && b[2] === 78 && b[3] === 71) ||
    (b[0] === 255 && b[1] === 216) ||
    (b[0] === 66 && b[1] === 77) ||
    (String.fromCharCode(...b.slice(0, 4)) === "RIFF" &&
      String.fromCharCode(...b.slice(8, 12)) === "WEBP");
  if (!signature) throw new ImportFileError("INVALID_HEADER");
  await rejectAnimation(file, b);

  // Browser decode is a demo boundary. Rust must enforce pre-decode limits in B02.
  const url = URL.createObjectURL(file);
  try {
    const image = new Image();
    image.src = url;
    await image.decode();
    if (image.naturalWidth * image.naturalHeight > 30_000_000)
      throw new ImportFileError("IMAGE_TOO_LARGE");
    return {
      id: crypto.randomUUID(),
      name: file.name,
      width: image.naturalWidth,
      height: image.naturalHeight,
      bytes: file.size,
      previewUrl: url,
      fingerprint: await sampledContentFingerprint(file),
    };
  } catch (error) {
    URL.revokeObjectURL(url);
    if (error instanceof ImportFileError) throw error;
    throw new ImportFileError("CORRUPT_IMAGE");
  }
}

async function rejectAnimation(file: File, header: Uint8Array): Promise<void> {
  const png = header[0] === 137;
  const webp = String.fromCharCode(...header.slice(8, 12)) === "WEBP";
  if (!png && !webp) return;
  let offset = png ? 8 : 12;
  while (offset + 8 <= file.size) {
    const raw = await file.slice(offset, offset + 8).arrayBuffer();
    const bytes = new Uint8Array(raw);
    const view = new DataView(raw);
    const tag = String.fromCharCode(...bytes.slice(png ? 4 : 0, png ? 8 : 4));
    if (tag === "acTL" || tag === "ANIM" || tag === "ANMF")
      throw new ImportFileError("ANIMATED_UNSUPPORTED");
    const length = view.getUint32(png ? 0 : 4, !png);
    offset += png ? length + 12 : 8 + length + (length % 2);
    if (png && tag === "IEND") return;
  }
}
