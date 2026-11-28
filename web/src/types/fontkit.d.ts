declare module "fontkit" {
  export interface Font {
    familyName: string;
    hasGlyphForCodePoint(codePoint: number): boolean;
  }

  export function openSync(path: string): Font;
}
