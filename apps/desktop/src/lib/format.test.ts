import { describe, expect, it } from "vitest";

import { formatBytes, formatSpan, percent } from "./format";

describe("formatBytes", () => {
    it("matches the Rust formatter", () => {
        expect(formatBytes(0)).toBe("0 B");
        expect(formatBytes(1536)).toBe("1.5 KB");
        expect(formatBytes(2_254_857_831)).toBe("2.1 GB");
        expect(formatBytes(20 * 1024 ** 3)).toBe("20 GB");
    });

    it("never prints a negative or broken number", () => {
        expect(formatBytes(-5)).toBe("0 B");
        expect(formatBytes(Number.NaN)).toBe("0 B");
    });
});

describe("formatSpan and percent", () => {
    it("reads like the terminal view", () => {
        expect(formatSpan(59)).toBe("0 min");
        expect(formatSpan(3_700)).toBe("1 h 1 min");
        expect(formatSpan(93_784)).toBe("1 d 2 h");
        expect(percent(94, 100)).toBe(94);
        expect(percent(1, 0)).toBe(0);
        expect(percent(200, 100)).toBe(100);
    });
});
