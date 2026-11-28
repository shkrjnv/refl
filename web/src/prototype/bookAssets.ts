import type { Book } from "../generated/core/Book";

/**
 * Compiled book assets. This module is imported only through
 * `loadBuiltInBooks`, so Vite puts the books in their own chunk instead of
 * the main bundle.
 */
const assets = import.meta.glob("../generated/books/book.*.json", {
  eager: true,
  import: "default",
}) as Record<string, unknown>;

export function builtInBookAsset(id: string): Book | null {
  const asset = assets[`../generated/books/${id}.json`];
  return asset && typeof asset === "object" ? (asset as Book) : null;
}
