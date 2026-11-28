/**
 * AGPL section 13 source offer. A deployment can point this at its exact
 * release/tag without rebuilding the UI; the default page is the project's
 * canonical source entry point (docs/14).
 */
const configured = import.meta.env.VITE_SOURCE_URL;

export const SOURCE_CODE_URL =
  typeof configured === "string" && configured.trim().length > 0
    ? configured
    : "https://refl.uz/source";

export const SOURCE_LICENSE = "AGPL-3.0-only";
