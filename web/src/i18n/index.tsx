import { FluentBundle, FluentResource } from "@fluent/bundle";
import { negotiateLanguages } from "@fluent/langneg";
import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
  type PropsWithChildren,
} from "react";

import enSource from "./en.ftl?raw";
import ruSource from "./ru.ftl?raw";
import uzSource from "./uz.ftl?raw";

export const locales = ["en", "ru", "uz"] as const;
export type Locale = (typeof locales)[number];
export type TranslationArgs = Record<string, string | number>;
export type Translate = (id: string, args?: TranslationArgs) => string;

const resources: Record<Locale, string> = {
  en: enSource,
  ru: ruSource,
  uz: uzSource,
};

const LOCALE_KEY = "refl.locale";

export function isLocale(value: string | null): value is Locale {
  return value !== null && locales.includes(value as Locale);
}

export function createBundle(locale: Locale, source: string): FluentBundle {
  const bundle = new FluentBundle(locale, { useIsolating: false });
  const errors = bundle.addResource(new FluentResource(source));
  if (errors.length > 0) {
    throw new Error(
      `Fluent resource ${locale} has errors: ${errors
        .map((error) => error.message)
        .join("; ")}`,
    );
  }
  return bundle;
}

export function createBundles(): Record<Locale, FluentBundle> {
  return {
    en: createBundle("en", resources.en),
    ru: createBundle("ru", resources.ru),
    uz: createBundle("uz", resources.uz),
  };
}

export function readStoredLocale(): Locale | null {
  try {
    const value = window.localStorage.getItem(LOCALE_KEY);
    return isLocale(value) ? value : null;
  } catch {
    return null;
  }
}

export function storeLocale(locale: Locale): void {
  try {
    window.localStorage.setItem(LOCALE_KEY, locale);
  } catch {
    // Storage can be unavailable in private or blocked contexts.
  }
}

export function browserLocales(): string[] {
  if (typeof navigator === "undefined") {
    return ["en"];
  }
  if (navigator.languages.length > 0) {
    return [...navigator.languages];
  }
  return navigator.language ? [navigator.language] : ["en"];
}

export function resolveLocale(
  requested: readonly string[],
  stored: string | null = null,
): Locale {
  if (isLocale(stored)) {
    return stored;
  }
  for (const candidate of requested) {
    if (isLocale(candidate)) {
      return candidate;
    }
  }
  const negotiated = negotiateLanguages(requested, [...locales], {
    defaultLocale: "en",
  });
  const first = negotiated[0];
  if (first !== undefined && isLocale(first)) {
    return first;
  }
  return "en";
}

export function formatMessage(
  bundle: FluentBundle,
  fallback: FluentBundle,
  id: string,
  args: TranslationArgs = {},
): string {
  const message = bundle.getMessage(id) ?? fallback.getMessage(id);
  if (!message?.value) {
    return id;
  }
  const owner = bundle.getMessage(id) ? bundle : fallback;
  const errors: Error[] = [];
  const text = owner.formatPattern(message.value, args, errors);
  if (errors.length > 0) {
    return id;
  }
  return text;
}

export interface I18nContextValue {
  locale: Locale;
  locales: readonly Locale[];
  setLocale: (locale: Locale) => void;
  t: Translate;
}

const I18nContext = createContext<I18nContextValue | null>(null);

interface I18nProviderProps {
  initialLocale?: Locale;
}

export function I18nProvider({
  children,
  initialLocale,
}: PropsWithChildren<I18nProviderProps>) {
  const bundles = useMemo(createBundles, []);
  const [locale, setLocaleState] = useState<Locale>(() => {
    if (initialLocale) {
      return initialLocale;
    }
    return resolveLocale(browserLocales(), readStoredLocale());
  });

  const setLocale = useCallback((next: Locale) => {
    setLocaleState(next);
    storeLocale(next);
  }, []);

  useEffect(() => {
    document.documentElement.lang = locale;
    document.documentElement.dir = "ltr";
  }, [locale]);

  const t = useCallback<Translate>(
    (id, args) => formatMessage(bundles[locale], bundles.en, id, args),
    [bundles, locale],
  );

  const value = useMemo<I18nContextValue>(
    () => ({ locale, locales, setLocale, t }),
    [locale, setLocale, t],
  );

  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>;
}

export function useTranslation(): I18nContextValue {
  const value = useContext(I18nContext);
  if (!value) {
    throw new Error("useTranslation must be used inside I18nProvider");
  }
  return value;
}
