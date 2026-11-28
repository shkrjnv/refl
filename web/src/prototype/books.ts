import { useEffect, useState } from "react";

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

const catalog = catalogSource as BookCatalog;

/** Catalog entries are small and ship with the app; the texts load lazily. */
export const BOOK_CATALOG: readonly BookMeta[] = catalog.books;

let loaded: LoadedBook[] | null = null;
let loading: Promise<LoadedBook[]> | null = null;

/**
 * Books are compiled by `book-build` into committed, versioned assets in a
 * separate chunk. The app starts this load right after the first render, and
 * the service worker caches the chunk on that first online visit, so the
 * reader still works on the first offline launch. Book marks carry
 * `version` and survive a later asset update.
 */
export function loadBuiltInBooks(): Promise<LoadedBook[]> {
  if (loaded) {
    return Promise.resolve(loaded);
  }
  loading ??= import("./bookAssets").then(({ builtInBookAsset }) => {
    loaded = BOOK_CATALOG.flatMap((meta) => {
      const book = builtInBookAsset(meta.id);
      if (!book) {
        return [];
      }
      return [
        {
          meta,
          book,
          json: JSON.stringify(book),
          coverUrl: BUILT_IN_COVERS[meta.id] ?? null,
        },
      ];
    });
    return loaded;
  });
  return loading;
}

/** The built-in books, or an empty list until their chunk has loaded. */
export function useBuiltInBooks(): LoadedBook[] {
  const [books, setBooks] = useState<LoadedBook[]>(() => loaded ?? []);
  useEffect(() => {
    if (loaded) {
      setBooks(loaded);
      return;
    }
    let cancelled = false;
    void loadBuiltInBooks().then((result) => {
      if (!cancelled) {
        setBooks(result);
      }
    });
    return () => {
      cancelled = true;
    };
  }, []);
  return books;
}
