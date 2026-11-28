// Draft Castellan crest (G0). The final mark is chosen in G1; see
// docs/brand. Geometry follows the desktop master plan, section 8.2:
// a pointy-top hexagon open at the upper right, a three-bar tower that also
// reads as a rising chart, and the watch light in the gap.

import { useId } from "react";

interface CrestProps {
    readonly size?: number;
    readonly title?: string;
}

const BARS: readonly { x: number; top: number }[] = [
    { x: 33.1, top: 47 },
    { x: 45.5, top: 34 },
    { x: 57.9, top: 47 },
];
const BASE = 72;
const WIDTH = 9;

function bar(x: number, top: number): string {
    return [
        `M ${String(x)} ${String(BASE)}`,
        `L ${String(x)} ${String(top)}`,
        `L ${String(x + 3.3)} ${String(top)}`,
        `L ${String(x + 3.3)} ${String(top + 3)}`,
        `L ${String(x + 5.7)} ${String(top + 3)}`,
        `L ${String(x + 5.7)} ${String(top)}`,
        `L ${String(x + WIDTH)} ${String(top)}`,
        `L ${String(x + WIDTH)} ${String(BASE)}`,
        "Z",
    ].join(" ");
}

export function Crest({ size = 64, title = "Castellan" }: CrestProps) {
    const id = useId();
    const gradient = `${id}-aurora`;
    const glow = `${id}-glow`;
    return (
        <svg
            className="crest"
            width={size}
            height={size}
            viewBox="0 0 100 100"
            role="img"
            aria-label={title}
        >
            <defs>
                <linearGradient id={gradient} x1="0" y1="1" x2="1" y2="0">
                    <stop offset="0" stopColor="var(--brand-amethyst)" />
                    <stop offset="1" stopColor="var(--brand-plasma)" />
                </linearGradient>
                <filter id={glow} x="-50%" y="-50%" width="200%" height="200%">
                    <feGaussianBlur stdDeviation="2.4" />
                </filter>
            </defs>
            <path
                className="crest-ring"
                d="M 81.18 46.68 L 81.18 69 L 50 87 L 18.82 69 L 18.82 33 L 50 15 L 69.33 26.16"
                fill="none"
                stroke={`url(#${gradient})`}
                strokeWidth="4.5"
                strokeLinecap="round"
                strokeLinejoin="round"
            />
            {BARS.map((b, index) => (
                <path
                    key={b.x}
                    className="crest-bar"
                    data-index={index}
                    d={bar(b.x, b.top)}
                    fill={`url(#${gradient})`}
                />
            ))}
            <circle
                cx="81.18"
                cy="33"
                r="6"
                fill="var(--brand-plasma)"
                opacity="0.55"
                filter={`url(#${glow})`}
            />
            <circle className="crest-light" cx="81.18" cy="33" r="3.4" fill="var(--brand-plasma)" />
        </svg>
    );
}
