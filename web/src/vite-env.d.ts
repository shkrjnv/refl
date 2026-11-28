/// <reference types="vite/client" />

interface ImportMetaEnv {
  readonly VITE_API_TARGET?: string;
  readonly VITE_TELEGRAM_BOT_ID?: string;
  readonly VITE_TELEGRAM_BOT_USERNAME?: string;
  readonly VITE_SENDO_UNLOCKED?: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
