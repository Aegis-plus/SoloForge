import { createEffect, createMemo, createSignal, For, on, onCleanup, Show } from "solid-js";
import type { Actor, Token } from "../lib/types";
import { act, currentTurnId, gm, imageUrl, pc, scene, world } from "./_shared";
import { reducedMotion, settings } from "../lib/store";
import { initials } from "../lib/format";

const BASE = 40;

export function useMapState() {
  const [zoom, setZoom] = createSignal(1);
  const [showGrid, setShowGrid] = createSignal(true);
  const [measure, setMeasure] = createSignal(false);
  return { zoom, setZoom, showGrid, setShowGrid, measure, setMeasure };
}
export type MapState = ReturnType<typeof useMapState>;

export function MapCanvas(props: { map: MapState; centerSignal: () => number }) {
  let canvas!: HTMLCanvasElement;
  let stage!: HTMLDivElement;
  const cell = () => BASE * props.map.zoom();
  const [selected, setSelected] = createSignal<string | null>(null);
  const [ghost, setGhost] = createSignal<{ x: number; y: number } | null>(null);
  const [target, setTarget] = createSignal<string | null>(null);
  const [dragging, setDragging] = createSignal(false);
  const [bg, setBg] = createSignal<HTMLImageElement | null>(null);

  const myToken = createMemo<Token | undefined>(() => scene()?.tokens.find((t) => t.actor_id === pc()?.id));
  const visible = createMemo(() => {
    const sc = scene(); const w = world(); if (!sc || !w) return [] as { t: Token; a: Actor }[];
    return sc.tokens.flatMap((t) => {
      const a = w.actors.find((x) => x.id === t.actor_id);
      if (!a || t.hidden) return [];
      const lit = !sc.fog || sc.revealed[t.y * sc.width + t.x];
      if (!lit && a.disposition !== "friendly" && a.kind !== "pc") return [];
      return [{ t, a }];
    });
  });

  // Load the generated map image when it changes.
  createEffect(on(() => [world()?.id, scene()?.background], () => {
    const url = imageUrl(world()?.id ?? "", scene()?.background);
    if (!url) { setBg(null); return; }
    const img = new Image();
    img.onload = () => setBg(img);
    img.src = url;
  }));

  // Draw floor, background, grid and fog.
  createEffect(() => {
    const sc = scene(); if (!sc || !canvas) return;
    settings(); // redraw fog when the theme changes
    const c = cell(); const W = sc.width * c, H = sc.height * c;
    const dpr = window.devicePixelRatio || 1;
    canvas.width = W * dpr; canvas.height = H * dpr;
    canvas.style.width = `${W}px`; canvas.style.height = `${H}px`;
    const g = canvas.getContext("2d"); if (!g) return;
    g.setTransform(dpr, 0, 0, dpr, 0, 0);
    const img = bg();
    if (img) g.drawImage(img, 0, 0, W, H);
    else {
      for (let y = 0; y < sc.height; y++) for (let x = 0; x < sc.width; x++) {
        g.fillStyle = (x + y) % 2 ? "#3b4250" : "#363d4a";
        g.fillRect(x * c, y * c, c, c);
      }
    }
    if (props.map.showGrid()) {
      g.strokeStyle = img ? "rgba(0,0,0,0.35)" : "rgba(11,14,19,0.55)";
      g.lineWidth = 1;
      g.beginPath();
      for (let x = 0; x <= sc.width; x++) { g.moveTo(x * c + 0.5, 0); g.lineTo(x * c + 0.5, H); }
      for (let y = 0; y <= sc.height; y++) { g.moveTo(0, y * c + 0.5); g.lineTo(W, y * c + 0.5); }
      g.stroke();
    }
    if (sc.fog) {
      g.fillStyle = document.documentElement.dataset.theme === "light" ? "rgba(20, 26, 36, 0.72)" : "rgba(5, 7, 10, 0.88)";
      for (let y = 0; y < sc.height; y++) for (let x = 0; x < sc.width; x++) if (!sc.revealed[y * sc.width + x]) g.fillRect(x * c, y * c, c, c);
    }
  });

  // Center on the hero when asked.
  createEffect(on(props.centerSignal, () => {
    const t = myToken(); if (!t || !stage) return;
    stage.scrollTo({ left: t.x * cell() - stage.clientWidth / 2 + 112, top: t.y * cell() - stage.clientHeight / 2 + 96, behavior: reducedMotion() ? "auto" : "smooth" });
  }, { defer: true }));

  const ringColor = (a: Actor) => (a.kind === "pc" ? "var(--sky)" : a.disposition === "hostile" ? "var(--wound)" : a.disposition === "friendly" ? "var(--vital)" : "var(--ink-muted)");
  const dist = (a: { x: number; y: number }, b: { x: number; y: number }) => Math.max(Math.abs(a.x - b.x), Math.abs(a.y - b.y)) * 5;
  const cellAt = (ev: { clientX: number; clientY: number }) => {
    const r = canvas.getBoundingClientRect();
    const sc = scene()!;
    return { x: Math.min(sc.width - 1, Math.max(0, Math.floor((ev.clientX - r.left) / cell()))), y: Math.min(sc.height - 1, Math.max(0, Math.floor((ev.clientY - r.top) / cell()))) };
  };

  async function confirmMove() {
    const t = myToken(); const g = ghost(); if (!t || !g) return;
    setGhost(null);
    if (g.x === t.x && g.y === t.y) return;
    await act("move_token", { tokenId: t.id, x: g.x, y: g.y });
  }

  function onTokenKey(e: KeyboardEvent, t: Token) {
    if (t.id !== myToken()?.id) return;
    const sc = scene()!;
    const g = ghost() ?? { x: t.x, y: t.y };
    const d: Record<string, [number, number]> = { ArrowLeft: [-1, 0], ArrowRight: [1, 0], ArrowUp: [0, -1], ArrowDown: [0, 1] };
    if (d[e.key]) {
      e.preventDefault();
      setSelected(t.id);
      setGhost({ x: Math.min(sc.width - 1, Math.max(0, g.x + d[e.key][0])), y: Math.min(sc.height - 1, Math.max(0, g.y + d[e.key][1])) });
    } else if (e.key === "Enter" && ghost()) { e.preventDefault(); void confirmMove(); }
    else if (e.key === "Escape") { setGhost(null); setSelected(null); }
  }

  function startDrag(e: PointerEvent, t: Token) {
    if (t.id !== myToken()?.id || e.button !== 0) return;
    const el = e.currentTarget as HTMLElement;
    el.setPointerCapture(e.pointerId);
    setSelected(t.id);
    let moved = false;
    const move = (ev: PointerEvent) => { moved = true; setDragging(true); setGhost(cellAt(ev)); };
    const up = () => {
      el.removeEventListener("pointermove", move); el.removeEventListener("pointerup", up);
      setDragging(false);
      if (moved) void confirmMove();
    };
    el.addEventListener("pointermove", move); el.addEventListener("pointerup", up);
  }

  const onBoardClick = (e: MouseEvent) => {
    if (e.target !== canvas) return;
    if (selected() && myToken()) setGhost(cellAt(e)); else setTarget(null);
  };
  const keyOff = (e: KeyboardEvent) => { if (e.key === "Escape") { setGhost(null); setTarget(null); } };
  window.addEventListener("keydown", keyOff);
  onCleanup(() => window.removeEventListener("keydown", keyOff));

  const targetActor = () => world()?.actors.find((a) => a.id === target());
  const targetToken = () => scene()?.tokens.find((t) => t.actor_id === target());

  return (
    <div class="map-stage" ref={stage}>
      <Show when={scene()} fallback={<div class="grid-bg" style={{ position: "absolute", inset: 0 }} />}>
        {(sc) => (
          <section class="map-board" aria-label={`Battle map: ${sc().name}, ${sc().width} by ${sc().height} squares of 5 feet`} style={{ width: `${sc().width * cell()}px`, height: `${sc().height * cell()}px` }}>
            <canvas ref={canvas} aria-hidden="true" onClick={onBoardClick} />
            <Show when={ghost() && myToken()}>
              <svg width={sc().width * cell()} height={sc().height * cell()} aria-hidden="true" style={{ position: "absolute", inset: 0, "pointer-events": "none" }}>
                <line x1={(myToken()!.x + 0.5) * cell()} y1={(myToken()!.y + 0.5) * cell()} x2={(ghost()!.x + 0.5) * cell()} y2={(ghost()!.y + 0.5) * cell()} stroke="#6cb8ff" stroke-width="2" stroke-dasharray="6 5" />
              </svg>
              <div class="ghost-cell" style={{ left: `${ghost()!.x * cell()}px`, top: `${ghost()!.y * cell()}px`, width: `${cell()}px`, height: `${cell()}px` }} />
              <Show when={!dragging()}>
                <button type="button" class="btn btn-sm" style={{ position: "absolute", left: `${ghost()!.x * cell() - 28}px`, top: `${(ghost()!.y + 1) * cell() + 4}px`, background: "var(--bg-200)", "z-index": 3 }} onClick={confirmMove}>
                  Move Here · {dist(myToken()!, ghost()!)}&nbsp;ft
                </button>
              </Show>
              <span class="mono small" aria-live="polite" style={{ position: "absolute", left: `${((myToken()!.x + ghost()!.x) / 2) * cell()}px`, top: `${((myToken()!.y + ghost()!.y) / 2) * cell() - 12}px`, padding: "2px 8px", "border-radius": "6px", background: "var(--bg-200)", color: "var(--sky)", "font-weight": 700, "box-shadow": "var(--shadow-float)", "pointer-events": "none", "white-space": "nowrap", "z-index": 3 }}>{dist(myToken()!, ghost()!)}&nbsp;ft</span>
            </Show>
            <For each={visible()}>
              {({ t, a }) => {
                const size = () => cell() - 8;
                const mine = () => a.kind === "pc";
                const current = () => currentTurnId() === a.id;
                const url = () => imageUrl(world()?.id ?? "", a.portrait);
                const hpPct = () => Math.max(0, Math.min(1, a.hp / Math.max(1, a.max_hp)));
                const label = () => `${a.name}${mine() ? ", your token" : `, ${a.disposition}`}${a.hp <= 0 ? ", down" : `, ${hpPct() > 0.5 ? "unhurt or lightly hurt" : "badly hurt"}`}${current() ? ", current turn" : ""}${myToken() && !mine() ? `, ${dist(t, myToken()!)} feet away` : ""}`;
                return (
                  <>
                    <Show when={current()}><span class="pulse" aria-hidden="true" style={{ position: "absolute", left: `${t.x * cell() - 2}px`, top: `${t.y * cell() - 2}px`, width: `${cell() + 4}px`, height: `${cell() + 4}px`, "border-radius": "9999px", border: "3px solid var(--brass)", "pointer-events": "none" }} /></Show>
                    <button type="button" class="token" aria-label={label()} aria-pressed={mine() ? selected() === t.id : target() === a.id}
                      data-dragging={dragging() && mine()}
                      title={mine() ? "Drag, or use the arrow keys, to move" : a.name}
                      style={{ left: `${t.x * cell() + 4}px`, top: `${t.y * cell() + 4}px`, width: `${size()}px`, height: `${size()}px`, background: mine() ? "#234566" : a.disposition === "hostile" ? "#47502a" : "#3a4150", color: "#e8ecf2", border: `3px solid ${a.hp <= 0 ? "#6b778a" : ringColor(a)}`, opacity: a.hp <= 0 ? 0.75 : 1, "z-index": 2 }}
                      onPointerDown={(e) => startDrag(e, t)}
                      onKeyDown={(e) => onTokenKey(e, t)}
                      onClick={() => { if (mine()) setSelected(selected() === t.id ? null : t.id); else setTarget(target() === a.id ? null : a.id); }}>
                      <Show when={a.hp > 0} fallback={<span aria-hidden="true">✕</span>}>
                        <Show when={url()} fallback={<span aria-hidden="true">{initials(a.name)}</span>}><img src={url()} alt="" width={size()} height={size()} /></Show>
                      </Show>
                    </button>
                    <Show when={a.hp > 0 && a.kind !== "pc"}><span class="hp-pip" aria-hidden="true" style={{ left: `${t.x * cell() + 8}px`, top: `${(t.y + 1) * cell() - 6}px`, width: `${(cell() - 16) * hpPct()}px`, background: hpPct() > 0.5 ? "var(--vital)" : "var(--wound)" }} /></Show>
                    <Show when={a.hp <= 0}><span class="token-label" style={{ left: `${(t.x + 0.5) * cell()}px`, top: `${(t.y + 1) * cell() - 2}px` }}>{a.name} · Down</span></Show>
                  </>
                );
              }}
            </For>
            <Show when={targetActor() && targetToken()}>
              <div class="panel" role="dialog" aria-label={`${targetActor()!.name}`} style={{ position: "absolute", left: `${(targetToken()!.x + 1) * cell() + 8}px`, top: `${targetToken()!.y * cell()}px`, width: "260px", padding: "12px", display: "flex", "flex-direction": "column", gap: "8px", "z-index": 4 }}>
                <div style={{ display: "flex", "justify-content": "space-between", gap: "8px" }}><strong class="truncate">{targetActor()!.name}</strong><span class="small muted">{targetActor()!.disposition === "hostile" ? "Hostile" : targetActor()!.disposition === "friendly" ? "Friendly" : "Neutral"}</span></div>
                <p class="small muted">{targetActor()!.hp <= 0 ? "Down" : targetActor()!.hp / targetActor()!.max_hp > 0.5 ? "Unhurt or lightly hurt" : "Badly hurt"}{myToken() ? ` · ${dist(targetToken()!, myToken()!)} ft away` : ""}</p>
                <Show when={targetActor()!.hp > 0}>
                  <For each={pc()?.attacks ?? []}>{(atk) => <button type="button" class="btn btn-sm" disabled={gm().state !== "idle"} onClick={async () => { await act("attack", { attack: atk.name, target: targetActor()!.id, adv: "" }); setTarget(null); }}>Attack with {atk.name}</button>}</For>
                </Show>
                <button type="button" class="btn btn-sm" onClick={() => setTarget(null)}>Close Target Card</button>
              </div>
            </Show>
          </section>
        )}
      </Show>
    </div>
  );
}
