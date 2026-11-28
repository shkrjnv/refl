import catalogSource from "../generated/books/index.json";
import type { Book } from "../generated/core/Book";
import type { Localized } from "../generated/core/Localized";

export interface BookMeta {
  id: string;
  version: string;
  bytes: number;
  digest: string;
  level: string;
  title: Localized;
}

interface BookCatalog {
  books: BookMeta[];
}

export interface LoadedBook {
  meta: BookMeta;
  book: Book;
  /** Canonical asset JSON handed to `core-wasm::book_coverage`. */
  json: string;
  /** Bundled typographic cover; a personal upload overrides it. */
  coverUrl: string | null;
}

const BUILT_IN_COVERS: Record<string, string> = {
  "book.aesop-lion-mouse": "/covers/aesop-lion-mouse.svg",
  "book.aesop-north-wind-sun": "/covers/aesop-north-wind-sun.svg",
  "book.ohenry-gift-magi": "/covers/ohenry-gift-magi.svg",
};

const assets = import.meta.glob("../generated/books/*.json", {
  eager: true,
  import: "default",
}) as Record<string, unknown>;

const catalog = catalogSource as BookCatalog;

/**
 * Books are compiled by `book-build` into committed, versioned assets. They
 * are part of the JS bundle, so a reader works on the first offline launch;
 * book marks still carry `version` and survive a later asset update.
 */
export const BOOKS: LoadedBook[] = catalog.books.flatMap((meta) => {
  const asset = assets[`../generated/books/${meta.id}.json`];
  if (!asset || typeof asset !== "object") {
    return [];
  }
  return [
    {
      meta,
      book: asset as Book,
      json: JSON.stringify(asset),
      coverUrl: BUILT_IN_COVERS[meta.id] ?? null,
    },
  ];
});

export function bookById(id: string): LoadedBook | null {
  return BOOKS.find((book) => book.meta.id === id) ?? null;
}
