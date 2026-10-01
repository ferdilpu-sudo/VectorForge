// Explicit P1 adapter. No silent production fallback and no native I/O claim.
import { generatePreview } from "./mock-engine";
import { readBrowserFile } from "./import-files";
export const api = { mode: "demo" as const, generatePreview, readBrowserFile };
