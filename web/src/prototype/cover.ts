export const COVER_WIDTH = 320;
export const COVER_HEIGHT = 480;
export const MAX_COVER_SOURCE_BYTES = 12 * 1024 * 1024;

export interface PreparedCover {
  dataUrl: string;
  bytes: number;
}

function imageFromBlob(blob: Blob): Promise<HTMLImageElement> {
  return new Promise((resolve, reject) => {
    const url = URL.createObjectURL(blob);
    const image = new Image();
    image.onload = () => {
      URL.revokeObjectURL(url);
      resolve(image);
    };
    image.onerror = () => {
      URL.revokeObjectURL(url);
      reject(new Error("cover image cannot be decoded"));
    };
    image.src = url;
  });
}

function drawCover(
  source: CanvasImageSource,
  width: number,
  height: number,
): HTMLCanvasElement {
  const canvas = document.createElement("canvas");
  canvas.width = COVER_WIDTH;
  canvas.height = COVER_HEIGHT;
  const context = canvas.getContext("2d");
  if (!context) {
    throw new Error("canvas is unavailable");
  }
  context.fillStyle = "#1d2523";
  context.fillRect(0, 0, COVER_WIDTH, COVER_HEIGHT);
  const scale = Math.max(COVER_WIDTH / width, COVER_HEIGHT / height);
  const drawWidth = width * scale;
  const drawHeight = height * scale;
  context.drawImage(
    source,
    (COVER_WIDTH - drawWidth) / 2,
    (COVER_HEIGHT - drawHeight) / 2,
    drawWidth,
    drawHeight,
  );
  return canvas;
}

function encoded(canvas: HTMLCanvasElement, type: string, quality: number): string {
  return canvas.toDataURL(type, quality);
}

function dataUrlBytes(dataUrl: string): number {
  const comma = dataUrl.indexOf(",");
  return comma < 0 ? dataUrl.length : Math.ceil((dataUrl.length - comma - 1) * 0.75);
}

/**
 * Crops a cover to 2:3, scales it to 320×480, and encodes WebP (JPEG when the
 * browser has no WebP encoder). The stored string is typically well below
 * 60 KB, leaving the 50 MB device budget to the books themselves.
 */
export async function prepareCover(blob: Blob): Promise<PreparedCover> {
  if (blob.size > MAX_COVER_SOURCE_BYTES) {
    throw new Error("cover_source_too_large");
  }
  if (blob.type && !blob.type.startsWith("image/")) {
    throw new Error("cover_not_an_image");
  }
  const image = await imageFromBlob(blob);
  const width = image.naturalWidth || image.width;
  const height = image.naturalHeight || image.height;
  if (width <= 0 || height <= 0) {
    throw new Error("cover_image_empty");
  }
  const canvas = drawCover(image, width, height);
  let dataUrl = encoded(canvas, "image/webp", 0.82);
  if (!dataUrl.startsWith("data:image/webp")) {
    dataUrl = encoded(canvas, "image/jpeg", 0.84);
  }
  if (dataUrlBytes(dataUrl) > 160_000) {
    dataUrl = encoded(canvas, "image/jpeg", 0.72);
  }
  return { dataUrl, bytes: dataUrlBytes(dataUrl) };
}

/** Testable size guard shared by the shelf and the upload dialog. */
export function isCoverFile(file: File): boolean {
  return ["image/jpeg", "image/png", "image/webp"].includes(file.type);
}
