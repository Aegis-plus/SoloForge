// The only bridge to Rust. In the Tauri window this calls real commands;
// in a plain browser (vite dev, tests, screenshots) it uses an in-memory
// mock so the UI can be developed and checked without the desktop shell.

type Unlisten = () => void;

export const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

export async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (inTauri) {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<T>(cmd, args);
  }
  const { mockInvoke } = await import("./mock");
  return mockInvoke(cmd, args ?? {}) as Promise<T>;
}

export async function on<T>(event: string, cb: (payload: T) => void): Promise<Unlisten> {
  if (inTauri) {
    const { listen } = await import("@tauri-apps/api/event");
    return listen<T>(event, (e) => cb(e.payload));
  }
  const { mockBus } = await import("./mock");
  const h = (e: Event) => cb((e as CustomEvent<T>).detail);
  mockBus.addEventListener(event, h);
  return () => mockBus.removeEventListener(event, h);
}

let imagesRoot = "";
let convert: ((p: string) => string) | null = null;

export async function initImages() {
  const info = await call<{ images_root: string }>("app_info");
  imagesRoot = info.images_root;
  if (inTauri) {
    const { convertFileSrc } = await import("@tauri-apps/api/core");
    convert = convertFileSrc;
  }
}

/** URL for a generated image inside a world's images folder. */
export function imageUrl(worldId: string, file: string | null | undefined): string | undefined {
  if (!file) return undefined;
  if (file.startsWith("data:")) return file;
  if (convert) return convert(`${imagesRoot}/${worldId}/images/${file}`);
  return undefined;
}

/** Errors from Rust arrive as plain strings. */
export function errText(e: unknown): string {
  if (typeof e === "string") return e;
  if (e instanceof Error) return e.message;
  return String(e);
}
