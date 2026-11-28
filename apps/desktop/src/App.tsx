import { useEffect, useState } from "react";

import { Crest } from "./brand/Crest";
import type { About, Facts, Overview, Pulse } from "./ipc/bindings";
import { about, checkContract, facts, onPulse, overview } from "./ipc/client";
import { formatBytes, formatSpan, percent } from "./lib/format";

interface Ready {
    readonly state: "ready";
    readonly about: About;
    readonly facts: Facts;
    readonly overview: Overview | null;
    readonly overviewError: string | null;
}

type Load =
    { readonly state: "loading" } | { readonly state: "failed"; readonly message: string } | Ready;

type Tone = "ok" | "steady" | "warn" | "critical" | "quiet";

/** Higher is better, as in the Keep Score. */
export function scoreTone(value: number | null): Tone {
    if (value === null) return "quiet";
    if (value >= 90) return "ok";
    if (value >= 75) return "steady";
    if (value >= 50) return "warn";
    return "critical";
}

/** Higher is worse, as in memory or disk use. */
export function usageTone(value: number | null): Tone {
    if (value === null) return "quiet";
    if (value >= 90) return "critical";
    if (value >= 75) return "warn";
    return "steady";
}

interface TileProps {
    readonly label: string;
    readonly value: string;
    readonly tone: Tone;
    readonly note: string;
}

function Tile({ label, value, tone, note }: TileProps) {
    return (
        <section className="tile" aria-label={label}>
            <span className="tile-label">{label}</span>
            <span className={`tile-value tone-${tone}`}>{value}</span>
            <span className="tile-note">{note}</span>
        </section>
    );
}

function errorText(error: unknown): string {
    return error instanceof Error ? error.message : String(error);
}

export function App() {
    const [load, setLoad] = useState<Load>({ state: "loading" });
    const [pulse, setPulse] = useState<Pulse | null>(null);

    useEffect(() => {
        let alive = true;
        const start = async () => {
            try {
                await checkContract();
                const [info, pc] = await Promise.all([about(), facts()]);
                let view: Overview | null = null;
                let overviewError: string | null = null;
                try {
                    view = await overview();
                } catch (error: unknown) {
                    overviewError = errorText(error);
                }
                if (alive) {
                    setLoad({
                        state: "ready",
                        about: info,
                        facts: pc,
                        overview: view,
                        overviewError,
                    });
                }
            } catch (error: unknown) {
                if (alive) setLoad({ state: "failed", message: errorText(error) });
            }
        };
        void start();
        const stop = onPulse((reading) => {
            if (alive) setPulse(reading);
        });
        return () => {
            alive = false;
            void stop.then((unlisten) => {
                unlisten();
            });
        };
    }, []);

    const build =
        load.state === "ready"
            ? `v${load.about.version} · ${load.about.devBuild ? "development build · " : ""}read-only`
            : "read-only";

    return (
        <main className="app">
            <div className="stage">
                <div className="mark">
                    <Crest size={112} />
                    <h1 className="wordmark">CASTELLAN</h1>
                    <p className="tagline">The keeper of your machine.</p>
                </div>
                {load.state === "failed" && (
                    <div className="notice" role="alert">
                        <strong>Castellan could not start.</strong> {load.message}
                    </div>
                )}
                {load.state === "loading" && (
                    <p className="tagline" role="status">
                        Lighting the torches.
                    </p>
                )}
                {load.state === "ready" && <Readings ready={load} pulse={pulse} />}
            </div>
            <footer className="footer">
                <span>Crafted by Denner Parreiras</span>
                <span>{build}</span>
            </footer>
        </main>
    );
}

function Readings({ ready, pulse }: { readonly ready: Ready; readonly pulse: Pulse | null }) {
    const score = ready.overview?.score.keepScore ?? null;
    const checked = ready.overview?.score.pillars.filter((p) => p.score !== null).length ?? 0;
    const band = ready.overview?.displayBand ?? null;
    const scoreNote =
        ready.overviewError ?? `${band ?? "Partial"}: ${String(checked)} of 5 areas checked`;

    const cpu = pulse?.cpuPercent ?? null;
    const threads = ready.facts.logicalCpus ?? 0;

    const memory = pulse?.memory ?? ready.facts.memory;
    const memoryUsed = memory ? memory.totalBytes - memory.availableBytes : 0;
    const memoryShare = memory ? percent(memoryUsed, memory.totalBytes) : null;

    const drive = ready.facts.drives.find((d) => d.system) ?? null;
    const driveShare = drive ? percent(drive.totalBytes - drive.freeBytes, drive.totalBytes) : null;
    const uptime = pulse?.uptimeSecs ?? ready.facts.uptimeSecs;
    const driveNote = drive
        ? `${formatBytes(drive.freeBytes)} free${uptime === null ? "" : ` · up ${formatSpan(uptime)}`}`
        : "Not read";

    return (
        <div className="tiles">
            <Tile
                label="Keep Score"
                value={score === null ? "--" : String(score)}
                tone={scoreTone(score)}
                note={scoreNote}
            />
            <Tile
                label="Processor"
                value={cpu === null ? "--" : `${String(cpu)}%`}
                tone={usageTone(cpu)}
                note={
                    cpu === null
                        ? "Measuring…"
                        : `busy across ${String(threads)} logical processors`
                }
            />
            <Tile
                label="Memory"
                value={memoryShare === null ? "--" : `${String(memoryShare)}%`}
                tone={usageTone(memoryShare)}
                note={
                    memory
                        ? `${formatBytes(memoryUsed)} of ${formatBytes(memory.totalBytes)}`
                        : "Not read"
                }
            />
            <Tile
                label={drive ? `Drive ${drive.label}` : "Windows drive"}
                value={driveShare === null ? "--" : `${String(driveShare)}%`}
                tone={usageTone(driveShare)}
                note={driveNote}
            />
        </div>
    );
}
