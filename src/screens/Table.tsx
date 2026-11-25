import { createMemo, createSignal, For, onCleanup, onMount, Show } from "solid-js";
import { act, actorById, currentTurnId, gm, imageUrl, pc, scene, showToast, world } from "../components/_shared";
import { Drawer } from "../components/Drawer";
import { Icon } from "../components/Icon";
import { MapCanvas, useMapState } from "../components/MapCanvas";
import { Sidebar, type Tab } from "../components/Sidebar";
import { initials, mod, signed, skillBonus } from "../lib/format";
import { leaveWorld } from "../lib/store";
import { Story } from "./Story";

interface Slot { key: string; label: string; sub: string; aria: string; run: () => void; pressed?: boolean }

export function Table() {
  const map = useMapState();
  const [tab, setTab] = createSignal<Tab>("chat");
  const [expanded, setExpanded] = createSignal(true);
  const [drawer, setDrawer] = createSignal(false);
  const [center, setCenter] = createSignal(0);
  const right = () => (expanded() ? "404px" : "80px");
  const busy = () => gm().state !== "idle" && gm().state !== "error";
  const myTurn = () => !world()?.combat || currentTurnId() === pc()?.id;

  const livingHostiles = createMemo(() => (scene()?.tokens ?? []).map((t) => actorById(t.actor_id)).filter((a) => a && a.disposition === "hostile" && a.hp > 0 && !scene()!.tokens.find((t) => t.actor_id === a.id)?.hidden));

  const slots = createMemo<Slot[]>(() => {
    const a = pc(); if (!a) return [];
    const out: Slot[] = [];
    for (const atk of a.attacks.slice(0, 2)) out.push({
      key: "", label: atk.name.split(" ")[0], sub: signed(atk.to_hit), aria: `${atk.name} attack, ${signed(atk.to_hit)}`,
      run: () => {
        const foes = livingHostiles();
        if (foes.length === 1) void act("attack", { attack: atk.name, target: foes[0]!.id, adv: "" });
        else showToast(foes.length ? "Select a target token on the map, then choose Attack." : "There’s nothing hostile to attack here.", undefined, 5000);
      },
    });
    for (const s of a.spells.slice(0, 2)) out.push({ key: "", label: s.name.split(/[\s’']/)[0], sub: a.concentrating === s.name ? "On" : s.level ? `L${s.level}` : "Cast", aria: `${s.name}${a.concentrating === s.name ? ", active" : ""}`, pressed: a.concentrating === s.name, run: () => act("send_message", { text: `I cast ${s.name}.` }) });
    const sk: [string, string, "dex" | "wis"][] = [["stealth", "Hide", "dex"], ["perception", "Look", "wis"]];
    for (const [k, l, ab] of sk) out.push({ key: "", label: l, sub: signed(skillBonus(a, k, ab)), aria: `${k[0].toUpperCase() + k.slice(1)} check, ${signed(skillBonus(a, k, ab))}`, run: () => act("check", { kind: k, adv: "" }) });
    const potion = a.inventory.find((i) => /potion/i.test(i.name));
    if (potion) out.push({ key: "", label: "Potion", sub: `×${potion.qty}`, aria: `Drink ${potion.name}, ${potion.qty} left`, run: () => act("send_message", { text: `I drink a ${potion.name.toLowerCase()}.` }) });
    out.push({ key: "", label: "Dash", sub: "Act", aria: "Dash", run: () => act("send_message", { text: "I take the Dash action." }) });
    return out.slice(0, 8).map((s, i) => ({ ...s, key: String(i + 1) }));
  });

  const onKey = (e: KeyboardEvent) => {
    const el = e.target as HTMLElement;
    if (el.closest("input, textarea, select, [contenteditable]") || e.metaKey || e.ctrlKey || e.altKey) return;
    const n = Number(e.key);
    if (n >= 1 && n <= 8 && slots()[n - 1]) { e.preventDefault(); slots()[n - 1].run(); }
    else if (e.key === "[") setExpanded(false);
    else if (e.key === "]") setExpanded(true);
    else if (e.key === "Escape" && busy()) void act("stop_gm");
  };
  onMount(() => window.addEventListener("keydown", onKey));
  onCleanup(() => window.removeEventListener("keydown", onKey));

  return (
    <Show when={!world()?.story_mode} fallback={<Story />}>
      <div style={{ position: "relative", width: "100%", height: "100%", "min-height": "760px", overflow: "hidden", background: "var(--bg-000)" }}>
        <a class="skip btn btn-primary" href="#chat-log">Skip to Chat</a>
        <h1 class="sr-only"><span translate="no">SoloForge</span> table — {world()?.name}, {scene()?.name ?? "no scene"}</h1>

        <MapCanvas map={map} centerSignal={center} />

        <Show when={scene()?.map_status === "pending"}>
          <div class="panel" role="status" aria-live="polite" style={{ position: "absolute", left: "50%", top: "50%", transform: "translate(-50%, -50%)", width: "280px", padding: "14px", display: "flex", "flex-direction": "column", gap: "8px", "z-index": 3 }}>
            <strong>Drawing “{scene()!.name}”…</strong><div class="bar" aria-hidden="true" /><p class="small muted">Tokens are already placed. The art fills in when it’s ready.</p>
          </div>
        </Show>

        <nav aria-label="World" class="panel" style={{ position: "absolute", left: "12px", top: "12px", height: "52px", display: "flex", "align-items": "center", gap: "4px", padding: "0 6px 0 14px", "z-index": 5, "max-width": "320px" }}>
          <button type="button" class="serif" translate="no" onClick={leaveWorld} title="Back to Your Worlds" style={{ "font-size": "20px", "font-weight": 700, color: "var(--brass)", background: "transparent", border: 0, padding: "0 6px 0 0" }}>SoloForge</button>
          <span class="truncate" style={{ "max-width": "160px", padding: "0 6px" }} title={scene()?.name}>{scene()?.name ?? world()?.name}</span>
          <button type="button" class="icon-btn" aria-label="Story Mode" title="Story Mode" onClick={() => act("set_story_mode", { on: true })}><Icon name="book" /></button>
        </nav>

        <div role="toolbar" aria-label="Map tools" aria-orientation="vertical" class="panel" style={{ position: "absolute", left: "12px", top: "76px", width: "56px", padding: "7px", display: "flex", "flex-direction": "column", gap: "4px", "z-index": 5 }}>
          <button type="button" class="icon-btn" aria-label="Show Grid" title="Grid" aria-pressed={map.showGrid()} onClick={() => map.setShowGrid(!map.showGrid())}><Icon name="grid" /></button>
          <span aria-hidden="true" style={{ height: "1px", background: "var(--line)", margin: "2px 4px" }} />
          <button type="button" class="icon-btn" aria-label="Zoom In" title="Zoom In" onClick={() => map.setZoom(Math.min(2, map.zoom() + 0.25))}><Icon name="plus" /></button>
          <button type="button" class="icon-btn" aria-label="Zoom Out" title="Zoom Out" onClick={() => map.setZoom(Math.max(0.5, map.zoom() - 0.25))}><Icon name="minus" /></button>
          <button type="button" class="icon-btn" aria-label="Center on Your Token" title="Center" onClick={() => setCenter(center() + 1)}><Icon name="center" /></button>
        </div>

        <Show when={world()?.combat}>
          {(c) => (
            <div style={{ position: "absolute", left: "330px", right: right(), top: "16px", display: "flex", "justify-content": "center", "z-index": 4, "pointer-events": "none" }}>
              <ol aria-label={`Initiative, round ${c().round}`} style={{ "list-style": "none", margin: 0, padding: 0, display: "flex", gap: "6px", "align-items": "center", "pointer-events": "auto", "flex-wrap": "wrap", "justify-content": "center" }}>
                <li class="chip" style={{ "box-shadow": "var(--shadow-float)" }}><span class="label">Round</span><span class="mono" style={{ "font-weight": 700 }}>{c().round}</span></li>
                <For each={c().order.filter((o) => !o.hidden)}>
                  {(o) => {
                    const a = () => actorById(o.actor_id);
                    const cur = () => currentTurnId() === o.actor_id;
                    return (
                      <li class="chip" aria-current={cur() ? "true" : undefined} style={{ border: cur() ? "2px solid var(--brass)" : undefined, "box-shadow": "var(--shadow-float)", color: (a()?.hp ?? 1) <= 0 ? "var(--ink-muted)" : undefined }}>
                        <span class="mono" style={{ color: cur() ? "var(--brass)" : "var(--ink-muted)", "font-weight": 700 }}>{o.initiative}</span>
                        <Show when={(a()?.hp ?? 1) > 0} fallback={<><s>{a()?.name}</s><span class="sr-only"> (down)</span></>}>{a()?.name}{a()?.kind === "pc" && cur() ? " · Your Turn" : ""}</Show>
                      </li>
                    );
                  }}
                </For>
                <Show when={c().order.some((o) => o.hidden)}><li class="chip" style={{ "border-style": "dashed", color: "var(--ink-muted)", "box-shadow": "var(--shadow-float)" }}>+{c().order.filter((o) => o.hidden).length} Hidden</li></Show>
              </ol>
            </div>
          )}
        </Show>

        <p class="panel small" style={{ position: "absolute", left: "12px", bottom: "112px", padding: "6px 12px", display: "flex", gap: "6px", "align-items": "center", "z-index": 4, color: "var(--ink-muted)", "flex-wrap": "wrap", "max-width": `calc(100% - ${right()} - 24px)` }}>
          <span style={{ color: "var(--ink)", "font-weight": 700 }}>Move:</span> select your token, then <kbd class="k">←</kbd><kbd class="k">↑</kbd><kbd class="k">→</kbd><kbd class="k">↓</kbd> and <kbd class="k">Enter</kbd>, drag it, or click a square · <kbd class="k">1</kbd>–<kbd class="k">8</kbd> hotbar
        </p>

        <Show when={pc()}>
          {(a) => (
            <div class="panel" role="region" aria-label="Hotbar" style={{ position: "absolute", left: "12px", right: right(), bottom: "12px", height: "88px", display: "flex", "align-items": "center", gap: "12px", padding: "0 16px", "z-index": 5 }}>
              <button type="button" class="art" aria-label={`Open Character Sheet for ${a().name}`} title="Character Sheet" onClick={() => setDrawer(true)} style={{ width: "56px", height: "56px", flex: "0 0 56px", "border-radius": "9999px", border: 0, "box-shadow": "0 0 0 3px var(--sky)", cursor: "pointer", overflow: "hidden", padding: 0 }}>
                <Show when={imageUrl(world()?.id ?? "", a().portrait)} fallback={<span aria-hidden="true">{initials(a().name)}</span>}>{(u) => <img src={u()} alt="" width={56} height={56} style={{ width: "100%", height: "100%", "object-fit": "cover" }} />}</Show>
              </button>
              <div style={{ flex: "0 1 150px", "min-width": 0, display: "flex", "flex-direction": "column", gap: "4px" }}>
                <span class="truncate" style={{ "font-weight": 700 }} translate="no">{a().name}</span>
                <div role="meter" aria-label="Hit points" aria-valuemin={0} aria-valuemax={a().max_hp} aria-valuenow={a().hp} style={{ height: "6px", "border-radius": "9999px", background: "var(--bg-300)", overflow: "hidden" }}><div style={{ width: `${(100 * a().hp) / Math.max(1, a().max_hp)}%`, height: "100%", background: a().hp / a().max_hp > 0.5 ? "var(--vital)" : "var(--wound)" }} /></div>
                <span class="small"><span class="mono" style={{ "font-weight": 700 }}>{a().hp}</span><span class="mono muted"> / {a().max_hp} HP</span></span>
              </div>
              <dl style={{ margin: 0, display: "flex", "flex-direction": "column", gap: "2px", flex: "0 0 auto" }}>
                <div style={{ display: "flex", gap: "6px", "align-items": "baseline" }}><dt class="label">AC</dt><dd class="mono" style={{ margin: 0, "font-weight": 700 }}>{a().ac}</dd></div>
                <div style={{ display: "flex", gap: "6px", "align-items": "center" }}><dt class="label">Fate</dt><dd style={{ margin: 0, display: "flex", gap: "4px" }} aria-label={`${a().fate} of 3`}><For each={[0, 1, 2]}>{(i) => <span style={{ width: "10px", height: "10px", "border-radius": "9999px", background: i < a().fate ? "var(--crit)" : "transparent", border: "1.5px solid var(--crit)" }} />}</For></dd></div>
              </dl>
              <span aria-hidden="true" style={{ width: "1px", height: "52px", background: "var(--line)" }} />
              <div role="toolbar" aria-label="Quick actions, keys 1 to 8" style={{ flex: 1, "min-width": 0, display: "flex", gap: "4px", overflow: "hidden" }}>
                <For each={slots()}>{(s) => <button type="button" class="slot" aria-label={`${s.aria}, key ${s.key}`} aria-pressed={s.pressed ? "true" : undefined} onClick={s.run} title={s.aria}><kbd>{s.key}</kbd><strong class="truncate" style={{ "max-width": "100%" }}>{s.label}</strong><span class="mono muted">{s.sub}</span></button>}</For>
              </div>
              <div style={{ display: "flex", "flex-direction": "column", gap: "4px", "align-items": "flex-end", flex: "0 0 auto" }}>
                <Show when={world()?.combat} fallback={<span class="small muted">Init {signed(mod(a().abilities.dex))} · {a().speed}&nbsp;ft</span>}>
                  <span class="small muted">{myTurn() ? "Your turn" : `${actorById(currentTurnId() ?? "")?.name ?? "…"}’s turn`}</span>
                </Show>
                <Show when={world()?.combat} fallback={<button type="button" class="btn btn-primary" disabled={busy()} onClick={() => act("send_message", { text: "" })}>Continue Story</button>}>
                  <button type="button" class="btn btn-primary" disabled={!myTurn()} onClick={async () => { await act("end_turn"); await act("send_message", { text: "" }); }}>End Turn</button>
                </Show>
              </div>
            </div>
          )}
        </Show>

        <Sidebar tab={tab()} setTab={setTab} expanded={expanded()} setExpanded={setExpanded} />
        <Show when={drawer()}><Drawer onClose={() => setDrawer(false)} /></Show>
      </div>
    </Show>
  );
}
