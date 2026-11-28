export type PersistenceStatus =
  | "granted"
  | "denied"
  | "unsupported"
  | "unknown";

export interface AppStatus {
  online: boolean;
  syncing: boolean;
  pendingOutbox: number;
  rejectedOutbox: number;
  clockDrift: boolean;
  persistence: PersistenceStatus;
  updateReady: boolean;
  lastSyncedAt: number | null;
}

export function initialStatus(online = true): AppStatus {
  return {
    online,
    syncing: false,
    pendingOutbox: 0,
    rejectedOutbox: 0,
    clockDrift: false,
    persistence: "unknown",
    updateReady: false,
    lastSyncedAt: null,
  };
}
