import heroNature480Avif from "../assets/hero-nature-480.avif";
import heroNature768Avif from "../assets/hero-nature-768.avif";
import heroNature1120Avif from "../assets/hero-nature-1120.avif";
import heroNature1600Avif from "../assets/hero-nature-1600.avif";
import heroNature2048Avif from "../assets/hero-nature-2048.avif";
import heroNature480Jpg from "../assets/hero-nature-480.jpg";
import heroNature768Jpg from "../assets/hero-nature-768.jpg";
import heroNature1120Jpg from "../assets/hero-nature-1120.jpg";
import heroNature1600Jpg from "../assets/hero-nature-1600.jpg";
import heroNature2048Jpg from "../assets/hero-nature-2048.jpg";
import heroNature480Webp from "../assets/hero-nature-480.webp";
import heroNature768Webp from "../assets/hero-nature-768.webp";
import heroNature1120Webp from "../assets/hero-nature-1120.webp";
import heroNature1600Webp from "../assets/hero-nature-1600.webp";
import heroNature2048Webp from "../assets/hero-nature-2048.webp";

/** The hero is full-width inside the studio column, never wider than 1180 px. */
const HERO_SIZES =
  "(max-width: 820px) calc(100vw - 32px), (min-width: 1260px) 1180px, calc(100vw - 80px)";

function srcSet(entries: ReadonlyArray<readonly [string, number]>): string {
  return entries.map(([url, width]) => `${url} ${width}w`).join(", ");
}

const AVIF_SRC_SET = srcSet([
  [heroNature480Avif, 480],
  [heroNature768Avif, 768],
  [heroNature1120Avif, 1120],
  [heroNature1600Avif, 1600],
  [heroNature2048Avif, 2048],
]);

const WEBP_SRC_SET = srcSet([
  [heroNature480Webp, 480],
  [heroNature768Webp, 768],
  [heroNature1120Webp, 1120],
  [heroNature1600Webp, 1600],
  [heroNature2048Webp, 2048],
]);

const JPEG_SRC_SET = srcSet([
  [heroNature480Jpg, 480],
  [heroNature768Jpg, 768],
  [heroNature1120Jpg, 1120],
  [heroNature1600Jpg, 1600],
  [heroNature2048Jpg, 2048],
]);

export function HeroIllustration() {
  return (
    <picture className="hero-picture">
      <source type="image/avif" srcSet={AVIF_SRC_SET} sizes={HERO_SIZES} />
      <source type="image/webp" srcSet={WEBP_SRC_SET} sizes={HERO_SIZES} />
      <img
        className="hero-illustration hero-nature"
        src={heroNature1120Jpg}
        srcSet={JPEG_SRC_SET}
        sizes={HERO_SIZES}
        alt=""
        role="presentation"
        aria-hidden="true"
        draggable={false}
        width={3000}
        height={1792}
        loading="eager"
        fetchPriority="high"
        decoding="async"
      />
    </picture>
  );
}
