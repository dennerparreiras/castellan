// Typed access to the Castellan backend. Every call here is read-only.
// The shapes come from `bindings.ts`, which is generated from the Rust types.

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import {
    CONTRACT_VERSION,
    EVENT_PULSE,
    EVENT_SWEEP,
    type About,
    type Facts,
    type IpcError,
    type Overview,
    type Pulse,
    type SweepEvent,
} from "./bindings";

/** Thrown when the window and the backend were built from different contracts. */
export class ContractMismatch extends Error {
    constructor(readonly backend: number) {
        super(
            `This window expects contract ${String(CONTRACT_VERSION)} but the backend speaks ${String(backend)}. Reinstall Castellan.`,
        );
        this.name = "ContractMismatch";
    }
}

/** An error from the backend, with a stable code and a plain message. */
export class BackendError extends Error {
    constructor(
        readonly code: string,
        message: string,
    ) {
        super(message);
        this.name = "BackendError";
    }
}

function isIpcError(value: unknown): value is IpcError {
    return (
        typeof value === "object" &&
        value !== null &&
        "code" in value &&
        "message" in value &&
        typeof value.code === "string" &&
        typeof value.message === "string"
    );
}

async function call<T>(command: string): Promise<T> {
    try {
        return await invoke<T>(command);
    } catch (error: unknown) {
        if (isIpcError(error)) {
            throw new BackendError(error.code, error.message);
        }
        throw new BackendError("CST-E0100", String(error));
    }
}

/** Check the contract before anything else. Throws `ContractMismatch`. */
export async function checkContract(): Promise<void> {
    const backend = await call<number>("contract_version");
    if (backend !== CONTRACT_VERSION) {
        throw new ContractMismatch(backend);
    }
}

export const about = (): Promise<About> => call<About>("about");
export const overview = (): Promise<Overview> => call<Overview>("overview");
export const facts = (): Promise<Facts> => call<Facts>("facts");

/** Live readings, about once a second. Returns the function that stops listening. */
export function onPulse(handler: (pulse: Pulse) => void): Promise<UnlistenFn> {
    return listen<Pulse>(EVENT_PULSE, (event) => {
        handler(event.payload);
    });
}

/** Each sweep target as it finishes its read-only walk. */
export function onSweep(handler: (event: SweepEvent) => void): Promise<UnlistenFn> {
    return listen<SweepEvent>(EVENT_SWEEP, (event) => {
        handler(event.payload);
    });
}
