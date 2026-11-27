import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import { App } from "./App";
import { I18nProvider } from "./i18n";
import { applyAppearance, initialSkin, initialTheme } from "./theme/tokens";
import "./theme/tokens.css";
import "./app.css";

const theme = initialTheme();
const skin = initialSkin();
applyAppearance(theme, skin);

const rootElement = document.getElementById("root");
if (!rootElement) {
  throw new Error("root element is missing");
}

createRoot(rootElement).render(
  <StrictMode>
    <I18nProvider>
      <App initialTheme={theme} initialSkin={skin} />
    </I18nProvider>
  </StrictMode>,
);

if (import.meta.env.PROD && "serviceWorker" in navigator) {
  window.addEventListener("load", () => {
    navigator.serviceWorker.register("/sw.js").catch(() => undefined);
  });
}
