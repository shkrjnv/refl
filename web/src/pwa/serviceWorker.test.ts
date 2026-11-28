import { readFileSync } from "node:fs";

import { describe, expect, it } from "vitest";

import {
  ServiceWorkerUpdates,
  type ContainerLike,
  type InstallingWorkerLike,
  type RegistrationLike,
  type WaitingWorkerLike,
} from "./serviceWorker";

class FakeInstalling implements InstallingWorkerLike {
  state = "installing";
  #listeners: Array<() => void> = [];

  addEventListener(_type: "statechange", listener: () => void): void {
    this.#listeners.push(listener);
  }

  finish(): void {
    this.state = "installed";
    for (const listener of this.#listeners) {
      listener();
    }
  }
}

class FakeRegistration implements RegistrationLike {
  waiting: WaitingWorkerLike | null = null;
  installing: InstallingWorkerLike | null = null;
  #listeners: Array<() => void> = [];

  addEventListener(_type: "updatefound", listener: () => void): void {
    this.#listeners.push(listener);
  }

  findUpdate(worker: FakeInstalling): void {
    this.installing = worker;
    for (const listener of this.#listeners) {
      listener();
    }
  }
}

class FakeContainer implements ContainerLike {
  controller: unknown = null;
  #listeners: Array<() => void> = [];

  addEventListener(_type: "controllerchange", listener: () => void): void {
    this.#listeners.push(listener);
  }

  changeController(): void {
    for (const listener of this.#listeners) {
      listener();
    }
  }
}

describe("service worker updates", () => {
  it("does not reload a first install without a controller", () => {
    const registration = new FakeRegistration();
    const container = new FakeContainer();
    const installing = new FakeInstalling();
    registration.installing = installing;
    let reloads = 0;
    const updates = new ServiceWorkerUpdates({
      registration,
      container,
      reload: () => {
        reloads += 1;
      },
    });
    installing.finish();
    expect(updates.pending).toBe(false);
    expect(updates.apply()).toBe(false);
    expect(reloads).toBe(0);
  });

  it("waits for an explicit boundary before activating an update", () => {
    const registration = new FakeRegistration();
    const container = new FakeContainer();
    container.controller = {};
    const messages: unknown[] = [];
    const waiting: WaitingWorkerLike = {
      postMessage: (message) => messages.push(message),
    };
    let reloads = 0;
    const updates = new ServiceWorkerUpdates({
      registration,
      container,
      reload: () => {
        reloads += 1;
      },
    });
    const installing = new FakeInstalling();
    registration.findUpdate(installing);
    installing.finish();
    expect(updates.pending).toBe(true);
    registration.waiting = waiting;

    expect(updates.apply()).toBe(true);
    expect(messages).toEqual(["SKIP_WAITING"]);
    expect(reloads).toBe(0);
    container.changeController();
    expect(reloads).toBe(1);
  });

  it("never takes over a controlled page from the worker itself", () => {
    const worker = readFileSync(
      new URL("../../public/sw.js", import.meta.url),
      "utf8",
    );
    const install = worker.slice(
      worker.indexOf('addEventListener("install"'),
      worker.indexOf('addEventListener("activate"'),
    );
    expect(install).not.toContain("skipWaiting");
    expect(worker).not.toContain("self.clients.claim");
    expect(worker).toContain('addEventListener("message"');
    expect(worker).toContain('event.data === "SKIP_WAITING"');
  });
});
