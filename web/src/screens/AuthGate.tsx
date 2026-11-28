import { useCallback, useEffect, useRef, useState } from "react";

import type { AuthTokens } from "../api/client";
import { LearnerStorage } from "../db/storage";
import { useTranslation, type Locale } from "../i18n";
import {
  readSessionOwner,
  restorePersistedSession,
  sessionRecordFromTokens,
  storeSessionOwner,
} from "../sync/session";
import type { SkinName, ThemeName } from "../theme/tokens";
import {
  AuthScreen,
  type AuthApi,
  type PendingNotice,
} from "./AuthScreen";
import { PendingScreen, type AccessState } from "./PendingScreen";

export interface AuthGateProps {
  api: AuthApi;
  inviteCode: string | null;
  theme: ThemeName;
  skin: SkinName;
  locale: Locale;
  locales: readonly Locale[];
  onLocaleChange: (locale: Locale) => void;
  onThemeChange: (theme: ThemeName) => void;
  onAuthenticated: (tokens: AuthTokens, method: "email" | "telegram") => void;
}

export function AuthGate({
  api,
  inviteCode,
  theme,
  skin,
  locale,
  locales,
  onLocaleChange,
  onThemeChange,
  onAuthenticated,
}: AuthGateProps) {
  const { t } = useTranslation();
  const [access, setAccess] = useState<AccessState | null>(null);
  const [pending, setPending] = useState<PendingNotice | null>(null);
  const [restoring, setRestoring] = useState(
    () => readSessionOwner() !== null,
  );
  const authenticatedRef = useRef(onAuthenticated);
  authenticatedRef.current = onAuthenticated;

  const persistAndAuthenticate = useCallback(
    (tokens: AuthTokens, method: "email" | "telegram") => {
      void (async () => {
        try {
          const storage = await LearnerStorage.open(tokens.learner_id);
          try {
            await storage.setAuthSession(sessionRecordFromTokens(tokens));
            storeSessionOwner({ learnerId: tokens.learner_id, method });
          } finally {
            storage.close();
          }
        } catch {
          // A storage failure must not block this sign-in attempt.
        }
        authenticatedRef.current(tokens, method);
      })();
    },
    [],
  );

  useEffect(() => {
    if (!restoring) {
      return;
    }
    let cancelled = false;
    void restorePersistedSession()
      .then((restored) => {
        if (cancelled) {
          return;
        }
        if (restored) {
          authenticatedRef.current(restored.tokens, restored.method);
          return;
        }
        setRestoring(false);
      })
      .catch(() => {
        if (!cancelled) {
          setRestoring(false);
        }
      });
    return () => {
      cancelled = true;
    };
  }, [restoring]);

  if (restoring) {
    return (
      <div className="startup-screen" role="status">
        <p>{t("auth-refreshing")}</p>
      </div>
    );
  }

  if (access) {
    return <PendingScreen state={access} onCheck={() => setAccess(null)} />;
  }
  if (pending) {
    return (
      <PendingScreen
        state="pending_approval"
        onCheck={() => setPending(null)}
      />
    );
  }
  return (
    <AuthScreen
      api={api}
      inviteCode={inviteCode}
      theme={theme}
      skin={skin}
      locale={locale}
      locales={locales}
      onLocaleChange={onLocaleChange}
      onThemeChange={onThemeChange}
      onAuthenticated={persistAndAuthenticate}
      onPending={setPending}
      onAccessBlocked={setAccess}
    />
  );
}
