/**
 * Service-worker updates are deliberately controlled by the app, not by the
 * worker. The worker installs and waits; this controller sends SKIP_WAITING
 * only when a session boundary has been reached (docs/13).
 */

export interface WaitingWorkerLike {
  postMessage(message: unknown): void;
}

export interface InstallingWorkerLike {
  state: string;
  addEventListener(type: "statechange", listener: () => void): void;
}

export interface RegistrationLike {
  waiting: WaitingWorkerLike | null;
  installing: InstallingWorkerLike | null;
  addEventListener(type: "updatefound", listener: () => void): void;
}

export interface ContainerLike {
  controller: unknown;
  addEventListener(type: "controllerchange", listener: () => void): void;
}

export interface UpdateEnvironment {
  registration: RegistrationLike;
  container: ContainerLike;
  reload: () => void;
}

export class ServiceWorkerUpdates {
  readonly #registration: RegistrationLike;
  readonly #container: ContainerLike;
  readonly #reload: () => void;
  #pending = false;
  #applying = false;

  constructor(environment: UpdateEnvironment) {
    this.#registration = environment.registration;
    this.#container = environment.container;
    this.#reload = environment.reload;
    this.#container.addEventListener("controllerchange", () => {
      if (this.#applying) {
        this.#applying = false;
        this.#reload();
      }
    });
    this.#registration.addEventListener("updatefound", () => {
      this.#trackInstalling();
    });
    this.#trackInstalling();
    if (this.#registration.waiting) {
      this.#pending = true;
    }
  }

  get pending(): boolean {
    return this.#pending;
  }

  /**
   * Asks the waiting worker to activate. Returns false when no update is
   * waiting or the app is already applying one.
   */
  apply(): boolean {
    const waiting = this.#registration.waiting;
    if (!this.#pending || waiting === null || this.#applying) {
      return false;
    }
    this.#applying = true;
    waiting.postMessage("SKIP_WAITING");
    return true;
  }

  #trackInstalling(): void {
    const installing = this.#registration.installing;
    if (installing === null) {
      return;
    }
    const installed = () => {
      if (installing.state !== "installed") {
        return;
      }
      // A first install has no controller yet and activates on its own; only
      // an update to an already-controlled page needs a reload.
      this.#pending = this.#container.controller !== null;
    };
    installing.addEventListener("statechange", installed);
    installed();
  }
}

let current: ServiceWorkerUpdates | null = null;

export function currentServiceWorkerUpdates(): ServiceWorkerUpdates | null {
  return current;
}

/** Registers the generated shell worker once, after the page has loaded. */
export async function registerServiceWorker(): Promise<ServiceWorkerUpdates | null> {
  if (
    typeof navigator === "undefined" ||
    !("serviceWorker" in navigator) ||
    typeof window === "undefined"
  ) {
    return null;
  }
  const registration = await navigator.serviceWorker.register("/sw.js");
  current = new ServiceWorkerUpdates({
    registration,
    container: navigator.serviceWorker,
    reload: () => window.location.reload(),
  });
  return current;
}

/** Applies a waiting shell update if one exists. Never called mid-session. */
export function applyServiceWorkerUpdate(): boolean {
  return current?.apply() ?? false;
}
