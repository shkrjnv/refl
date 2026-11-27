import type { ApiClient } from "../api/client";
import type { LearnerStorage } from "../db/storage";

/**
 * Sign out, then remove this learner's local database even when the network
 * logout request fails. Callers that still have unsynced events should sync
 * first; rejected events remain visible only until this wipe.
 */
export async function logoutAndWipe(
  api: Pick<ApiClient, "logout">,
  storage: LearnerStorage,
): Promise<void> {
  let failure: unknown = null;
  try {
    await api.logout();
  } catch (error) {
    failure = error;
  }
  await storage.wipe();
  if (failure !== null) {
    throw failure;
  }
}
