import heroNature from "../assets/hero-nature.jpg";

export function HeroIllustration() {
  return (
    <img
      className="hero-illustration hero-nature"
      src={heroNature}
      alt=""
      role="presentation"
      aria-hidden="true"
      draggable={false}
    />
  );
}
