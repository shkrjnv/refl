import { useState } from "react";

import type { AuthTokens } from "../api/client";
import {
  AuthScreen,
  type AuthApi,
  type PendingNotice,
} from "./AuthScreen";
import { PendingScreen, type AccessState } from "./PendingScreen";

export interface AuthGateProps {
  api: AuthApi;
  inviteCode: string | null;
  onAuthenticated: (tokens: AuthTokens, method: "email" | "telegram") => void;
}

export function AuthGate({
  api,
  inviteCode,
  onAuthenticated,
}: AuthGateProps) {
  const [access, setAccess] = useState<AccessState | null>(null);
  const [pending, setPending] = useState<PendingNotice | null>(null);

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
      onAuthenticated={onAuthenticated}
      onPending={setPending}
      onAccessBlocked={setAccess}
    />
  );
}
