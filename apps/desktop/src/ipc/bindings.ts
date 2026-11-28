// Generated from Rust by `apps/desktop/src-tauri/src/ipc.rs`. Do not edit.
// Refresh: CASTELLAN_UPDATE_BINDINGS=1 cargo test -p castellan-desktop bindings

export const CONTRACT_VERSION = 1;
export const EVENT_PULSE = "castellan:pulse";
export const EVENT_SWEEP = "castellan:sweep";
export const COMMANDS = ["contract_version","about","overview","facts"] as const;

export type Severity = "Info" | "Notice" | "Warning" | "Critical";

export type Pillar = "Storage" | "Stability" | "Performance" | "Security" | "Network";

export type Band = "Fortified" | "Sound" | "NeedsAttention" | "AtRisk";

export type DeductionLine = { 
/**
 * Finding that caused the line.
 */
findingId: string, 
/**
 * Pillar.
 */
pillar: Pillar, 
/**
 * Points actually removed by this finding after weight and curve.
 */
points: number, 
/**
 * Pillar ceiling applied because of this finding, if it was the tightest.
 */
vetoCeiling: number | null, };

export type PillarScore = { 
/**
 * Which pillar.
 */
pillar: Pillar, 
/**
 * `None` when the pillar was not assessed.
 */
score: number | null, 
/**
 * Confidence in thousandths. Unassessed pillars are 0.
 */
confidenceMillis: number, 
/**
 * Tightest veto ceiling applied, if any.
 */
vetoCeiling: number | null, };

export type ScoreReport = { 
/**
 * Overall score. `None` when nothing was assessed.
 */
keepScore: number | null, 
/**
 * Band for [`Self::keep_score`].
 */
band: Band | null, 
/**
 * Confidence in the overall score, in thousandths. Unassessed weight counts as zero.
 */
confidenceMillis: number, 
/**
 * One entry per pillar, in display order.
 */
pillars: Array<PillarScore>, 
/**
 * Deductions and vetoes, in the order they were applied.
 */
lines: Array<DeductionLine>, };

export type Memory = { 
/**
 * Installed memory Windows can use.
 */
totalBytes: number, 
/**
 * Memory that is free or can be reclaimed right now.
 */
availableBytes: number, };

export type Drive = { 
/**
 * Short name, such as `C:`.
 */
label: string, 
/**
 * Size of the volume.
 */
totalBytes: number, 
/**
 * Bytes free for this user.
 */
freeBytes: number, 
/**
 * Windows lives on this drive.
 */
system: boolean, };

export type Facts = { 
/**
 * Fixed local drives. The Windows drive comes first.
 */
drives: Array<Drive>, 
/**
 * Physical memory.
 */
memory: Memory | null, 
/**
 * Seconds since Windows started.
 */
uptimeSecs: number | null, 
/**
 * Logical processors available to this process.
 */
logicalCpus: number | null, 
/**
 * This binary is a development build, which refuses real changes.
 */
devBuild: boolean, };

export type Pulse = { 
/**
 * Busy share of all processors since the previous reading, 0 to 100.
 */
cpuPercent: number | null, 
/**
 * Physical memory right now.
 */
memory: Memory | null, 
/**
 * Seconds since Windows started.
 */
uptimeSecs: number | null, };

export type SweepSize = { 
/**
 * Files seen.
 */
files: number, 
/**
 * Their size.
 */
bytes: number, 
/**
 * Files old enough for a sweep to consider.
 */
agedFiles: number, 
/**
 * Their size.
 */
agedBytes: number, 
/**
 * The walk stopped early, so the numbers are a floor.
 */
truncated: boolean, };

export type SweepReading = { "state": "measuring" } | { "state": "measured", "detail": SweepSize } | { "state": "unavailable", "detail": string };

export type MeasuredTarget = "userTemp" | "windowsTemp";

export type Room = "overview" | "sweep" | "vitals" | "network" | "shield" | "boost" | "game" | "fix" | "ledger";

export type FindingView = { 
/**
 * Stable id, such as `STORAGE-001`.
 */
id: string, 
/**
 * How serious it is.
 */
severity: Severity, 
/**
 * One line, in English.
 */
title: string, 
/**
 * Evidence the person can check.
 */
detail: string, 
/**
 * What the person can do about it, in one sentence.
 */
advice: string, 
/**
 * Room that helps, if one does.
 */
room: Room | null, };

export type VolumeReading = { 
/**
 * Short label, such as `C:`.
 */
label: string, 
/**
 * Capacity in bytes.
 */
totalBytes: number, 
/**
 * Free bytes.
 */
freeBytes: number, };

export type Overview = { 
/**
 * Score and its explanation. Unassessed pillars have no number.
 */
score: ScoreReport, 
/**
 * Band safe to show. `None` while too little was inspected.
 */
displayBand: Band | null, 
/**
 * Findings that lower the score, most serious first.
 */
findings: Array<FindingView>, 
/**
 * One persona line for this reading.
 */
persona: string, 
/**
 * The Windows volume the score read.
 */
systemVolume: VolumeReading, };

export type IpcError = { 
/**
 * Stable code, such as `CST-E0101`.
 */
code: string, 
/**
 * What happened and what to do.
 */
message: string, };

export type About = { 
/**
 * Product name.
 */
name: string, 
/**
 * Semantic version.
 */
version: string, 
/**
 * Author credit.
 */
author: string, 
/**
 * Source repository.
 */
repository: string, 
/**
 * Development builds refuse real changes.
 */
devBuild: boolean, 
/**
 * Contract version the backend speaks.
 */
contractVersion: number, };

export type SweepEvent = { 
/**
 * Which folder.
 */
target: MeasuredTarget, 
/**
 * What was found.
 */
reading: SweepReading, };

