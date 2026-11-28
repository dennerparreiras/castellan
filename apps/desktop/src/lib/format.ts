// Formatting that matches the Rust side, so every frontend says the same number.

const KIB = 1024;
const MIB = KIB * 1024;
const GIB = MIB * 1024;

function unit(bytes: number, size: number, label: string): string {
    const whole = Math.floor(bytes / size);
    const tenth = Math.floor(((bytes % size) * 10) / size);
    return tenth === 0 ? `${String(whole)} ${label}` : `${String(whole)}.${String(tenth)} ${label}`;
}

/** Binary units labeled the way Explorer labels them: `27.8 GB`. */
export function formatBytes(bytes: number): string {
    if (!Number.isFinite(bytes) || bytes < 0) return "0 B";
    if (bytes >= GIB) return unit(bytes, GIB, "GB");
    if (bytes >= MIB) return unit(bytes, MIB, "MB");
    if (bytes >= KIB) return unit(bytes, KIB, "KB");
    return `${String(Math.floor(bytes))} B`;
}

/** `93784` seconds as `1 d 2 h`. */
export function formatSpan(seconds: number): string {
    const days = Math.floor(seconds / 86_400);
    const hours = Math.floor((seconds % 86_400) / 3_600);
    const minutes = Math.floor((seconds % 3_600) / 60);
    if (days > 0) return `${String(days)} d ${String(hours)} h`;
    if (hours > 0) return `${String(hours)} h ${String(minutes)} min`;
    return `${String(minutes)} min`;
}

/** Share of `part` in `whole`, 0 to 100. */
export function percent(part: number, whole: number): number {
    if (whole <= 0) return 0;
    return Math.min(100, Math.max(0, Math.floor((part * 100) / whole)));
}
