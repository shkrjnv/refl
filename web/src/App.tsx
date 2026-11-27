import { useEffect, useRef, useState } from "react";

import {
  ApiClient,
  inviteCodeFromUrl,
  type AuthTokens,
} from "./api/client";
import { AppShell, type AppRoute } from "./components/AppShell";
import { Button, Card } from "./components/ui";
import {
  LearnerStorage,
  requestPersistentStorage,
  type OutboxRecord,
} from "./db/storage";
import { useTranslation } from "./i18n";
import { LessonRunner, type LessonSummary, type LessonView } from "./prototype/lesson";
import { loadPrototypeContent } from "./prototype/load";
import { isTourComplete, resetTour } from "./prototype/onboarding";
import { PrototypeContent } from "./prototype/content";
import {
  activitySummary,
  type ActivitySummary,
} from "./prototype/activity";
import {
  normalizeLayout,
  readLayout,
  resetLayout,
  storeLayout,
  type DashboardLayout,
  type NavigationPosition,
} from "./prototype/layout";
import { initialStatus, type AppStatus } from "./prototype/status";
import { noteActiveDay, readStreak } from "./prototype/streak";
import { AuthGate } from "./screens/AuthGate";
import { LessonScreen } from "./screens/LessonScreen";
import { ProfileScreen } from "./screens/ProfileScreen";
import { SettingsScreen } from "./screens/SettingsScreen";
import { ProgressScreen } from "./screens/ProgressScreen";
import { SummaryScreen } from "./screens/SummaryScreen";
import { TodayScreen } from "./screens/TodayScreen";
import { logoutAndWipe } from "./sync/session";
import { SyncEngine } from "./sync/engine";
import { ClientClock } from "./sync/uuidv7";
import {
  applySkin,
  applyTheme,
  isSkinName,
  storeSkin,
  storeTheme,
  type SkinName,
  type ThemeName,
} from "./theme/tokens";
import type { JsonObject } from "./types/json";
import { CoreWorkerClient } from "./worker/client";

interface AppProps {
  initialTheme: ThemeName;
  initialSkin: SkinName;
}

type VisibleRoute = AppRoute | "summary";

interface Identity {
  learnerId: string;
  label: string;
  method: "email" | "telegram";
}

function sessionFromTokens(tokens: AuthTokens) {
  return {
    learner_id: tokens.learner_id,
    access_token: tokens.access_token,
    refresh_token: tokens.refresh_token,
    expires_at: Date.now() + tokens.expires_in * 1000,
  };
}

export function App({ initialTheme, initialSkin }: AppProps) {
  const { locale, locales, setLocale, t } = useTranslation();
  const [theme, setTheme] = useState<ThemeName>(initialTheme);
  const [skin, setSkin] = useState<SkinName>(initialSkin);
  const [route, setRoute] = useState<VisibleRoute>("today");
  const [identity, setIdentity] = useState<Identity | null>(null);
  const [storage, setStorage] = useState<LearnerStorage | null>(null);
  const [content, setContent] = useState<PrototypeContent | null>(null);
  const [contentError, setContentError] = useState(false);
  const [stateJson, setStateJson] = useState<string | null>(null);
  const [status, setStatus] = useState<AppStatus>(() =>
    initialStatus(typeof navigator === "undefined" ? true : navigator.onLine),
  );
  const [rejected, setRejected] = useState<OutboxRecord[]>([]);
  const [isAdmin, setIsAdmin] = useState(false);
  const [tourOpen, setTourOpen] = useState(false);
  const [runner, setRunner] = useState<LessonRunner | null>(null);
  const [lessonView, setLessonView] = useState<LessonView | null>(null);
  const [summary, setSummary] = useState<LessonSummary | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);
  const [streak, setStreak] = useState(() => readStreak());
  const [layout, setLayout] = useState<DashboardLayout>(() => readLayout());
  const [activity, setActivity] = useState<ActivitySummary>(() =>
    activitySummary([], null),
  );
  const skinRef = useRef<SkinName>(skin);
  skinRef.current = skin;
  const appearanceDirtyRef = useRef(false);

  const sessionRef = useRef<ReturnType<typeof sessionFromTokens> | null>(null);
  const storageRef = useRef<LearnerStorage | null>(null);
  const contentRef = useRef<PrototypeContent | null>(null);
  const coreRef = useRef<CoreWorkerClient | null>(null);
  const clockRef = useRef(new ClientClock());
  const syncingRef = useRef(false);
  const apiRef = useRef<ApiClient | null>(null);
  if (!apiRef.current) {
    apiRef.current = new ApiClient({
      getSession: async () => sessionRef.current,
      setSession: async (session) => {
        sessionRef.current = session;
        if (storageRef.current && session) {
          await storageRef.current.setAuthSession(session);
        }
      },
    });
  }
  const api = apiRef.current;

  useEffect(() => {
    const core = new CoreWorkerClient();
    coreRef.current = core;
    return () => {
      core.terminate();
      coreRef.current = null;
    };
  }, []);

  useEffect(() => {
    const markOnline = () => setStatus((current) => ({ ...current, online: true }));
    const markOffline = () =>
      setStatus((current) => ({ ...current, online: false, syncing: false }));
    window.addEventListener("online", markOnline);
    window.addEventListener("offline", markOffline);
    return () => {
      window.removeEventListener("online", markOnline);
      window.removeEventListener("offline", markOffline);
    };
  }, []);

  useEffect(() => {
    if (!identity || !storage || !content) {
      return;
    }
    void syncNow();
    // The sync loop is intentionally re-run when the authenticated learner
    // or the active bundle changes.
  }, [identity, storage, content]);

  const refreshLocalState = async (store: LearnerStorage) => {
    const snapshot = await store.getSnapshot();
    setStateJson(snapshot?.state_json ?? null);
    const [outbox, events] = await Promise.all([
      store.listOutbox(),
      store.listEvents(),
    ]);
    setRejected(outbox.filter((event) => event.status === "rejected"));
    setActivity(
      activitySummary(
        [
          ...events.map((record) => ({
            event_id: record.event_id,
            event: record.event,
          })),
          ...outbox.map((record) => ({
            event_id: record.event_id,
            event: record.event,
          })),
        ],
        snapshot?.state_json ?? null,
      ),
    );
    setStatus((current) => ({
      ...current,
      pendingOutbox: outbox.filter((event) => event.status === "pending").length,
      rejectedOutbox: outbox.filter((event) => event.status === "rejected").length,
    }));
  };

  const syncNow = async (): Promise<boolean> => {
    const store = storageRef.current;
    const loaded = contentRef.current;
    const core = coreRef.current;
    if (!store || !loaded || !core || !identity || syncingRef.current) {
      return false;
    }
    syncingRef.current = true;
    setStatus((current) => ({ ...current, syncing: true }));
    try {
      const info = await core.coreInfo();
      const engine = new SyncEngine({
        storage: store,
        api,
        core,
        coreVersion: info.core_version,
        contentVersion: loaded.contentVersion,
        clock: clockRef.current,
        content: {
          bundleJson: loaded.bundleJson,
          lexiconJson: loaded.lexiconJson,
          contentVersion: loaded.contentVersion,
        },
      });
      const result = await engine.run();
      await refreshLocalState(store);
      const selection = await store.getThemeSelection();
      if (selection !== null && isSkinName(selection.theme)) {
        storeSkin(selection.theme);
        applySkin(selection.theme);
        setSkin(selection.theme);
      }
      setStatus((current) => ({
        ...current,
        syncing: false,
        online: true,
        clockDrift: result.clockSuspect,
        lastSyncedAt: Date.now(),
      }));
      return true;
    } catch {
      await refreshLocalState(store).catch(() => undefined);
      setStatus((current) => ({ ...current, syncing: false, online: false }));
      return false;
    } finally {
      syncingRef.current = false;
    }
  };

  const handleAuthenticated = async (
    tokens: AuthTokens,
    method: "email" | "telegram",
  ) => {
    const learnerId = tokens.learner_id;
    setActionError(null);
    sessionRef.current = sessionRef.current ?? sessionFromTokens(tokens);
    const store = await LearnerStorage.open(learnerId);
    storageRef.current = store;
    setStorage(store);
    await store.setAuthSession(sessionRef.current);
    const persistence = await requestPersistentStorage();
    setStatus((current) => ({ ...current, persistence }));
    setIdentity({
      learnerId,
      label: learnerId.slice(0, 8),
      method,
    });
    setContentError(false);
    try {
      const loaded = await loadPrototypeContent(api, store);
      contentRef.current = loaded.content;
      setContent(loaded.content);
      setStatus((current) => ({
        ...current,
        updateReady: loaded.updateReady,
      }));
      await refreshLocalState(store);
      setStreak(noteActiveDay());
      if (appearanceDirtyRef.current) {
        appearanceDirtyRef.current = false;
        enqueueThemeSelection(store, learnerId, skinRef.current);
      }
      setTourOpen(!isTourComplete());
      void api
        .pendingRequests()
        .then(() => setIsAdmin(true))
        .catch(() => setIsAdmin(false));
      void syncNow();
    } catch {
      setContentError(true);
    }
  };

  const startLesson = async () => {
    const store = storageRef.current;
    const loaded = contentRef.current;
    const core = coreRef.current;
    if (!store || !loaded || !core || !identity) {
      return;
    }
    setActionError(null);
    try {
      const lesson = new LessonRunner({
        storage: store,
        core,
        content: loaded,
        learnerId: identity.learnerId,
        locale,
        clock: clockRef.current,
      });
      const view = await lesson.start();
      setRunner(lesson);
      setLessonView(view);
      setRoute("lesson");
      setTourOpen(false);
    } catch {
      setActionError(t("lesson-start-failed"));
    }
  };

  const applyStagedUpdate = async () => {
    const store = storageRef.current;
    if (!store) {
      return;
    }
    try {
      const staged = await store.getStagedContent();
      if (!staged) {
        return;
      }
      await store.activateStagedContent();
      const active = await store.getActiveContent();
      if (active) {
        const next = new PrototypeContent(
          active.bundle_json,
          active.content_version,
          active.bundle_digest,
          active.dictionary.text,
        );
        contentRef.current = next;
        setContent(next);
      }
      setStatus((current) => ({ ...current, updateReady: false }));
    } catch {
      // The previous active pair remains usable.
    }
    if (!status.updateReady) {
      return;
    }
    try {
      const loaded = await loadPrototypeContent(api, store, true);
      contentRef.current = loaded.content;
      setContent(loaded.content);
      setStatus((current) => ({ ...current, updateReady: false }));
    } catch {
      // Keep the previous compatible bundle until the server is reachable.
    }
  };

  const handleLessonComplete = (result: LessonSummary) => {
    setStreak(noteActiveDay());
    setSummary(result);
    setRunner(null);
    setLessonView(null);
    setRoute("summary");
    void applyStagedUpdate().then(() => syncNow());
    void storageRef.current?.getSnapshot().then((snapshot) => {
      setStateJson(snapshot?.state_json ?? null);
    });
  };

  const handleLogout = async () => {
    const store = storageRef.current;
    if (!store) {
      return;
    }
    setStatus((current) => ({ ...current, syncing: true }));
    const flushed = await syncNow();
    if (!flushed) {
      const pending = await store.listPendingOutbox().catch(() => []);
      if (pending.length > 0) {
        setStatus((current) => ({ ...current, syncing: false, online: false }));
        setActionError(t("profile-logout-blocked"));
        return;
      }
    }
    try {
      await logoutAndWipe(api, store);
    } catch {
      // Local data is still wiped when the network logout fails.
    }
    storageRef.current = null;
    contentRef.current = null;
    sessionRef.current = null;
    setStorage(null);
    setContent(null);
    setIdentity(null);
    setSummary(null);
    setRunner(null);
    setLessonView(null);
    setRejected([]);
    setIsAdmin(false);
    setRoute("today");
    setStatus(initialStatus(true));
  };

  const changeTheme = (next: ThemeName) => {
    setTheme(next);
    storeTheme(next);
    applyTheme(next);
  };

  const changeLayout = (next: DashboardLayout) => {
    const normalized = normalizeLayout(next);
    setLayout(normalized);
    storeLayout(normalized);
  };

  const changeNavigation = (next: NavigationPosition) => {
    changeLayout({ ...layout, nav: next });
  };

  const resetDashboardLayout = () => {
    setLayout(resetLayout());
  };

  const changeAuthTheme = (next: ThemeName) => {
    appearanceDirtyRef.current = true;
    changeTheme(next);
  };

  const enqueueThemeSelection = (
    store: LearnerStorage,
    learnerId: string,
    next: SkinName,
  ) => {
    const stamp = clockRef.current.stamp();
    const payload: JsonObject = { type: "theme_selected", theme: next };
    const event: JsonObject = {
      schema_version: 2,
      event_id: stamp.event_id,
      learner_id: learnerId,
      session_id: null,
      client_ts: stamp.client_ts,
      server_ts: null,
      payload,
    };
    void store
      .rememberThemeSelection({
        theme: next,
        event_id: stamp.event_id,
      })
      .then(() =>
        store.queueOutbox({
          learner_id: learnerId,
          event_id: stamp.event_id,
          event,
          confirmed: false,
        }),
      )
      .then(() => refreshLocalState(store))
      .then(() => syncNow())
      .catch(() => undefined);
  };

  const rememberSkinOnServer = (next: SkinName) => {
    const store = storageRef.current;
    const currentIdentity = identity;
    if (!store || !currentIdentity) {
      return;
    }
    enqueueThemeSelection(store, currentIdentity.learnerId, next);
  };

  const changeSkin = (next: SkinName) => {
    if (next === skin) {
      return;
    }
    if (!identity) {
      appearanceDirtyRef.current = true;
    }
    setSkin(next);
    storeSkin(next);
    applySkin(next);
    rememberSkinOnServer(next);
  };

  const exportLocalCache = async () => {
    const store = storageRef.current;
    if (!store) {
      return;
    }
    const json = await store.exportCache();
    const blob = new Blob([json], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const link = document.createElement("a");
    link.href = url;
    link.download = `refl-local-cache-${identity?.learnerId ?? "learner"}.json`;
    link.rel = "noopener";
    document.body.append(link);
    link.click();
    link.remove();
    window.setTimeout(() => URL.revokeObjectURL(url), 0);
  };

  const resetLocalCache = async () => {
    const store = storageRef.current;
    if (!store) {
      return;
    }
    await store.wipe();
    storageRef.current = null;
    contentRef.current = null;
    sessionRef.current = null;
    setStorage(null);
    setContent(null);
    setStateJson(null);
    setContentError(false);
    setIdentity(null);
    setSummary(null);
    setRunner(null);
    setLessonView(null);
    setRejected([]);
    setIsAdmin(false);
    setRoute("today");
    setStatus(initialStatus(true));
  };

  const replayTour = () => {
    resetTour();
    setRoute("today");
    setTourOpen(true);
  };

  if (!identity) {
    return (
      <AuthGate
        api={api}
        inviteCode={inviteCodeFromUrl()}
        theme={theme}
        skin={skin}
        locale={locale}
        locales={locales}
        onLocaleChange={setLocale}
        onThemeChange={changeAuthTheme}
        onAuthenticated={(tokens, method) => {
          void handleAuthenticated(tokens, method).catch(() =>
            setContentError(true),
          );
        }}
      />
    );
  }

  if (contentError) {
    return (
      <div className="startup-screen">
        <Card tone="plain" className="startup-card">
          <h1>{t("app-content-error-title")}</h1>
          <p>{t("app-content-error-body")}</p>
          <Button type="button" variant="primary" onClick={() => window.location.reload()}>
            {t("common-retry")}
          </Button>
        </Card>
      </div>
    );
  }

  if (!content) {
    return (
      <div className="startup-screen">
        <Card tone="plain" className="startup-card">
          <h1>{t("app-content-loading")}</h1>
          <p>{t("app-content-loading-body")}</p>
        </Card>
      </div>
    );
  }

  const activeRoute: AppRoute =
    route === "summary" ? "today" : route;
  const learnerLabel = identity.label;

  return (
    <AppShell
      active={activeRoute}
      onNavigate={(next) => {
        if (next !== "lesson") {
          setRoute(next);
        }
      }}
      learnerLabel={learnerLabel}
      locale={locale}
      locales={locales}
      onLocaleChange={setLocale}
      theme={theme}
      onThemeChange={changeTheme}
      skin={skin}
      onSkinChange={changeSkin}
      navigationPosition={layout.nav}
      onIellaActivate={replayTour}
      lessonAvailable={runner !== null}
    >
      {actionError ? (
        <p className="inline-alert danger" role="alert">
          {actionError}
        </p>
      ) : null}
      {route === "today" ? (
        <TodayScreen
          content={content}
          stateJson={stateJson}
          learnerLabel={learnerLabel}
          streak={streak}
          status={status}
          activity={activity}
          layout={layout}
          onLayoutChange={changeLayout}
          onResetLayout={resetDashboardLayout}
          onStart={() => void startLesson()}
          onOpenStats={() => setRoute("stats")}
          tourOpen={tourOpen}
          onTourComplete={() => setTourOpen(false)}
        />
      ) : null}
      {route === "lesson" && runner && lessonView ? (
        <LessonScreen
          runner={runner}
          initialView={lessonView}
          onViewChange={setLessonView}
          onComplete={handleLessonComplete}
          onExit={() => {
            setRoute("today");
            setRunner(null);
            setLessonView(null);
            const store = storageRef.current;
            if (store) {
              void refreshLocalState(store);
            }
            void syncNow();
          }}
        />
      ) : null}
      {route === "summary" && summary ? (
        <SummaryScreen
          summary={summary}
          onDone={() => {
            setRoute("today");
            const store = storageRef.current;
            if (store) {
              void refreshLocalState(store);
            }
            void syncNow();
          }}
        />
      ) : null}
      {route === "stats" ? (
        <ProgressScreen content={content} stateJson={stateJson} />
      ) : null}
      {route === "profile" ? (
        <ProfileScreen
          learnerLabel={learnerLabel}
          learnerId={identity.learnerId}
          authMethod={identity.method}
          content={content}
          stateJson={stateJson}
          streak={streak}
          isAdmin={isAdmin}
          onOpenSettings={() => setRoute("settings")}
          onLogout={() => void handleLogout()}
        />
      ) : null}
      {route === "settings" ? (
        <SettingsScreen
          locale={locale}
          locales={locales}
          onLocaleChange={setLocale}
          theme={theme}
          onThemeChange={changeTheme}
          skin={skin}
          onSkinChange={changeSkin}
          navigationPosition={layout.nav}
          onNavigationPositionChange={changeNavigation}
          status={status}
          rejected={rejected}
          onExportCache={exportLocalCache}
          onResetCache={resetLocalCache}
          isAdmin={isAdmin}
          adminApi={api}
        />
      ) : null}
    </AppShell>
  );
}
