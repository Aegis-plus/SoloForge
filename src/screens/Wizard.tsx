import { createMemo, createResource, createSignal, For, Show } from "solid-js";
import { call, errText } from "../lib/ipc";
import { ABILITIES, ABILITY_NAMES, ANCESTRY_BONUS, mod, POINT_COST, signed } from "../lib/format";
import { setScreen, setWorld, showToast } from "../lib/store";
import type { Abilities, Ability, World } from "../lib/types";

interface ClassInfo { name: string; hit_die: number; blurb: string; solo: string }
const ANCESTRIES: [string, string][] = [["Human", "+1 to every ability"], ["Wood Elf", "+2 DEX, +1 WIS · darkvision · 35 ft speed"], ["Dwarf", "+2 CON, +1 WIS · poison resistance"], ["Halfling", "+2 DEX, +1 CHA · reroll natural 1s"], ["Half-Orc", "+2 STR, +1 CON · relentless"], ["Custom", "Describe it; the GM balances it"]];
const TONES = ["Heroic", "Gritty", "Horror", "Whimsical", "Mystery", "Swashbuckling"];
const HINT: Record<Ability, string> = { str: "Melee, carrying", dex: "Bows, stealth, AC", con: "Hit points", int: "Lore, arcana", wis: "Perception", cha: "Presence" };

export function Wizard() {
  const [classes] = createResource(() => call<ClassInfo[]>("class_list"));
  const [step, setStep] = createSignal(1);
  const [name, setName] = createSignal("");
  const [tried, setTried] = createSignal(false);
  const [pitch, setPitch] = createSignal("");
  const [tone, setTone] = createSignal<string[]>(["Gritty"]);
  const [dial, setDial] = createSignal("balanced");
  const [ancestry, setAncestry] = createSignal("Human");
  const [cls, setCls] = createSignal("Fighter");
  const [scores, setScores] = createSignal<Abilities>({ str: 14, dex: 13, con: 14, int: 10, wis: 12, cha: 10 });
  const [hero, setHero] = createSignal("");
  const [heroTried, setHeroTried] = createSignal(false);
  const [look, setLook] = createSignal("");
  const [hook, setHook] = createSignal("");
  const [companion, setCompanion] = createSignal(false);
  const [creating, setCreating] = createSignal(false);
  let nameInput!: HTMLInputElement;
  let heroInput!: HTMLInputElement;

  const spent = createMemo(() => ABILITIES.reduce((n, k) => n + (POINT_COST[scores()[k]] ?? 99), 0));
  const total = (k: Ability) => scores()[k] + (ANCESTRY_BONUS[ancestry()]?.[k] ?? 0);
  const info = () => classes()?.find((c) => c.name === cls());
  const hp = () => (info()?.hit_die ?? 10) + mod(total("con")) + Math.max(0, mod(total("con")) * 2);
  const nameOk = () => name().trim().length > 0;
  const LABELS = ["World", "Ancestry", "Class", "Abilities", "Look & Story"];
  const NEXT = ["Next: Ancestry", "Next: Class", "Next: Abilities", "Next: Look & Story"];

  function go(n: number) {
    if (step() === 1 && n > 1 && !nameOk()) { setTried(true); queueMicrotask(() => nameInput?.focus()); return; }
    if (step() === 4 && n > 4 && spent() > 27) { showToast(`That spends ${spent()} points; the budget is 27. Lower a score first.`, undefined, 6000); return; }
    setStep(n);
  }
  function bump(k: Ability, d: number) {
    const v = scores()[k] + d;
    if (v < 8 || v > 15) return;
    const next = { ...scores(), [k]: v };
    if (ABILITIES.reduce((n, a) => n + POINT_COST[next[a]], 0) > 27) return;
    setScores(next);
  }
  async function create() {
    if (!hero().trim()) { setHeroTried(true); queueMicrotask(() => heroInput?.focus()); return; }
    setCreating(true);
    try {
      const w = await call<World>("create_world", { spec: { name: name().trim(), pitch: pitch().trim(), tone: tone(), rules_dial: dial(), companion: companion(), hero: { name: hero().trim(), ancestry: ancestry(), class: cls(), scores: scores(), appearance: look().trim(), background: hook().trim() } } });
      setWorld(w);
      setScreen("table");
      await call("send_message", { text: "" });
    } catch (e) { showToast(errText(e), undefined, 8000); }
    finally { setCreating(false); }
  }

  return (
    <div class="page">
      <a class="skip btn btn-primary" href="#step-body">Skip to Step</a>
      <header class="page-head">
        <button type="button" class="serif" translate="no" onClick={() => setScreen("launcher")} style={{ "font-size": "22px", "font-weight": 700, color: "var(--brass)", background: "transparent", border: 0, padding: 0 }}>SoloForge</button>
        <span class="muted" aria-hidden="true">›</span><span>New World</span>
        <span style={{ flex: 1 }} />
        <button type="button" class="btn btn-sm" onClick={() => setScreen("launcher")}>Cancel</button>
      </header>
      <div class="wrap">
        <nav aria-label="Steps" style={{ flex: "1 1 220px", "max-width": "240px" }}>
          <ol style={{ "list-style": "none", margin: 0, padding: 0, display: "flex", "flex-direction": "column", gap: "4px" }}>
            <For each={LABELS}>{(l, i) => <li><button type="button" class="step" aria-current={step() === i() + 1 ? "step" : undefined} onClick={() => go(i() + 1)}><span class="step-n">{step() > i() + 1 ? "✓" : i() + 1}</span>{l}</button></li>}</For>
          </ol>
        </nav>

        <main id="step-body" style={{ flex: "999 1 520px", "min-width": 0, display: "flex", "flex-direction": "column", gap: "24px" }}>
          <Show when={step() === 1}>
            <h1 class="display">Pitch Your World</h1>
            <p class="muted" style={{ "max-width": "60ch" }}>The GM reads this before session zero. Say what you want to feel, not just the genre.</p>
            <div style={{ display: "flex", "flex-direction": "column", gap: "6px", "max-width": "560px" }}>
              <label for="wname" class="label">World Name</label>
              <input id="wname" ref={nameInput} name="world-name" class="field" autocomplete="off" placeholder="e.g. The Drowned Marches…" value={name()} onInput={(e) => setName(e.currentTarget.value)} aria-invalid={tried() && !nameOk()} aria-describedby={tried() && !nameOk() ? "wname-err" : undefined} />
              <Show when={tried() && !nameOk()}><p id="wname-err" role="alert" class="small" style={{ color: "var(--wound)" }}>Give your world a name to continue, e.g. “The Drowned Marches”.</p></Show>
            </div>
            <div style={{ display: "flex", "flex-direction": "column", gap: "6px", "max-width": "560px" }}>
              <label for="pitch" class="label">Pitch for the GM</label>
              <textarea id="pitch" name="pitch" class="field" rows="4" autocomplete="off" style={{ resize: "vertical" }} placeholder="e.g. grim low-magic mystery in a plague-struck river city…" value={pitch()} onInput={(e) => setPitch(e.currentTarget.value)} />
            </div>
            <fieldset style={{ border: 0, padding: 0, margin: 0 }}><legend class="label" style={{ "margin-bottom": "8px" }}>Tone</legend>
              <div style={{ display: "flex", "flex-wrap": "wrap", gap: "6px" }}><For each={TONES}>{(t) => <button type="button" class="chip" aria-pressed={tone().includes(t)} onClick={() => setTone(tone().includes(t) ? tone().filter((x) => x !== t) : [...tone(), t])}>{t}</button>}</For></div>
            </fieldset>
            <fieldset style={{ border: 0, padding: 0, margin: 0, display: "flex", "flex-direction": "column", gap: "8px" }}><legend class="label" style={{ "margin-bottom": "8px" }}>Rules Dial</legend>
              <div class="group" style={{ display: "grid", "grid-template-columns": "repeat(3, minmax(0, 1fr))", "max-width": "460px", border: "1px solid var(--line-strong)", "border-radius": "6px", overflow: "hidden" }}>
                <For each={[["story", "Story First"], ["balanced", "Balanced"], ["tactical", "Tactical"]] as const}>{([k, l]) => <button type="button" class="seg" aria-pressed={dial() === k} onClick={() => setDial(k)}>{l}</button>}</For>
              </div>
              <p class="small muted">How often the GM calls for rolls and grid combat.</p>
            </fieldset>
          </Show>

          <Show when={step() === 2}>
            <h1 class="display">Choose an Ancestry</h1>
            <div style={{ display: "grid", "grid-template-columns": "repeat(auto-fill, minmax(200px, 1fr))", gap: "12px" }}>
              <For each={ANCESTRIES}>{([a, d]) => <button type="button" class="opt" aria-pressed={ancestry() === a} onClick={() => setAncestry(a)}><strong>{a}</strong><span class="small muted">{d}</span></button>}</For>
            </div>
          </Show>

          <Show when={step() === 3}>
            <h1 class="display">Choose a Class</h1>
            <p class="muted">Every class is tuned for one hero. You can rename and reflavour it later.</p>
            <div style={{ display: "grid", "grid-template-columns": "repeat(auto-fill, minmax(200px, 1fr))", gap: "12px" }}>
              <For each={classes() ?? []}>{(c) => <button type="button" class="opt" aria-pressed={cls() === c.name} onClick={() => setCls(c.name)}><span style={{ display: "flex", "justify-content": "space-between", gap: "8px" }}><strong>{c.name}</strong><span class="mono small muted">d{c.hit_die}</span></span><span class="small muted">{c.blurb}</span></button>}</For>
            </div>
            <Show when={info()}><div class="card"><h2 class="label">Solo Feature</h2><p style={{ "margin-top": "4px" }}>{info()!.solo}</p></div></Show>
          </Show>

          <Show when={step() === 4}>
            <div style={{ display: "flex", "justify-content": "space-between", "align-items": "flex-end", "flex-wrap": "wrap", gap: "12px" }}>
              <h1 class="display">Set Your Abilities</h1>
              <p aria-live="polite"><span class="label">Points Left </span><span class="mono" style={{ "font-size": "26px", "font-weight": 700, color: 27 - spent() === 0 ? "var(--vital)" : undefined }}>{27 - spent()}</span><span class="muted"> / 27</span></p>
            </div>
            <div style={{ display: "grid", "grid-template-columns": "repeat(auto-fill, minmax(200px, 1fr))", gap: "12px" }}>
              <For each={ABILITIES}>{(k) => (
                <div class="card" role="group" aria-label={ABILITY_NAMES[k]} style={{ display: "flex", "flex-direction": "column", gap: "8px" }}>
                  <div style={{ display: "flex", "justify-content": "space-between", gap: "8px" }}><span class="label">{k.toUpperCase()}</span><span class="small muted truncate">{HINT[k]}</span></div>
                  <div style={{ display: "flex", "align-items": "center", gap: "10px" }}>
                    <button type="button" class="btn" aria-label={`Lower ${ABILITY_NAMES[k]}`} onClick={() => bump(k, -1)} disabled={scores()[k] <= 8} style={{ width: "40px", padding: 0 }}>−</button>
                    <div style={{ flex: 1, "text-align": "center" }}><div class="mono" style={{ "font-size": "26px", "line-height": "30px", "font-weight": 700 }}>{signed(mod(total(k)))}</div><div class="mono small muted">{scores()[k]}{total(k) !== scores()[k] ? ` + ${total(k) - scores()[k]} = ${total(k)}` : ""}</div></div>
                    <button type="button" class="btn" aria-label={`Raise ${ABILITY_NAMES[k]}`} onClick={() => bump(k, 1)} disabled={scores()[k] >= 15} style={{ width: "40px", padding: 0 }}>+</button>
                  </div>
                </div>
              )}</For>
            </div>
            <p class="small muted">Scores run 8–15 before ancestry bonuses.</p>
          </Show>

          <Show when={step() === 5}>
            <h1 class="display">Look &amp; Story</h1>
            <div style={{ display: "flex", "flex-direction": "column", gap: "6px", "max-width": "560px" }}>
              <label for="hname" class="label">Hero Name</label>
              <input id="hname" ref={heroInput} name="hero-name" class="field" autocomplete="off" spellcheck={false} placeholder="e.g. Aria Thorne…" value={hero()} onInput={(e) => setHero(e.currentTarget.value)} aria-invalid={heroTried() && !hero().trim()} aria-describedby={heroTried() && !hero().trim() ? "hname-err" : undefined} />
              <Show when={heroTried() && !hero().trim()}><p id="hname-err" role="alert" class="small" style={{ color: "var(--wound)" }}>Give your hero a name to begin, e.g. “Aria Thorne”.</p></Show>
            </div>
            <div style={{ display: "flex", "flex-direction": "column", gap: "6px" }}><label for="look" class="label">Appearance (used for the portrait and scene art)</label><textarea id="look" name="appearance" class="field" rows="3" autocomplete="off" style={{ resize: "vertical" }} placeholder="e.g. lean wood elf in a moss-green hooded cloak…" value={look()} onInput={(e) => setLook(e.currentTarget.value)} /></div>
            <div style={{ display: "flex", "flex-direction": "column", gap: "6px" }}><label for="hook" class="label">Hook for the GM</label><textarea id="hook" name="hook" class="field" rows="3" autocomplete="off" style={{ resize: "vertical" }} placeholder="e.g. she hunts whoever started the fire…" value={hook()} onInput={(e) => setHook(e.currentTarget.value)} /></div>
            <label class="check"><input type="checkbox" name="companion" checked={companion()} onChange={(e) => setCompanion(e.currentTarget.checked)} />Add a GM-run companion</label>
          </Show>

          <div style={{ display: "flex", gap: "8px", "margin-top": "auto", "padding-top": "16px", "border-top": "1px solid var(--line)" }}>
            <button type="button" class="btn" onClick={() => (step() === 1 ? setScreen("launcher") : setStep(step() - 1))}>Back</button>
            <span style={{ flex: 1 }} />
            <Show when={step() < 5} fallback={<button type="button" class="btn btn-primary" onClick={create} disabled={creating()}>{creating() ? "Creating World…" : "Begin Session Zero"}</button>}>
              <button type="button" class="btn btn-primary" onClick={() => go(step() + 1)}>{NEXT[step() - 1]}</button>
            </Show>
          </div>
        </main>

        <aside aria-label="Hero preview" style={{ flex: "1 1 280px", "max-width": "320px" }}>
          <div class="panel" style={{ padding: "20px", display: "flex", "flex-direction": "column", gap: "14px", "align-items": "center", "text-align": "center" }}>
            <div class="art" aria-hidden="true" style={{ width: "152px", height: "152px", "border-radius": "9999px", "box-shadow": "0 0 0 3px var(--sky)" }}>Portrait after creation</div>
            <div><p class="title truncate" translate="no">{hero().trim() || "Your Hero"}</p><p class="small muted">Level 1 {cls()} · {ancestry()}</p></div>
            <dl style={{ margin: 0, display: "grid", "grid-template-columns": "repeat(3, minmax(0, 1fr))", gap: "8px", width: "100%" }}>
              <div class="card" style={{ padding: "8px" }}><dt class="label">HP</dt><dd class="mono" style={{ margin: 0, "font-weight": 700 }}>{hp()}</dd></div>
              <div class="card" style={{ padding: "8px" }}><dt class="label">DEX</dt><dd class="mono" style={{ margin: 0, "font-weight": 700 }}>{signed(mod(total("dex")))}</dd></div>
              <div class="card" style={{ padding: "8px" }}><dt class="label">Fate</dt><dd class="mono" style={{ margin: 0, "font-weight": 700, color: "var(--crit)" }}>3</dd></div>
            </dl>
            <p class="small muted">Solo hero bonus: max HP + CON × 2, and 3 Fate per long rest. Generate a portrait from the character sheet once you start.</p>
          </div>
        </aside>
      </div>
    </div>
  );
}
