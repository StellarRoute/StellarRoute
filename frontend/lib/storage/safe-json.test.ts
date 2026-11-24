import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { readJson } from "./safe-json";

interface SampleRecord {
  count: number;
  tags: string[];
}

function isSampleRecord(v: unknown): v is SampleRecord {
  if (!v || typeof v !== "object" || Array.isArray(v)) return false;
  const candidate = v as Record<string, unknown>;
  return (
    typeof candidate.count === "number" &&
    Number.isFinite(candidate.count) &&
    Array.isArray(candidate.tags) &&
    candidate.tags.every((t): t is string => typeof t === "string")
  );
}

describe("readJson", () => {
  const KEY = "stellarroute-test-safe-json";
  const FALLBACK: SampleRecord = { count: 0, tags: [] };

  beforeEach(() => {
    localStorage.clear();
  });

  afterEach(() => {
    localStorage.clear();
    vi.restoreAllMocks();
  });

  it("returns fallback without calling removeItem when key is absent", () => {
    const removeSpy = vi.spyOn(localStorage, "removeItem");
    expect(readJson(KEY, isSampleRecord, FALLBACK)).toEqual(FALLBACK);
    expect(removeSpy).not.toHaveBeenCalled();
  });

  it("returns parsed value and preserves key when guard succeeds", () => {
    const valid: SampleRecord = { count: 3, tags: ["xlm", "usdc"] };
    localStorage.setItem(KEY, JSON.stringify(valid));

    expect(readJson(KEY, isSampleRecord, FALLBACK)).toEqual(valid);
    expect(localStorage.getItem(KEY)).toBe(JSON.stringify(valid));
  });

  it("returns fallback and removes key on malformed JSON", () => {
    localStorage.setItem(KEY, "{not-valid-json");

    expect(readJson(KEY, isSampleRecord, FALLBACK)).toEqual(FALLBACK);
    expect(localStorage.getItem(KEY)).toBeNull();
  });

  it("returns fallback and removes key when guard rejects wrong shape", () => {
    localStorage.setItem(KEY, JSON.stringify({ count: "three" }));

    expect(readJson(KEY, isSampleRecord, FALLBACK)).toEqual(FALLBACK);
    expect(localStorage.getItem(KEY)).toBeNull();
  });
});
