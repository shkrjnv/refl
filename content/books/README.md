# Built-in books

The app currently ships no built-in books; the Library shows the learner's own uploads.

To add a graded reader, put it in `content/books/<slug>/` (`source.md` or `book.ncl`). Add its license record to `THIRD_PARTY.md` and `content/LICENSE`, then run `just books`. `book-build` skips files in this folder and reads only subfolders.

Ids already in `content/book-versions.lock` stay reserved: the lock is append-only, so book marks never point at a reused id. The former starter readers are now `book-build`'s test fixtures (`tools/book-build/tests/fixtures/books`).
