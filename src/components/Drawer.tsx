import { createSignal, For, onCleanup, onMount, Show } from "solid-js";
import { act, imageUrl, pc, world } from "./_shared";
import { ABILITIES, ABILITY_NAMES, initials, mod, num, prof, saveBonus, signed, skillBonus, SKILLS, XP_TABLE } from "../lib/format";
import { Icon } from "./Icon";

type DTab = "core" | "skills" | "inventory" | "spells" | "notes";

export function Drawer(props: { onClose: () => void }) {
  const [tab, setTab] = createSignal<DTab>("core");
  let closeBtn!: HTMLButtonElement;
  let notesTimer: ReturnType<typeof setTimeout> | undefined;
  const key = (e: KeyboardEvent) => { if (e.key === "Escape") props.onClose(); };
  onMount(() => { closeBtn?.focus(); window.addEventListener("keydown", key); });
  onCleanup(() => window.removeEventListener("keydown", key));
  const tabs: [DTab, string][] = [["core", "Core"], ["skills", "Skills"], ["inventory", "Inventory"], ["spells", "Spells"], ["notes", "Notes"]];
  const roll = (kind: string) => act("check", { kind, adv: "" });

  return (
    <Show when={pc()}>
      {(a) => {
        const nextXp = () => XP_TABLE[a().level] ?? a().xp;
        return (
          <>
            <button type="button" class="scrim-btn" aria-label="Close Character Sheet" style={{ position: "fixed", "z-index": 20 }} onClick={props.onClose} />
            <section role="dialog" aria-modal="true" aria-labelledby="drawer-title" class="panel" style={{ position: "fixed", left: "12px", top: "12px", bottom: "12px", width: "440px", "max-width": "calc(100vw - 24px)", display: "flex", "flex-direction": "column", overflow: "hidden", "z-index": 21 }}>
              <header style={{ display: "flex", gap: "14px", "align-items": "center", padding: "16px 16px 12px" }}>
                <div class="art" style={{ width: "72px", height: "72px", flex: "0 0 72px", "border-radius": "9999px", "box-shadow": "0 0 0 3px var(--sky)", overflow: "hidden" }}>
                  <Show when={imageUrl(world()?.id ?? "", a().portrait)} fallback={<span aria-hidden="true">{initials(a().name)}</span>}>{(u) => <img src={u()} alt={`Portrait of ${a().name}`} width={72} height={72} style={{ width: "100%", height: "100%", "object-fit": "cover" }} />}</Show>
                </div>
                <div style={{ flex: 1, "min-width": 0, display: "flex", "flex-direction": "column", gap: "2px" }}>
                  <h2 id="drawer-title" class="title truncate" translate="no">{a().name}</h2>
                  <span class="small muted truncate">Level {a().level} {a().class} · {a().ancestry}</span>
                </div>
                <button type="button" ref={closeBtn} class="icon-btn" aria-label="Close Character Sheet" title="Close (Esc)" onClick={props.onClose}><Icon name="close" /></button>
              </header>

              <div style={{ padding: "0 16px 12px", display: "flex", "flex-direction": "column", gap: "10px" }}>
                <div style={{ display: "flex", "align-items": "baseline", "justify-content": "space-between" }}>
                  <span class="label">Hit Points</span>
                  <span><span class="mono" style={{ "font-size": "26px", "line-height": "30px", "font-weight": 700 }}>{a().hp}</span><span class="mono muted"> / {a().max_hp}</span><span class="small muted"> · Temp {a().temp_hp}</span></span>
                </div>
                <div role="meter" aria-label="Hit points" aria-valuemin={0} aria-valuemax={a().max_hp} aria-valuenow={a().hp} style={{ height: "8px", "border-radius": "9999px", background: "var(--bg-300)", overflow: "hidden" }}><div style={{ width: `${(100 * a().hp) / Math.max(1, a().max_hp)}%`, height: "100%", background: a().hp / a().max_hp > 0.5 ? "var(--vital)" : "var(--wound)" }} /></div>
                <dl style={{ margin: 0, display: "grid", "grid-template-columns": "repeat(5, minmax(0, 1fr))", gap: "8px", "text-align": "center" }}>
                  <div class="card" style={{ padding: "6px" }}><dt class="label">AC</dt><dd class="mono" style={{ margin: 0, "font-weight": 700 }}>{a().ac}</dd></div>
                  <div class="card" style={{ padding: "6px" }}><dt class="label">Speed</dt><dd class="mono" style={{ margin: 0, "font-weight": 700 }}>{a().speed}&nbsp;ft</dd></div>
                  <div class="card" style={{ padding: "6px" }}><dt class="label">Init</dt><dd class="mono" style={{ margin: 0, "font-weight": 700 }}>{signed(mod(a().abilities.dex))}</dd></div>
                  <div class="card" style={{ padding: "6px" }}><dt class="label">Prof</dt><dd class="mono" style={{ margin: 0, "font-weight": 700 }}>{signed(prof(a().level))}</dd></div>
                  <div class="card" style={{ padding: "6px" }}><dt class="label">Fate</dt><dd class="mono" style={{ margin: 0, "font-weight": 700, color: "var(--crit)" }}>{a().fate}/3</dd></div>
                </dl>
              </div>

              <div role="tablist" aria-label="Character sheet sections" style={{ display: "flex", gap: "2px", padding: "0 12px", "border-bottom": "1px solid var(--line)" }}>
                <For each={tabs}>{([id, l]) => <button role="tab" type="button" class="tab" aria-selected={tab() === id} onClick={() => setTab(id)} style={{ flex: "0 1 auto", padding: "0 10px" }}>{l}</button>}</For>
              </div>

              <div class="scroll" role="tabpanel" aria-label={tabs.find((t) => t[0] === tab())?.[1]} style={{ flex: 1, "min-height": 0, padding: "16px", display: "flex", "flex-direction": "column", gap: "16px" }}>
                <Show when={tab() === "core"}>
                  <div style={{ display: "flex", "flex-direction": "column", gap: "8px" }}>
                    <h3 class="label">Abilities · Select to Roll a Check</h3>
                    <div style={{ display: "grid", "grid-template-columns": "repeat(3, minmax(0, 1fr))", gap: "8px" }}>
                      <For each={ABILITIES}>{(k) => (
                        <button type="button" class="card" aria-label={`Roll ${ABILITY_NAMES[k]} check, ${signed(mod(a().abilities[k]))}`} onClick={() => roll(k)} style={{ display: "flex", "flex-direction": "column", "align-items": "center", gap: "2px", cursor: "pointer" }}>
                          <span class="label">{k.toUpperCase()}</span>
                          <span class="mono" style={{ "font-size": "22px", "line-height": "26px", "font-weight": 700 }}>{signed(mod(a().abilities[k]))}</span>
                          <span class="mono small muted">{a().abilities[k]} · save {signed(saveBonus(a(), k))}</span>
                        </button>
                      )}</For>
                    </div>
                    <div style={{ display: "flex", gap: "6px", "flex-wrap": "wrap" }}><For each={ABILITIES}>{(k) => <button type="button" class="chip" onClick={() => roll(`${k} save`)}>{k.toUpperCase()} Save {signed(saveBonus(a(), k))}</button>}</For></div>
                  </div>
                  <div style={{ display: "flex", "flex-direction": "column", gap: "8px" }}>
                    <h3 class="label">Attacks</h3>
                    <For each={a().attacks}>{(atk) => (
                      <div class="row card" style={{ "justify-content": "space-between" }}>
                        <span style={{ "min-width": 0 }}><strong>{atk.name}</strong><br /><span class="small muted">{atk.range.replace(/ ft\b/g, "\u00a0ft")} · {atk.damage_type}</span></span>
                        <span class="mono" style={{ "text-align": "right" }}>{signed(atk.to_hit)}<br /><span class="small muted">{atk.damage}</span></span>
                      </div>
                    )}</For>
                    <p class="small muted">To attack, select a target token on the map.</p>
                  </div>
                  <div style={{ display: "flex", "flex-direction": "column", gap: "8px" }}>
                    <h3 class="label">Conditions</h3>
                    <Show when={a().conditions.length} fallback={<p class="small muted">No conditions. The GM adds them here when something affects you.</p>}>
                      <div style={{ display: "flex", gap: "6px", "flex-wrap": "wrap" }}><For each={a().conditions}>{(c) => <span class="chip" style={{ "border-color": "var(--wound)" }}>{c}</span>}</For></div>
                    </Show>
                  </div>
                </Show>

                <Show when={tab() === "skills"}>
                  <h3 class="label">Skills · ● Proficient · Select to Roll</h3>
                  <ul style={{ "list-style": "none", margin: 0, padding: 0, display: "grid", "grid-template-columns": "repeat(2, minmax(0, 1fr))", gap: "2px 8px" }}>
                    <For each={SKILLS}>{([k, name, ab]) => (
                      <li><button type="button" class="row" style={{ "justify-content": "space-between" }} onClick={() => roll(k)} aria-label={`Roll ${name}, ${signed(skillBonus(a(), k, ab))}`}>
                        <span class="truncate" style={{ color: a().skill_profs.includes(k) ? undefined : "var(--ink-muted)" }}>{a().skill_profs.includes(k) ? "●" : "○"} {name}</span>
                        <span class="mono">{signed(skillBonus(a(), k, ab))}</span>
                      </button></li>
                    )}</For>
                  </ul>
                </Show>

                <Show when={tab() === "inventory"}>
                  <h3 class="label">Inventory</h3>
                  <Show when={a().inventory.length} fallback={<p class="small muted">Your pack is empty.</p>}>
                    <ul style={{ "list-style": "none", margin: 0, padding: 0, display: "flex", "flex-direction": "column", gap: "2px" }}>
                      <For each={a().inventory}>{(i) => <li class="row" style={{ "justify-content": "space-between" }}><span class="truncate">{i.name}{i.note ? <span class="small muted"> · {i.note}</span> : null}</span><span class="mono small muted">{i.qty > 1 ? `×${i.qty}` : "1"}</span></li>}</For>
                    </ul>
                  </Show>
                  <div class="card" style={{ display: "flex", gap: "16px", "justify-content": "center" }}><span><span class="mono" style={{ color: "var(--crit)", "font-weight": 700 }}>{num(a().gold)}</span>&nbsp;gp</span></div>
                </Show>

                <Show when={tab() === "spells"}>
                  <div style={{ display: "flex", "justify-content": "space-between", "align-items": "baseline" }}><h3 class="label">Spells</h3><Show when={a().spell_slots_max.length}><span class="small muted">Level 1 slots <span class="mono">{a().spell_slots[0] ?? 0} / {a().spell_slots_max[0]}</span></span></Show></div>
                  <Show when={a().spells.length} fallback={<p class="small muted">{a().class} has no spells at level {a().level}.</p>}>
                    <For each={a().spells}>{(s) => (
                      <div class="card" style={{ display: "flex", "flex-direction": "column", gap: "4px" }}>
                        <div style={{ display: "flex", "justify-content": "space-between", gap: "8px" }}><strong>{s.name}</strong><Show when={a().concentrating === s.name}><span class="chip" style={{ "border-color": "var(--sky)" }}>Concentrating</span></Show></div>
                        <span class="small muted">{s.summary}</span>
                        <button type="button" class="btn btn-sm" style={{ "align-self": "flex-start" }} onClick={() => act("send_message", { text: `I cast ${s.name}.` })}>Cast {s.name}</button>
                      </div>
                    )}</For>
                  </Show>
                </Show>

                <Show when={tab() === "notes"}>
                  <label for="hero-notes" class="label">Your Notes</label>
                  <textarea id="hero-notes" name="notes" class="field" rows="8" autocomplete="off" placeholder="Names, clues, grudges…" style={{ resize: "vertical" }} value={a().notes}
                    onInput={(e) => { const v = e.currentTarget.value; clearTimeout(notesTimer); notesTimer = setTimeout(() => act("update_notes", { notes: v }), 600); }} />
                  <Show when={a().background}><h3 class="label">Background</h3><p class="narration serif">{a().background}</p></Show>
                </Show>
              </div>

              <footer style={{ padding: "12px 16px 16px", "border-top": "1px solid var(--line)", display: "flex", "flex-direction": "column", gap: "10px" }}>
                <div style={{ display: "flex", "justify-content": "space-between" }}><span class="label">XP</span><span class="mono small muted">{num(a().xp)} / {num(nextXp())}</span></div>
                <div role="meter" aria-label={`Experience toward level ${a().level + 1}`} aria-valuemin={0} aria-valuemax={nextXp()} aria-valuenow={a().xp} style={{ height: "6px", "border-radius": "9999px", background: "var(--bg-300)", overflow: "hidden" }}><div style={{ width: `${Math.min(100, (100 * a().xp) / Math.max(1, nextXp()))}%`, height: "100%", background: "var(--crit)" }} /></div>
                <div style={{ display: "flex", gap: "8px" }}>
                  <button type="button" class="btn" style={{ flex: 1 }} onClick={() => act("rest", { long: false })}>Short Rest</button>
                  <button type="button" class="btn" style={{ flex: 1 }} onClick={() => act("rest", { long: true })}>Long Rest</button>
                </div>
              </footer>
            </section>
          </>
        );
      }}
    </Show>
  );
}
