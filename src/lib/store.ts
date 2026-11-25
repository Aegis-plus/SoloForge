import { createSignal, createMemo, batch } from "solid-js";
import { createStore, reconcile } from "solid-js/store";
import { call, errText, initImages, on } from "./ipc";
import type { Actor, GmStatus, Scene, Settings, World } from "./types";

export type Screen = "launcher" | "wizard" | "table" | "settings";

const [state, setState] = createStore<{ world: World | null }>({ world: null });
export const world = () => state.world;

export const [screen, setScreen] = createSignal<Screen>("launcher");
export const [settingsReturn, setSettingsReturn] = createSignal<Screen>("launcher");
export const [gm, setGm] = createSignal<GmStatus>({ state: "idle", detail: "" });
export const [streaming, setStreaming] = createSignal("");
export const [settings, setSettings] = createSignal<Settings | null>(null);
export const [toast, setToast] = createSignal<{ text: string; action?: { label: string; run: () => void } } | null>(null);
export const [systemReducedMotion, setSystemReducedMotion] = createSignal(false);

let toastTimer: ReturnType<typeof setTimeout> | undefined;
export function showToast(text: string, action?: { label: string; run: () => void }, ms = 10000) {
  clearTimeout(toastTimer);
  setToast({ text, action });
  toastTimer = setTimeout(() => setToast(null), ms);
}

export function setWorld(w: World | null) {
  setState("world", w ? reconcile(w, { key: "id" }) : null);
}

export const pc = createMemo<Actor | undefined>(() => state.world?.actors.find((a) => a.kind === "pc"));
export const scene = createMemo<Scene | undefined>(() => state.world?.scenes.find((s) => s.id === state.world?.active_scene));
export const actorById = (id: string) => state.world?.actors.find((a) => a.id === id);
export const currentTurnId = createMemo(() => {
  const c = state.world?.combat;
  return c ? c.order[c.turn]?.actor_id : undefined;
});
export const reducedMotion = () => systemReducedMotion() || Boolean(settings()?.appearance.reduce_motion);

export function applyTheme(s: Settings | null) {
  const pref = s?.appearance.theme ?? "dark";
  const dark = pref === "system" ? window.matchMedia("(prefers-color-scheme: dark)").matches : pref !== "light";
  const html = document.documentElement;
  html.dataset.theme = dark ? "dark" : "light";
  html.dataset.contrast = s?.appearance.high_contrast ? "high" : "normal";
  html.dataset.motion = reducedMotion() ? "reduce" : "full";
  html.style.setProperty("--narration-px", `${s?.appearance.narration_px ?? 17}px`);
  document.querySelector('meta[name="theme-color"]')?.setAttribute("content", dark ? "#0b0e13" : "#eef1f5");
}

export async function boot() {
  const mq = window.matchMedia("(prefers-reduced-motion: reduce)");
  setSystemReducedMotion(mq.matches);
  mq.addEventListener("change", (e) => { setSystemReducedMotion(e.matches); applyTheme(settings()); });
  await initImages();
  const s = await call<Settings>("get_settings");
  setSettings(s);
  applyTheme(s);
  await on<World>("world://changed", (w) => setWorld(w));
  await on<GmStatus>("gm://status", (s) => batch(() => {
    setGm(s);
    if (s.state === "idle" || s.state === "error") setStreaming("");
  }));
  await on<{ text: string }>("gm://delta", (d) => setStreaming((t) => t + d.text));
  await on("gm://message-done", () => setStreaming(""));
  const w = await call<World | null>("get_world");
  if (w) { setWorld(w); setScreen("table"); }
}

/** Run a command; show its error as a toast instead of throwing. */
export async function act<T>(cmd: string, args?: Record<string, unknown>): Promise<T | undefined> {
  try {
    return await call<T>(cmd, args);
  } catch (e) {
    showToast(errText(e), undefined, 8000);
    return undefined;
  }
}

export async function openWorld(id: string) {
  const w = await act<World>("open_world", { id });
  if (w) { setWorld(w); setScreen("table"); }
}

export async function leaveWorld() {
  await act("close_world");
  setWorld(null);
  setScreen("launcher");
}

export function openSettings() {
  setSettingsReturn(screen());
  setScreen("settings");
}
