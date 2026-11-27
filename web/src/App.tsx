import { useEffect, useState } from "react";

import { useTranslation } from "./i18n";
import {
  applyTheme,
  storeTheme,
  type ThemeName,
} from "./theme/tokens";
import { CoreWorkerClient } from "./worker/client";

interface AppProps {
  initialTheme: ThemeName;
}

export function App({ initialTheme }: AppProps) {
  const { locale, locales, setLocale, t } = useTranslation();
  const [theme, setTheme] = useState<ThemeName>(initialTheme);
  const [coreVersion, setCoreVersion] = useState<string | null>(null);
  const [coreFailed, setCoreFailed] = useState(false);
  const [online, setOnline] = useState<boolean>(() =>
    typeof navigator === "undefined" ? true : navigator.onLine,
  );

  useEffect(() => {
    const core = new CoreWorkerClient();
    let active = true;
    core
      .coreInfo()
      .then((info) => {
        if (active) {
          setCoreVersion(info.core_version);
        }
      })
      .catch(() => {
        if (active) {
          setCoreFailed(true);
        }
      });
    return () => {
      active = false;
      core.terminate();
    };
  }, []);

  useEffect(() => {
    const markOnline = () => setOnline(true);
    const markOffline = () => setOnline(false);
    window.addEventListener("online", markOnline);
    window.addEventListener("offline", markOffline);
    return () => {
      window.removeEventListener("online", markOnline);
      window.removeEventListener("offline", markOffline);
    };
  }, []);

  const toggleTheme = () => {
    const next: ThemeName = theme === "light" ? "dark" : "light";
    setTheme(next);
    storeTheme(next);
    applyTheme(next);
  };

  return (
    <div className="app-shell">
      <header className="topbar">
        <div className="brand">
          <span className="brand-mark" aria-hidden="true">
            r
          </span>
          <span className="brand-name">{t("app-name")}</span>
        </div>
        <div className="controls">
          <div className="segmented" aria-label={t("app-language")}>
            {locales.map((candidate) => (
              <button
                key={candidate}
                type="button"
                className={candidate === locale ? "segment active" : "segment"}
                aria-pressed={candidate === locale}
                onClick={() => setLocale(candidate)}
              >
                {t(`language.${candidate}`)}
              </button>
            ))}
          </div>
          <button
            type="button"
            className="button ghost"
            aria-label={t("app-theme")}
            onClick={toggleTheme}
          >
            {theme === "light" ? t("common-dark") : t("common-light")}
          </button>
        </div>
      </header>

      <main className="main">
        <section className="hero">
          <div className="hero-copy">
            <p className="eyebrow">{t("nav-today")}</p>
            <h1>{t("app-tagline")}</h1>
            <p className="hero-note">{t("app-foundation")}</p>
          </div>
          <div className={online ? "status online" : "status offline"}>
            <span className="status-dot" aria-hidden="true" />
            {online ? t("sync-online") : t("sync-offline")}
          </div>
        </section>

        <section className="foundation-grid">
          <article className="card tile">
            <p className="tile-label">{t("session-title")}</p>
            <p className="tile-value">{t("app-foundation")}</p>
            <p className="tile-note">{t("app-tagline")}</p>
          </article>
          <article className="card tile">
            <p className="tile-label">{t("app-name")}</p>
            <p className="tile-value">
              {coreVersion
                ? t("app-core-version", { version: coreVersion })
                : coreFailed
                  ? t("common-unknown-error")
                  : t("app-core-starting")}
            </p>
            <p className="tile-note">{t("app-foundation")}</p>
          </article>
          <article className="card tile accent">
            <p className="tile-label">{t("app-theme")}</p>
            <p className="tile-value">
              {theme === "light" ? t("common-light") : t("common-dark")}
            </p>
            <p className="tile-note">{t("session-update-locked")}</p>
          </article>
        </section>
      </main>
    </div>
  );
}
