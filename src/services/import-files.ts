import type { SourceFile } from "../types/project";
export async function readBrowserFile(file: File): Promise<SourceFile> {
  if (!/\.(png|jpe?g|webp|bmp)$/i.test(file.name))
    throw new Error("Format tidak didukung / Unsupported format");
  const b = new Uint8Array(await file.slice(0, 32).arrayBuffer());
  const signature =
    (b[0] === 137 && b[1] === 80 && b[2] === 78 && b[3] === 71) ||
    (b[0] === 255 && b[1] === 216) ||
    (b[0] === 66 && b[1] === 77) ||
    (String.fromCharCode(...b.slice(0, 4)) === "RIFF" &&
      String.fromCharCode(...b.slice(8, 12)) === "WEBP");
  if (!signature)
    throw new Error("Header gambar tidak valid / Invalid image header");
  await rejectAnimation(file, b);
  // Browser decode is a demo boundary. Rust must enforce pre-decode limits in B02.
  const url = URL.createObjectURL(file);
  try {
    const image = new Image();
    image.src = url;
    await image.decode();
    if (image.naturalWidth * image.naturalHeight > 30_000_000)
      throw new Error("Gambar melebihi 30 MP / Image exceeds 30 MP");
    return {
      id: crypto.randomUUID(),
      name: file.name,
      width: image.naturalWidth,
      height: image.naturalHeight,
      bytes: file.size,
      previewUrl: url,
      fingerprint: `${file.name}:${file.size}:${file.lastModified}`,
    };
  } catch (e) {
    URL.revokeObjectURL(url);
    throw e instanceof Error ? e : new Error("Gambar rusak / Corrupt image");
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
      throw new Error(
        "Gambar animasi tidak didukung / Animated images are unsupported",
      );
    const length = view.getUint32(png ? 0 : 4, !png);
    offset += png ? length + 12 : 8 + length + (length % 2);
    if (png && tag === "IEND") return;
  }
}
