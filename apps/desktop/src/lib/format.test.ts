import { describe, expect, it } from "vitest";
import { formatBytes, formatPercent } from "./format";

describe("format helpers", () => {
  it("formats byte values", () => {
    expect(formatBytes(1024)).toBe("1.0 KB");
  });

  it("clamps progress values", () => {
    expect(formatPercent(150, 100)).toBe(100);
    expect(formatPercent(1, 0)).toBe(0);
  });
});

