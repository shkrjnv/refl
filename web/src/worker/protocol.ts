import {
  isJsonObject,
  isJsonValue,
  type JsonValue,
} from "../types/json";

export type CoreRequest =
  | { id: number; method: "coreInfo" }
  | {
      id: number;
      method: "checkBundleCompatibility";
      bundleJson: string;
    }
  | { id: number; method: "decodeEvent"; event: JsonValue }
  | {
      id: number;
      method: "foldState";
      bundleJson: string;
      lexiconJson: string;
      snapshotJson: string | null;
      events: JsonValue[];
    }
  | {
      id: number;
      method: "planSession";
      bundleJson: string;
      snapshotJson: string;
      nowMs: number;
      budgetMs: number;
    }
  | {
      id: number;
      method: "nextItem";
      bundleJson: string;
      snapshotJson: string;
      planJson: string;
      nowMs: number;
    }
  | {
      id: number;
      method: "replanSession";
      bundleJson: string;
      snapshotJson: string;
      planJson: string;
      outcome: JsonValue;
      nowMs: number;
    }
  | {
      id: number;
      method: "checkAnswer";
      bundleJson: string;
      lexiconJson: string;
      itemRef: JsonValue;
      response: JsonValue;
      elapsedMs: number;
    };

export interface CoreRpcErrorPayload {
  code: string;
  message: string;
}

export type CoreResponse =
  | { id: number; ok: true; result: JsonValue }
  | { id: number; ok: false; error: CoreRpcErrorPayload };

function isFiniteNumber(value: JsonValue | undefined): value is number {
  return typeof value === "number" && Number.isFinite(value);
}

export function isCoreRequest(value: unknown): value is CoreRequest {
  if (!isJsonObject(value)) {
    return false;
  }
  const id = value["id"];
  const method = value["method"];
  if (typeof id !== "number" || !Number.isInteger(id)) {
    return false;
  }
  if (typeof method !== "string") {
    return false;
  }
  switch (method) {
    case "coreInfo":
      return true;
    case "checkBundleCompatibility":
      return typeof value["bundleJson"] === "string";
    case "decodeEvent":
      return isJsonValue(value["event"]);
    case "foldState":
      return (
        typeof value["bundleJson"] === "string" &&
        typeof value["lexiconJson"] === "string" &&
        (value["snapshotJson"] === null ||
          typeof value["snapshotJson"] === "string") &&
        Array.isArray(value["events"]) &&
        value["events"].every(isJsonValue)
      );
    case "planSession":
      return (
        typeof value["bundleJson"] === "string" &&
        typeof value["snapshotJson"] === "string" &&
        isFiniteNumber(value["nowMs"]) &&
        isFiniteNumber(value["budgetMs"])
      );
    case "nextItem":
      return (
        typeof value["bundleJson"] === "string" &&
        typeof value["snapshotJson"] === "string" &&
        typeof value["planJson"] === "string" &&
        isFiniteNumber(value["nowMs"])
      );
    case "replanSession":
      return (
        typeof value["bundleJson"] === "string" &&
        typeof value["snapshotJson"] === "string" &&
        typeof value["planJson"] === "string" &&
        isJsonValue(value["outcome"]) &&
        isFiniteNumber(value["nowMs"])
      );
    case "checkAnswer":
      return (
        typeof value["bundleJson"] === "string" &&
        typeof value["lexiconJson"] === "string" &&
        isJsonValue(value["itemRef"]) &&
        isJsonValue(value["response"]) &&
        isFiniteNumber(value["elapsedMs"])
      );
    default:
      return false;
  }
}

export function isCoreResponse(value: unknown): value is CoreResponse {
  if (!isJsonObject(value)) {
    return false;
  }
  const id = value["id"];
  const ok = value["ok"];
  if (typeof id !== "number" || !Number.isInteger(id)) {
    return false;
  }
  if (ok === true) {
    return isJsonValue(value["result"]);
  }
  if (ok === false) {
    const error = value["error"];
    return (
      isJsonObject(error) &&
      typeof error["code"] === "string" &&
      typeof error["message"] === "string"
    );
  }
  return false;
}
