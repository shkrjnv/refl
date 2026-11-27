import type { ApiClient } from "../api/client";
import type { LearnerStorage } from "../db/storage";
import { PrototypeContent } from "./content";

export interface LoadedContent {
  content: PrototypeContent;
  updateReady: boolean;
  fromCache: boolean;
}

export async function loadPrototypeContent(
  api: Pick<ApiClient, "activeContent">,
  storage: LearnerStorage,
  preferServer = false,
): Promise<LoadedContent> {
  const active = preferServer ? null : await storage.getActiveContent();
  if (active) {
    return {
      content: new PrototypeContent(
        active.bundle_json,
        active.content_version,
        active.bundle_digest,
        active.dictionary.text,
      ),
      updateReady: false,
      fromCache: true,
    };
  }

  const cached = preferServer ? null : await storage.getCachedBundle();
  let response: Awaited<ReturnType<ApiClient["activeContent"]>> | null = null;
  try {
    response = await api.activeContent();
  } catch (error) {
    if (cached) {
      return {
        content: new PrototypeContent(
          cached.bundle_json,
          cached.content_version,
          cached.digest,
          cached.lexicon_text,
        ),
        updateReady: false,
        fromCache: true,
      };
    }
    throw error;
  }

  if (cached) {
    const content = new PrototypeContent(
      cached.bundle_json,
      cached.content_version,
      cached.digest,
      cached.lexicon_text,
    );
    return {
      content,
      updateReady: cached.content_version !== response.content_version,
      fromCache: true,
    };
  }

  const content = PrototypeContent.fromActive(response);
  try {
    await storage.cacheBundle({
      learner_id: storage.learnerId,
      content_version: content.contentVersion,
      core_version: "",
      bundle_json: content.bundleJson,
      digest: content.digest,
      lexicon_text: content.lexiconText,
      cached_at: Date.now(),
    });
  } catch {
    // A failed cache should not prevent the first online session.
  }
  return { content, updateReady: false, fromCache: false };
}
