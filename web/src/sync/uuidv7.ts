const MAX_SEQUENCE = 0xfff;
const MAX_TIMESTAMP = 2 ** 48 - 1;
const MAX_OFFSET_MS = 24 * 60 * 60 * 1000;

export type RandomBytes = (length: number) => Uint8Array;

export interface UuidV7Generator {
  next(timestampMs?: number): string;
}

function cryptoRandomBytes(length: number): Uint8Array {
  const bytes = new Uint8Array(length);
  globalThis.crypto.getRandomValues(bytes);
  return bytes;
}

function hex(bytes: Uint8Array): string {
  let output = "";
  for (const byte of bytes) {
    output += byte.toString(16).padStart(2, "0");
  }
  return output;
}

export function createUuidV7Generator(
  now: () => number = Date.now,
  randomBytes: RandomBytes = cryptoRandomBytes,
): UuidV7Generator {
  let lastTimestamp = -1;
  let sequence = 0;

  return {
    next(timestampMs = now()): string {
      const supplied = Math.floor(timestampMs);
      if (!Number.isFinite(supplied) || supplied < 0) {
        throw new Error("UUIDv7 timestamp must be a non-negative number");
      }
      let timestamp = Math.min(supplied, MAX_TIMESTAMP);
      if (timestamp < lastTimestamp) {
        timestamp = lastTimestamp;
      }
      if (timestamp === lastTimestamp) {
        if (sequence >= MAX_SEQUENCE) {
          timestamp = Math.min(lastTimestamp + 1, MAX_TIMESTAMP);
          sequence = 0;
        } else {
          sequence += 1;
        }
      } else {
        sequence = 0;
      }
      lastTimestamp = timestamp;

      const bytes = new Uint8Array(16);
      const high = Math.floor(timestamp / 2 ** 32);
      const low = timestamp % 2 ** 32;
      bytes[0] = (high >> 8) & 0xff;
      bytes[1] = high & 0xff;
      bytes[2] = (low >> 24) & 0xff;
      bytes[3] = (low >> 16) & 0xff;
      bytes[4] = (low >> 8) & 0xff;
      bytes[5] = low & 0xff;
      bytes[6] = 0x70 | ((sequence >> 8) & 0x0f);
      bytes[7] = sequence & 0xff;

      const random = randomBytes(8);
      for (let index = 0; index < 8; index += 1) {
        bytes[8 + index] = random[index] ?? 0;
      }
      bytes[8] = ((bytes[8] ?? 0) & 0x3f) | 0x80;

      const value = hex(bytes);
      return [
        value.slice(0, 8),
        value.slice(8, 12),
        value.slice(12, 16),
        value.slice(16, 20),
        value.slice(20),
      ].join("-");
    },
  };
}

export interface ClockOffset {
  offsetMs: number;
  suspicious: boolean;
}

export interface EventStamp {
  event_id: string;
  client_ts: number;
}

export function calculateClockOffset(
  serverTimeMs: number,
  localTimeMs: number,
): ClockOffset {
  const offsetMs = Math.round(serverTimeMs - localTimeMs);
  return {
    offsetMs,
    suspicious: Math.abs(offsetMs) > MAX_OFFSET_MS,
  };
}

export class ClientClock {
  #offsetMs: number;
  #generator: UuidV7Generator;
  readonly #now: () => number;

  constructor(initialOffsetMs = 0, now: () => number = Date.now) {
    this.#offsetMs = initialOffsetMs;
    this.#now = now;
    this.#generator = createUuidV7Generator(() => this.now());
  }

  get offsetMs(): number {
    return this.#offsetMs;
  }

  setOffset(offsetMs: number): void {
    this.#offsetMs = offsetMs;
  }

  update(serverTimeMs: number, localTimeMs: number = this.#now()): ClockOffset {
    const offset = calculateClockOffset(serverTimeMs, localTimeMs);
    this.#offsetMs = offset.offsetMs;
    return offset;
  }

  now(): number {
    return Math.floor(this.#now() + this.#offsetMs);
  }

  nextEventId(timestampMs?: number): string {
    return this.#generator.next(timestampMs ?? this.now());
  }

  stamp(): EventStamp {
    const clientTs = this.now();
    return {
      event_id: this.#generator.next(clientTs),
      client_ts: clientTs,
    };
  }
}
