import { describe, expect, it } from "vitest";

import {
  ClientClock,
  calculateClockOffset,
  createUuidV7Generator,
} from "./uuidv7";

const UUID_V7 =
  /^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/;

describe("UUIDv7", () => {
  it("generates time-ordered unique ids when the clock stands still", () => {
    const generator = createUuidV7Generator(() => 1_700_000_000_000);
    const ids = Array.from({ length: 5000 }, () => generator.next());
    expect(new Set(ids).size).toBe(ids.length);
    for (const id of ids) {
      expect(id).toMatch(UUID_V7);
    }
    for (let index = 1; index < ids.length; index += 1) {
      const previous = ids[index - 1] ?? "";
      const current = ids[index] ?? "";
      expect(current > previous).toBe(true);
    }
  });

  it("keeps ordering when the wall clock moves backwards", () => {
    let now = 1_700_000_000_000;
    const generator = createUuidV7Generator(() => now);
    const first = generator.next();
    now -= 5000;
    const second = generator.next();
    expect(second > first).toBe(true);
  });
});

describe("clock offset", () => {
  it("uses server time minus local time and warns beyond 24 hours", () => {
    const day = 24 * 60 * 60 * 1000;
    const normal = calculateClockOffset(1_000_000 + 3_600_000, 1_000_000);
    expect(normal.offsetMs).toBe(3_600_000);
    expect(normal.suspicious).toBe(false);

    const suspicious = calculateClockOffset(1_000_000 + day + 1, 1_000_000);
    expect(suspicious.offsetMs).toBe(day + 1);
    expect(suspicious.suspicious).toBe(true);
  });

  it("creates event ids and client timestamps from corrected time", () => {
    const local = 1_700_000_000_000;
    const clock = new ClientClock(0, () => local);
    const offset = clock.update(local + 25 * 60 * 60 * 1000);
    expect(offset.suspicious).toBe(true);
    expect(clock.now()).toBe(local + 25 * 60 * 60 * 1000);
    expect(clock.nextEventId()).toMatch(UUID_V7);
    const stamp = clock.stamp();
    expect(stamp.client_ts).toBe(local + 25 * 60 * 60 * 1000);
    expect(stamp.event_id).toMatch(UUID_V7);
  });
});
