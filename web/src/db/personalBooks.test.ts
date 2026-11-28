import "fake-indexeddb/auto";

import { afterEach, describe, expect, it } from "vitest";

import type { Book } from "../generated/core/Book";
import { LearnerStorage, type BookCoverRecord, type PersonalBookRecord } from "./storage";

const opened: LearnerStorage[] = [];

async function openStorage(learnerId: string): Promise<LearnerStorage> {
  const storage = await LearnerStorage.open(learnerId);
  opened.push(storage);
  return storage;
}

function book(id: string): Book {
  return {
    id,
    version: "1".repeat(64),
    title: { en: "A personal book" },
    level: "A2",
    license: "personal",
    source: "Uploaded by you",
    chapters: [],
  };
}

function personal(learnerId: string, id: string, importedAt: number): PersonalBookRecord {
  const value = book(id);
  return {
    learner_id: learnerId,
    book: value,
    imported_at: importedAt,
    bytes: JSON.stringify(value).length,
  };
}

function cover(learnerId: string, bookId: string): BookCoverRecord {
  return {
    learner_id: learnerId,
    book_id: bookId,
    data_url: "data:image/webp;base64,AAAA",
    updated_at: 10,
  };
}

afterEach(async () => {
  for (const storage of opened.splice(0)) {
    await storage.wipe().catch(() => undefined);
  }
});

describe("personal bookshelf storage", () => {
  it("stores personal books and covers per learner", async () => {
    const first = await openStorage("shelf-a");
    const second = await openStorage("shelf-b");
    await first.putPersonalBook(personal("shelf-a", "book.personal-one", 1));
    await first.putPersonalBook(personal("shelf-a", "book.personal-two", 2));
    await first.putBookCover(cover("shelf-a", "book.personal-one"));
    await second.putPersonalBook(personal("shelf-b", "book.personal-other", 1));

    const books = await first.listPersonalBooks();
    expect(books.map((entry) => entry.book.id)).toEqual([
      "book.personal-two",
      "book.personal-one",
    ]);
    expect(await first.listBookCovers()).toHaveLength(1);
    expect((await second.listPersonalBooks()).map((entry) => entry.book.id)).toEqual([
      "book.personal-other",
    ]);
    expect(await second.listBookCovers()).toHaveLength(0);
  });

  it("rejects a book outside the personal namespace and an empty cover", async () => {
    const storage = await openStorage("shelf-guard");
    await expect(
      storage.putPersonalBook(personal("shelf-guard", "book.compiled", 1)),
    ).rejects.toThrow("book.personal");
    await expect(
      storage.putBookCover({
        learner_id: "shelf-guard",
        book_id: "book.personal-one",
        data_url: "https://example.com/cover.jpg",
        updated_at: 1,
      }),
    ).rejects.toThrow("data URL");
  });

  it("deletes a personal book together with its cover and exports both", async () => {
    const storage = await openStorage("shelf-export");
    await storage.putPersonalBook(personal("shelf-export", "book.personal-one", 1));
    await storage.putBookCover(cover("shelf-export", "book.personal-one"));
    const exported = JSON.parse(await storage.exportCache()) as {
      personal_books: unknown[];
      book_covers: unknown[];
    };
    expect(exported.personal_books).toHaveLength(1);
    expect(exported.book_covers).toHaveLength(1);

    await storage.deletePersonalBook("book.personal-one");
    expect(await storage.listPersonalBooks()).toHaveLength(0);
    expect(await storage.listBookCovers()).toHaveLength(0);
  });
});
