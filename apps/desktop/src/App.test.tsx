import { render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { App, scoreTone, usageTone } from "./App";
import { CONTRACT_VERSION, type About, type Facts, type Overview } from "./ipc/bindings";

const invoke = vi.fn<(command: string) => Promise<unknown>>();
const listen = vi.fn(() => Promise.resolve(() => undefined));

vi.mock("@tauri-apps/api/core", () => ({ invoke: (command: string) => invoke(command) }));
vi.mock("@tauri-apps/api/event", () => ({ listen: () => listen() }));

const GB = 1024 ** 3;

const ABOUT: About = {
    name: "Castellan",
    version: "0.1.0",
    author: "Denner Parreiras",
    repository: "https://github.com/dennerparreiras/castellan",
    devBuild: true,
    contractVersion: CONTRACT_VERSION,
};

const FACTS: Facts = {
    drives: [{ label: "C:", totalBytes: 500 * GB, freeBytes: 30 * GB, system: true }],
    memory: { totalBytes: 16 * GB, availableBytes: 6 * GB },
    uptimeSecs: 3 * 86_400,
    logicalCpus: 8,
    devBuild: true,
};

const OVERVIEW: Overview = {
    score: {
        keepScore: 64,
        band: "NeedsAttention",
        confidenceMillis: 220,
        pillars: [
            { pillar: "Storage", score: 64, confidenceMillis: 1000, vetoCeiling: null },
            { pillar: "Stability", score: null, confidenceMillis: 0, vetoCeiling: null },
            { pillar: "Performance", score: null, confidenceMillis: 0, vetoCeiling: null },
            { pillar: "Security", score: null, confidenceMillis: 0, vetoCeiling: null },
            { pillar: "Network", score: null, confidenceMillis: 0, vetoCeiling: null },
        ],
        lines: [],
    },
    displayBand: null,
    findings: [],
    persona: "One thing needs your attention: the system drive is 94% full.",
    systemVolume: { label: "C:", totalBytes: 500 * GB, freeBytes: 30 * GB },
};

function backend(version = CONTRACT_VERSION) {
    invoke.mockImplementation((command: string) => {
        switch (command) {
            case "contract_version":
                return Promise.resolve(version);
            case "about":
                return Promise.resolve(ABOUT);
            case "facts":
                return Promise.resolve(FACTS);
            case "overview":
                return Promise.resolve(OVERVIEW);
            default:
                return Promise.reject(new Error(`unexpected command ${command}`));
        }
    });
}

describe("App", () => {
    beforeEach(() => {
        invoke.mockReset();
    });

    it("shows measured readings and never claims a band for a partial score", async () => {
        backend();
        render(<App />);
        expect(screen.getByRole("status")).toHaveTextContent("Lighting the torches.");
        expect(await screen.findByText("64")).toBeInTheDocument();
        expect(screen.getByText("Partial: 1 of 5 areas checked")).toBeInTheDocument();
        expect(screen.getByText("94%")).toBeInTheDocument();
        expect(screen.getByText("62%")).toBeInTheDocument();
        expect(screen.getByText(/30 GB free/)).toBeInTheDocument();
        expect(screen.queryByText(/NeedsAttention/)).not.toBeInTheDocument();
        expect(screen.getByText("Crafted by Denner Parreiras")).toBeInTheDocument();
    });

    it("refuses to run against a different contract", async () => {
        backend(CONTRACT_VERSION + 1);
        render(<App />);
        expect(await screen.findByRole("alert")).toHaveTextContent("Reinstall Castellan");
        expect(invoke).not.toHaveBeenCalledWith("overview");
    });

    it("shows a backend error in plain words", async () => {
        invoke.mockImplementation((command: string) =>
            command === "contract_version"
                ? Promise.resolve(CONTRACT_VERSION)
                : // Tauri rejects a failed command with the serialized error object, not an Error.
                  // eslint-disable-next-line @typescript-eslint/prefer-promise-reject-errors
                  Promise.reject({ code: "CST-E0101", message: "The system could not be read." }),
        );
        render(<App />);
        expect(await screen.findByRole("alert")).toHaveTextContent("The system could not be read.");
    });
});

describe("tones", () => {
    it("colors scores and usage the same way as the terminal view", () => {
        expect(scoreTone(null)).toBe("quiet");
        expect(scoreTone(95)).toBe("ok");
        expect(scoreTone(80)).toBe("steady");
        expect(scoreTone(60)).toBe("warn");
        expect(scoreTone(10)).toBe("critical");
        expect(usageTone(50)).toBe("steady");
        expect(usageTone(80)).toBe("warn");
        expect(usageTone(95)).toBe("critical");
    });
});
