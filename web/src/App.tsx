import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import {
  ApiClient,
  ApiError,
  inviteCodeFromUrl,
  type AuthTokens,
  type ProfileResponse,
} from "./api/client";
import { AppShell, type AppRoute } from "./components/AppShell";
import {
  readCompanion,
  readSendoPreview,
  sendoUnlocked,
  storeCompanion,
  storeSendoPreview,
  type CompanionId,
} from "./companions/registry";
import { BOOKS } from "./prototype/books";
import { parseRecords, RecordStore } from "./prototype/records";
import { VocabularyScreen } from "./screens/VocabularyScreen";
import { LibraryScreen } from "./screens/LibraryScreen";
import { GraphScreen } from "./screens/GraphScreen";
import { Button, Card } from "./components/ui";
import {
  LearnerStorage,
  requestPersistentStorage,
  type OutboxRecord,
} from "./db/storage";
import { useTranslation } from "./i18n";
import { applyServiceWorkerUpdate } from "./pwa/serviceWorker";
import { LessonRunner, type LessonSummary, type LessonView } from "./prototype/lesson";
import {
  contentFromRecord,
  fetchActiveContentPair,
  loadPrototypeContent,
} from "./prototype/load";
import { isTourComplete, resetTour } from "./prototype/onboarding";
import type { PrototypeContent } from "./prototype/content";
import {
  entryDecision,
  hasLearningHistory,
  recordZeroEntry,
  resetEntryChoice,
} from "./prototype/entry";
import {
  PlacementRunner,
  type PlacementView,
} from "./prototype/placement";
import {
  activitySummary,
  type ActivitySummary,
} from "./prototype/activity";
import {
  normalizeLayout,
  readLayout,
  storeLayout,
  type DashboardLayout,
  type NavigationPosition,
} from "./prototype/layout";
import { initialStatus, type AppStatus } from "./prototype/status";
import { noteActiveDay, readStreak } from "./prototype/streak";
import { AuthGate } from "./screens/AuthGate";
import { EntryScreen } from "./screens/EntryScreen";
import { LessonScreen } from "./screens/LessonScreen";
import { PlacementScreen } from "./screens/PlacementScreen";
import { ProfileScreen } from "./screens/ProfileScreen";
import { ProgressScreen } from "./screens/ProgressScreen";
import { SummaryScreen } from "./screens/SummaryScreen";
import { NotesWorkspace } from "./screens/NotesWorkspace";
import { TodayScreen } from "./screens/TodayScreen";
import {
  clearSessionOwner,
  logoutAndWipe,
  sessionRecordFromTokens,
  storeSessionOwner,
} from "./sync/session";
import { SyncEngine } from "./sync/engine";
import { ContentUpdateManager } from "./sync/content";
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

function downloadJson(filename: string, json: string): void {
  const blob = new Blob([json], { type: "application/json" });
  const url = URL.createObjectURL(blob);
  const link = document.createElement("a");
  link.href = url;
  link.download = filename;
  link.rel = "noopener";
  document.body.append(link);
  link.click();
  link.remove();
  window.setTimeout(() => URL.revokeObjectURL(url), 0);
}

export function App({ initialTheme, initialSkin }: AppProps) {
  const { locale, locales, setLocale, t } = useTranslation();
  const [theme, setTheme] = useState<ThemeName>(initialTheme);
  const [skin, setSkin] = useState<SkinName>(initialSkin);
  const [route, setRoute] = useState<VisibleRoute>("today");
  const [notesTarget, setNotesTarget] = useState<string | null>(null);
  const [notesReturn, setNotesReturn] = useState<AppRoute>("library");
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
  const [serverProfile, setServerProfile] = useState<ProfileResponse | null>(
    null,
  );
  const [tourOpen, setTourOpen] = useState(false);
  const [entryState, setEntryState] = useState<
    "pending" | "needed" | "done"
  >("pending");
  const [entryBusy, setEntryBusy] = useState(false);
  const [entryError, setEntryError] = useState<string | null>(null);
  const [initialSyncSettled, setInitialSyncSettled] = useState(false);
  const [placementRunner, setPlacementRunner] =
    useState<PlacementRunner | null>(null);
  const [placementView, setPlacementView] =
    useState<PlacementView | null>(null);
  const [runner, setRunner] = useState<LessonRunner | null>(null);
  const [lessonView, setLessonView] = useState<LessonView | null>(null);
  const [summary, setSummary] = useState<LessonSummary | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);
  const [streak, setStreak] = useState(() => readStreak());
  const [layout, setLayout] = useState<DashboardLayout>(() => readLayout());
  const [activity, setActivity] = useState<ActivitySummary>(() =>
    activitySummary([], null),
  );
  const [companion, setCompanion] = useState<CompanionId>(() => readCompanion());
  const [sendoPreview, setSendoPreview] = useState(() => readSendoPreview());
  const [sendoCoverage, setSendoCoverage] = useState<number | null>(null);
  const [recordsWriter, setRecordsWriter] = useState<RecordStore | null>(null);
  const placementAvailable = useMemo(
    () =>
      content !== null &&
      content
        .visibleArticles(locale)
        .some((entry) => content.itemsForKc(entry.kc.id, locale).length > 0),
    [content, locale],
  );

  const sessionRef = useRef<ReturnType<typeof sessionRecordFromTokens> | null>(null);
  const storageRef = useRef<LearnerStorage | null>(null);
  const contentRef = useRef<PrototypeContent | null>(null);
  const coreRef = useRef<CoreWorkerClient | null>(null);
  const contentUpdatesRef = useRef<ContentUpdateManager | null>(null);
  const clockRef = useRef(new ClientClock());
  const syncingRef = useRef(false);
  const placementActiveRef = useRef(false);
  const apiRef = useRef<ApiClient | null>(null);
  if (!apiRef.current) {
    apiRef.current = new ApiClient({
      getSession: async () => sessionRef.current,
      setSession: async (session) => {
        sessionRef.current = session;
        if (storageRef.current) {
          await storageRef.current
            .setAuthSession(session)
            .catch(() => undefined);
        }
        if (session === null) {
          clearSessionOwner();
          coreRef.current?.updates.endSession();
          contentUpdatesRef.current = null;
          contentRef.current = null;
          setIdentity(null);
          setServerProfile(null);
          setStorage(null);
          setContent(null);
          setStateJson(null);
          setRunner(null);
          setLessonView(null);
          setSummary(null);
          setRoute("today");
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
    const core = coreRef.current;
    if (!identity || !storage || !content || !core) {
      setRecordsWriter(null);
      return;
    }
    setRecordsWriter(
      new RecordStore({
        storage,
        core,
        content,
        learnerId: identity.learnerId,
        clock: clockRef.current,
      }),
    );
  }, [identity, storage, content]);

  useEffect(() => {
    const core = coreRef.current;
    if (!stateJson || !core || BOOKS.length === 0) {
      setSendoCoverage(null);
      return;
    }
    let cancelled = false;
    const book = BOOKS[0];
    if (!book) {
      return;
    }
    void core
      .bookCoverage(book.json, stateJson, Date.now())
      .then((result) => {
        if (!cancelled) {
          setSendoCoverage(
            result.coverage.total_tokens === 0
              ? null
              : Math.round(result.coverage.percent * 100),
          );
        }
      })
      .catch(() => {
        if (!cancelled) {
          setSendoCoverage(null);
        }
      });
    return () => {
      cancelled = true;
    };
  }, [stateJson]);

  const bookCoverage = useCallback(
    (bookJson: string, snapshotJson: string, nowMs: number) => {
      const core = coreRef.current;
      if (!core) {
        return Promise.reject(new Error("the local core is unavailable"));
      }
      return core.bookCoverage(bookJson, snapshotJson, nowMs);
    },
    [],
  );

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

  useEffect(() => {
    if (
      entryState !== "pending" ||
      !initialSyncSettled ||
      !identity ||
      !storage ||
      !content
    ) {
      return;
    }
    let cancelled = false;
    void (async () => {
      const snapshot = await storage.getSnapshot();
      const [events, outbox] = await Promise.all([
        storage.listEvents(),
        storage.listOutbox(),
      ]);
      const decision = entryDecision(
        snapshot?.state_json ?? null,
        identity.learnerId,
        hasLearningHistory([...events, ...outbox]),
      );
      if (!cancelled) {
        setEntryState(decision);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [entryState, initialSyncSettled, identity, storage, content]);

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

  const handleStateChange = (next: string) => {
    setStateJson(next);
    const store = storageRef.current;
    if (store) {
      void refreshLocalState(store);
    }
  };

  const sendoIsUnlocked = sendoUnlocked(sendoCoverage);

  const changeCompanion = async (next: CompanionId) => {
    if (next === "sendo" && !sendoIsUnlocked) {
      return;
    }
    setCompanion(next);
    storeCompanion(next);
    const writer = recordsWriter;
    if (writer && stateJson) {
      try {
        handleStateChange(await writer.selectCompanion(stateJson, next));
      } catch {
        // The local choice still applies; the event retries with the next write.
      }
    }
  };

  const toggleSendoPreview = () => {
    const next = !sendoPreview;
    setSendoPreview(next);
    storeSendoPreview(next);
  };

  const practiceKc = () => {
    void startLesson(15);
  };

  const openNotes = (noteId?: string, from: AppRoute = "library") => {
    setNotesTarget(noteId ?? null);
    setNotesReturn(from);
    setRoute("notes");
  };

  const syncNow = async (): Promise<boolean> => {
    const store = storageRef.current;
    const loaded = contentRef.current;
    const core = coreRef.current;
    if (
      !store ||
      !loaded ||
      !core ||
      !identity ||
      syncingRef.current ||
      placementActiveRef.current
    ) {
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
      setInitialSyncSettled(true);
    }
  };

  const handleAuthenticated = async (
    tokens: AuthTokens,
    method: "email" | "telegram",
  ) => {
    const learnerId = tokens.learner_id;
    setActionError(null);
    setEntryState("pending");
    setInitialSyncSettled(false);
    setEntryBusy(false);
    setEntryError(null);
    setPlacementRunner(null);
    setPlacementView(null);
    placementActiveRef.current = false;
    sessionRef.current = sessionRef.current ?? sessionRecordFromTokens(tokens);
    const store = await LearnerStorage.open(learnerId);
    storageRef.current = store;
    setStorage(store);
    await store.setAuthSession(sessionRef.current);
    storeSessionOwner({ learnerId, method });
    setIdentity({
      learnerId,
      label: learnerId.slice(0, 8),
      method,
    });
    setContentError(false);
    // Persistence is a best-effort capability check; it must not delay entry.
    void requestPersistentStorage()
      .then((persistence) =>
        setStatus((current) => ({ ...current, persistence })),
      )
      .catch(() => undefined);
    try {
      const core = coreRef.current;
      if (!core) {
        throw new Error("the local core is unavailable");
      }
      const manager = new ContentUpdateManager(store, core.updates);
      contentUpdatesRef.current = manager;
      const loaded = await loadPrototypeContent(
        api,
        store,
        manager,
        async (bundleJson) =>
          (await core.checkBundleCompatibility(bundleJson)).compatible,
      );
      contentRef.current = loaded.content;
      setContent(loaded.content);
      setStatus((current) => ({
        ...current,
        updateReady: loaded.updateReady,
      }));
      await refreshLocalState(store);
      setStreak(noteActiveDay());
      setTourOpen(!isTourComplete());
      void api
        .profile()
        .then(setServerProfile)
        .catch(() => setServerProfile(null));
      void api
        .pendingRequests()
        .then(() => setIsAdmin(true))
        .catch(() => setIsAdmin(false));
      void syncNow();
    } catch (error) {
      if (
        error instanceof ApiError &&
        (error.status === 401 || error.status === 403)
      ) {
        await store.setAuthSession(null).catch(() => undefined);
        clearSessionOwner();
        sessionRef.current = null;
        storageRef.current = null;
        contentUpdatesRef.current = null;
        coreRef.current?.updates.endSession();
        setStorage(null);
        setContent(null);
        setServerProfile(null);
        setIdentity(null);
        setContentError(false);
        setEntryState("pending");
        setInitialSyncSettled(false);
        setPlacementRunner(null);
        setPlacementView(null);
        placementActiveRef.current = false;
        return;
      }
      setContentError(true);
    }
  };

  const startFromZero = async () => {
    const store = storageRef.current;
    const loaded = contentRef.current;
    const core = coreRef.current;
    if (!store || !loaded || !core || !identity) {
      return;
    }
    setEntryBusy(true);
    setEntryError(null);
    try {
      const snapshot = await store.getSnapshot();
      const current = snapshot?.state_json ?? stateJson;
      const next = await recordZeroEntry(
        {
          storage: store,
          core,
          content: loaded,
          learnerId: identity.learnerId,
          clock: clockRef.current,
        },
        current,
      );
      setStateJson(next);
      setEntryState("done");
      setTourOpen(!isTourComplete());
      void syncNow();
    } catch (error) {
      setEntryError(
        error instanceof Error ? error.message : t("common-unknown-error"),
      );
    } finally {
      setEntryBusy(false);
    }
  };

  const startPlacement = async () => {
    const store = storageRef.current;
    const loaded = contentRef.current;
    const core = coreRef.current;
    if (!store || !loaded || !core || !identity || !placementAvailable) {
      return;
    }
    setEntryBusy(true);
    setEntryError(null);
    try {
      // Wait for a settled log before the runner starts folding locally.
      await syncNow().catch(() => false);
      placementActiveRef.current = true;
      const runner = new PlacementRunner({
        storage: store,
        core,
        content: loaded,
        learnerId: identity.learnerId,
        locale,
        clock: clockRef.current,
        gate: core.updates,
      });
      const snapshot = await store.getSnapshot();
      const view = await runner.start(
        snapshot?.state_json ?? stateJson,
      );
      setPlacementRunner(runner);
      setPlacementView(view);
    } catch (error) {
      placementActiveRef.current = false;
      core.updates.endSession();
      setEntryError(
        error instanceof Error ? error.message : t("common-unknown-error"),
      );
    } finally {
      setEntryBusy(false);
    }
  };

  const finishPlacement = () => {
    placementRunner?.finish();
    coreRef.current?.updates.endSession();
    placementActiveRef.current = false;
    setPlacementRunner(null);
    setPlacementView(null);
    setEntryState("done");
    setTourOpen(!isTourComplete());
    const store = storageRef.current;
    if (store) {
      void refreshLocalState(store);
    }
    void refreshContentUpdate()
      .then(() => syncNow())
      .then(() => applyServiceWorkerUpdate());
  };

  const exitPlacement = () => {
    placementRunner?.finish();
    coreRef.current?.updates.endSession();
    placementActiveRef.current = false;
    setPlacementRunner(null);
    setPlacementView(null);
    void refreshContentUpdate().then(() => applyServiceWorkerUpdate());
  };

  const startLesson = async (minutes = 25) => {
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
        budgetMs: Math.max(1, Math.round(minutes)) * 60_000,
        gate: core.updates,
      });
      const view = await lesson.start();
      setRunner(lesson);
      setLessonView(view);
      setRoute("lesson");
      setTourOpen(false);
    } catch {
      core.updates.endSession();
      setActionError(t("lesson-start-failed"));
    }
  };

  const applyStagedUpdate = async (): Promise<boolean> => {
    const store = storageRef.current;
    const manager = contentUpdatesRef.current;
    const core = coreRef.current;
    if (!store || !manager || !core) {
      return false;
    }
    try {
      if (!(await manager.applyBetweenSessions())) {
        return false;
      }
      const active = await store.getActiveContent();
      if (active) {
        const next = contentFromRecord(active);
        contentRef.current = next;
        setContent(next);
      }
      setStatus((current) => ({ ...current, updateReady: false }));
      return true;
    } catch {
      // The previous active pair remains usable.
      return false;
    }
  };

  const refreshContentUpdate = async (): Promise<boolean> => {
    const store = storageRef.current;
    const manager = contentUpdatesRef.current;
    const core = coreRef.current;
    if (!store || !manager || !core) {
      return false;
    }
    try {
      const active = await store.getActiveContent();
      const response = await api.activeContent();
      if (active?.content_version === response.content_version) {
        return await applyStagedUpdate();
      }
      const pair = await fetchActiveContentPair(api, response);
      await manager.stage(
        pair.active,
        pair.dictionary,
        async (bundleJson) =>
          (await core.checkBundleCompatibility(bundleJson)).compatible,
      );
      setStatus((current) => ({ ...current, updateReady: true }));
      return await applyStagedUpdate();
    } catch {
      // Stay on the previous pair; retry after the next session or sync.
      return false;
    }
  };

  useEffect(() => {
    if (!initialSyncSettled || !identity || !storage || !content) {
      return;
    }
    void refreshContentUpdate();
    // Check once per settled session; the version comparison avoids a download
    // when the active pair is already current.
  }, [initialSyncSettled, identity, storage, content]);

  const handleLessonComplete = (result: LessonSummary) => {
    runner?.finish();
    coreRef.current?.updates.endSession();
    setStreak(noteActiveDay());
    setSummary(result);
    setRunner(null);
    setLessonView(null);
    setRoute("summary");
    void refreshContentUpdate().then(() => syncNow());
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
    contentUpdatesRef.current = null;
    contentRef.current = null;
    sessionRef.current = null;
    coreRef.current?.updates.endSession();
    clearSessionOwner();
    setStorage(null);
    setContent(null);
    setServerProfile(null);
    setIdentity(null);
    setSummary(null);
    setRunner(null);
    setLessonView(null);
    setRejected([]);
    setIsAdmin(false);
    setEntryState("pending");
    setInitialSyncSettled(false);
    setPlacementRunner(null);
    setPlacementView(null);
    placementActiveRef.current = false;
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
    changeLayout({ nav: next });
  };

  const exportLocalCache = async () => {
    const store = storageRef.current;
    if (!store) {
      return;
    }
    downloadJson(
      `refl-local-cache-${identity?.learnerId ?? "learner"}.json`,
      await store.exportCache(),
    );
  };

  const exportAccountData = async () => {
    downloadJson(
      `refl-export-${identity?.learnerId ?? "learner"}.json`,
      JSON.stringify(await api.exportAccount(), null, 2),
    );
  };

  const resetLocalCache = async () => {
    const store = storageRef.current;
    if (!store) {
      return;
    }
    await store.wipe();
    resetEntryChoice(identity?.learnerId);
    storageRef.current = null;
    contentUpdatesRef.current = null;
    contentRef.current = null;
    sessionRef.current = null;
    coreRef.current?.updates.endSession();
    setStorage(null);
    setContent(null);
    setServerProfile(null);
    setStateJson(null);
    setContentError(false);
    setIdentity(null);
    setSummary(null);
    setRunner(null);
    setLessonView(null);
    setRejected([]);
    setIsAdmin(false);
    setEntryState("pending");
    setInitialSyncSettled(false);
    setPlacementRunner(null);
    setPlacementView(null);
    placementActiveRef.current = false;
    setRoute("today");
    setStatus(initialStatus(true));
  };

  const eraseAccount = async () => {
    await api.eraseAccount();
    await resetLocalCache();
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
        onThemeChange={changeTheme}
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

  if (entryState === "pending") {
    return (
      <div className="startup-screen">
        <Card tone="plain" className="startup-card">
          <h1>{t("app-content-loading")}</h1>
          <p>{t("app-content-loading-body")}</p>
        </Card>
      </div>
    );
  }

  if (entryState === "needed") {
    if (placementRunner && placementView) {
      return (
        <PlacementScreen
          runner={placementRunner}
          initialView={placementView}
          onComplete={finishPlacement}
          onExit={exitPlacement}
        />
      );
    }
    return (
      <EntryScreen
        placementAvailable={placementAvailable}
        busy={entryBusy}
        error={entryError}
        onStartFromZero={() => void startFromZero()}
        onCheck={() => void startPlacement()}
      />
    );
  }

  const activeRoute: AppRoute =
    route === "summary"
      ? "today"
      : route === "notes"
        ? "library"
        : route;
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
      navigationPosition={layout.nav}
      onIellaActivate={replayTour}
      lessonAvailable={runner !== null}
      companion={companion}
    >
      {actionError ? (
        <p className="inline-alert danger" role="alert">
          {actionError}
        </p>
      ) : null}
      {route === "vocabulary" ? (
        recordsWriter && stateJson ? (
          <VocabularyScreen
            content={content}
            stateJson={stateJson}
            locale={locale}
            recordsWriter={recordsWriter}
            onStateChange={handleStateChange}
            onOpenNotes={(noteId) => openNotes(noteId, "vocabulary")}
          />
        ) : null
      ) : null}
      {route === "library" ? (
        recordsWriter && stateJson ? (
          <LibraryScreen
            content={content}
            stateJson={stateJson}
            locale={locale}
            recordsWriter={recordsWriter}
            onStateChange={handleStateChange}
            bookCoverage={bookCoverage}
            onPractice={practiceKc}
            storage={storage}
            onOpenNotes={(noteId) => openNotes(noteId, "library")}
          />
        ) : null
      ) : null}
      {route === "notes" ? (
        recordsWriter && stateJson ? (
          <NotesWorkspace
            records={parseRecords(stateJson)}
            content={content}
            stateJson={stateJson}
            locale={locale}
            recordsWriter={recordsWriter}
            onStateChange={handleStateChange}
            books={BOOKS}
            initialNoteId={notesTarget}
            onBack={() => setRoute(notesReturn)}
            onPractice={practiceKc}
          />
        ) : null
      ) : null}
      {route === "graph" ? (
        recordsWriter && stateJson ? (
          <GraphScreen
            content={content}
            stateJson={stateJson}
            locale={locale}
            recordsWriter={recordsWriter}
            onStateChange={handleStateChange}
            onPractice={practiceKc}
            onOpenNotes={(noteId) => openNotes(noteId, "graph")}
          />
        ) : null
      ) : null}
      {route === "today" ? (
        <TodayScreen
          content={content}
          stateJson={stateJson}
          learnerLabel={learnerLabel}
          streak={streak}
          status={status}
          activity={activity}
          onStart={(minutes) => void startLesson(minutes)}
          onOpenStats={() => setRoute("stats")}
          tourOpen={tourOpen}
          onTourComplete={() => setTourOpen(false)}
          companion={companion}
          recordsWriter={recordsWriter}
          onStateChange={handleStateChange}
          onOpenNotes={() => openNotes(undefined, "today")}
        />
      ) : null}
      {route === "lesson" && runner && lessonView ? (
        <LessonScreen
          runner={runner}
          initialView={lessonView}
          onViewChange={setLessonView}
          onComplete={handleLessonComplete}
          recordsWriter={recordsWriter}
          stateJson={stateJson ?? undefined}
          onStateChange={handleStateChange}
          onOpenNotes={(noteId) => openNotes(noteId, "lesson")}
          onExit={() => {
            runner?.finish();
            coreRef.current?.updates.endSession();
            setRoute("today");
            setRunner(null);
            setLessonView(null);
            const store = storageRef.current;
            if (store) {
              void refreshLocalState(store);
            }
            void refreshContentUpdate()
              .then(() => syncNow())
              .then(() => applyServiceWorkerUpdate());
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
            void syncNow().then(() => applyServiceWorkerUpdate());
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
          locale={locale}
          locales={locales}
          onLocaleChange={setLocale}
          theme={theme}
          onThemeChange={changeTheme}
          navigationPosition={layout.nav}
          onNavigationPositionChange={changeNavigation}
          status={status}
          rejected={rejected}
          minor={serverProfile?.minor ?? false}
          serverProfile={serverProfile}
          onExportCache={exportLocalCache}
          onExportAccount={exportAccountData}
          onEraseAccount={eraseAccount}
          onResetCache={resetLocalCache}
          adminApi={api}
          onLogout={() => void handleLogout()}
          onSyncNow={() => syncNow()}
          companion={companion}
          sendoUnlocked={sendoIsUnlocked}
          sendoPreview={sendoPreview}
          onCompanionChange={(next) => void changeCompanion(next)}
          onSendoPreview={toggleSendoPreview}
        />
      ) : null}
    </AppShell>
  );
}
