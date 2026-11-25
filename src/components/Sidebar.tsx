import { createMemo, createResource, createSignal, For, Show } from "solid-js";
import { call } from "../lib/ipc";
import type { Compendium as Comp, JournalEntry, MonsterEntry } from "../lib/types";
import { act, gm, openSettings, world } from "./_shared";
import { Chat } from "./Chat";
import { Icon } from "./Icon";
import { mod, signed } from "../lib/format";

export type Tab = "chat" | "journal" | "comp";

function Journal() {
  const [filter, setFilter] = createSignal<string>("all");
  const [openId, setOpenId] = createSignal<string | null>(null);
  const entries = () => (world()?.journal ?? []).slice().sort((a, b) => Number(b.active) - Number(a.active) || b.updated_at - a.updated_at);
  const shown = () => entries().filter((j) => filter() === "all" || j.category === filter());
  const current = (): JournalEntry | undefined => entries().find((j) => j.id === openId()) ?? shown()[0];
  const count = (c: string) => entries().filter((j) => j.category === c).length;
  const cats: [string, string][] = [["all", "All"], ["quest", "Quests"], ["npc", "NPCs"], ["lore", "Lore"], ["handout", "Handouts"], ["note", "Notes"]];
  const label = (c: string) => ({ quest: "Quest", npc: "NPC", lore: "Lore", handout: "Handout", note: "Your Note" } as Record<string, string>)[c] ?? c;
  const [noteTitle, setNoteTitle] = createSignal("");
  const [noteBody, setNoteBody] = createSignal("");
  const [adding, setAdding] = createSignal(false);
  return (
    <div class="scroll" style={{ flex: 1, "min-height": 0, padding: "16px", display: "flex", "flex-direction": "column", gap: "14px" }}>
      <div role="group" aria-label="Filter entries" style={{ display: "flex", "flex-wrap": "wrap", gap: "6px" }}>
        <For each={cats.filter(([c]) => c === "all" || count(c) > 0)}>{([c, l]) => <button type="button" class="chip" aria-pressed={filter() === c} onClick={() => setFilter(c)}>{l} {c === "all" ? entries().length : count(c)}</button>}</For>
        <button type="button" class="chip" onClick={() => setAdding(!adding())}>Add a Note</button>
      </div>
      <Show when={adding()}>
        <form class="card" style={{ display: "flex", "flex-direction": "column", gap: "8px" }} onSubmit={async (e) => { e.preventDefault(); if (!noteTitle().trim()) return; await act("add_note", { title: noteTitle(), body: noteBody() }); setNoteTitle(""); setNoteBody(""); setAdding(false); }}>
          <label for="note-t" class="label">Note Title</label>
          <input id="note-t" name="note-title" class="field" autocomplete="off" value={noteTitle()} onInput={(e) => setNoteTitle(e.currentTarget.value)} placeholder="e.g. Who lit the lantern?…" />
          <label for="note-b" class="label">Note</label>
          <textarea id="note-b" name="note-body" class="field" rows="3" autocomplete="off" value={noteBody()} onInput={(e) => setNoteBody(e.currentTarget.value)} />
          <button type="submit" class="btn btn-sm btn-primary" disabled={!noteTitle().trim()}>Save Note</button>
        </form>
      </Show>
      <Show when={entries().length > 0} fallback={
        <div class="card" style={{ display: "flex", "flex-direction": "column", gap: "6px" }}><p style={{ "font-weight": 700 }}>Your Journal Is Empty</p><p class="small muted">The GM writes quests, NPCs and lore here as you play. You can add your own notes too.</p></div>
      }>
        <ul style={{ "list-style": "none", margin: 0, padding: 0, display: "flex", "flex-direction": "column", gap: "2px" }}>
          <For each={shown()}>
            {(j) => (
              <li><button type="button" class="row" aria-current={current()?.id === j.id} onClick={() => setOpenId(j.id)} style={{ "flex-direction": "column", "align-items": "flex-start", gap: 0 }}>
                <span class="label" style={{ color: j.active ? "var(--brass)" : undefined }}>{label(j.category)}{j.active ? " · Active" : ""}</span>
                <span class="truncate" style={{ "max-width": "100%" }}>{j.title}</span>
              </button></li>
            )}
          </For>
        </ul>
        <Show when={current()}>
          {(j) => (
            <article style={{ display: "flex", "flex-direction": "column", gap: "8px", "border-top": "1px solid var(--line)", "padding-top": "14px" }}>
              <h2 class="title">{j().title}</h2>
              <span class="small muted">{j().by_player ? "Your note" : "Written by the GM"}</span>
              <div class="md narration serif"><For each={j().body.split(/\n{2,}/)}>{(p) => <p>{p}</p>}</For></div>
              <Show when={j().objectives.length > 0}>
                <h3 class="label" style={{ "margin-top": "4px" }}>Objectives</h3>
                <ul style={{ "list-style": "none", margin: 0, padding: 0, display: "flex", "flex-direction": "column", gap: "4px" }}>
                  <For each={j().objectives}>{(o) => <li><span aria-hidden="true" style={{ color: o.done ? "var(--vital)" : "var(--ink-muted)" }}>{o.done ? "✓" : "○"}</span> {o.text}<Show when={o.done}><span class="sr-only"> (done)</span></Show></li>}</For>
                </ul>
              </Show>
            </article>
          )}
        </Show>
      </Show>
    </div>
  );
}

function Compendium() {
  const [data] = createResource(() => call<Comp>("get_compendium"));
  const [q, setQ] = createSignal("");
  const [cat, setCat] = createSignal<"monsters" | "items" | "spells">("monsters");
  const [sel, setSel] = createSignal<string | null>(null);
  const results = createMemo(() => {
    const d = data(); if (!d) return [];
    const s = q().trim().toLowerCase();
    const list = d[cat()] as { id: string; name: string }[];
    return list.filter((x) => !s || x.name.toLowerCase().includes(s));
  });
  const monster = (): MonsterEntry | undefined => (cat() === "monsters" ? (data()?.monsters.find((m) => m.id === (sel() ?? results()[0]?.id))) : undefined);
  const AB = ["STR", "DEX", "CON", "INT", "WIS", "CHA"];
  return (
    <div class="scroll" style={{ flex: 1, "min-height": 0, padding: "16px", display: "flex", "flex-direction": "column", gap: "12px" }}>
      <div style={{ position: "relative" }}>
        <label for="comp-q" class="sr-only">Search the compendium</label>
        <input id="comp-q" name="compendium-search" type="search" class="field" autocomplete="off" spellcheck={false} placeholder="Search monsters, items, spells…" value={q()} onInput={(e) => { setQ(e.currentTarget.value); setSel(null); }} style={{ "padding-left": "36px" }} />
        <span style={{ position: "absolute", left: "12px", top: "11px", color: "var(--ink-muted)" }}><Icon name="search" size={16} /></span>
      </div>
      <div role="group" aria-label="Category" class="group" style={{ display: "grid", "grid-template-columns": "repeat(3, minmax(0, 1fr))", border: "1px solid var(--line-strong)", "border-radius": "6px", overflow: "hidden" }}>
        <For each={[["monsters", "Monsters"], ["items", "Items"], ["spells", "Spells"]] as const}>{([k, l]) => <button type="button" class="seg" aria-pressed={cat() === k} onClick={() => { setCat(k); setSel(null); }}>{l}</button>}</For>
      </div>
      <p class="small muted" aria-live="polite">{results().length} {results().length === 1 ? "result" : "results"}{q().trim() ? ` for “${q().trim()}”` : ""}</p>
      <Show when={results().length > 0} fallback={<div class="card"><p style={{ "font-weight": 700 }}>No Results for “{q().trim()}”</p><p class="small muted">Check the spelling, or ask the GM in chat to create a custom one.</p></div>}>
        <ul class="scroll" style={{ "list-style": "none", margin: 0, padding: 0, display: "flex", "flex-direction": "column", gap: "2px", "max-height": cat() === "monsters" ? "220px" : undefined, "flex-shrink": 0 }}>
          <For each={results()}>
            {(r) => {
              const m = r as MonsterEntry & { summary?: string; level?: number };
              return (
                <li><button type="button" class="row" aria-current={(sel() ?? results()[0]?.id) === r.id} onClick={() => setSel(r.id)} style={{ "justify-content": "space-between", "flex-wrap": "wrap" }}>
                  <strong class="truncate">{r.name}</strong>
                  <span class="mono small muted">{cat() === "monsters" ? `CR ${m.cr}${m.minion ? " · Minion" : ""}` : cat() === "spells" ? `Level ${m.level ?? 0}` : ""}</span>
                  <Show when={cat() !== "monsters"}><span class="small muted" style={{ width: "100%" }}>{m.summary}</span></Show>
                </button></li>
              );
            }}
          </For>
        </ul>
      </Show>
      <Show when={monster()}>
        {(m) => (
          <article class="card" style={{ display: "flex", "flex-direction": "column", gap: "8px" }}>
            <div style={{ display: "flex", "justify-content": "space-between", "align-items": "baseline", gap: "8px" }}><h2 class="title">{m().name}</h2><span class="small muted">{m().kind}</span></div>
            <div class="small" style={{ display: "flex", gap: "12px", "flex-wrap": "wrap" }}><span><strong>AC</strong> <span class="mono">{m().ac}</span></span><span><strong>HP</strong> <span class="mono">{m().hp} ({m().hp_dice})</span></span><span><strong>Speed</strong> <span class="mono">{m().speed}&nbsp;ft</span></span></div>
            <table class="small" style={{ width: "100%", "text-align": "center", "border-collapse": "collapse" }}>
              <thead><tr><For each={AB}>{(a) => <th class="label" scope="col">{a}</th>}</For></tr></thead>
              <tbody><tr class="mono"><For each={m().abilities}>{(s) => <td>{s} {signed(mod(s))}</td>}</For></tr></tbody>
            </table>
            <For each={m().traits}>{(t) => <p class="small">{t}</p>}</For>
            <For each={m().attacks}>{(a) => <p class="small"><strong>{a.name}.</strong> <span class="mono">{signed(a.to_hit)}</span> to hit, <span class="mono">{a.damage}</span> {a.damage_type}.</p>}</For>
            <Show when={m().minion}><p class="small muted"><strong>Solo rule · Minion:</strong> drops on any hit unless the GM marks it Elite.</p></Show>
            <button type="button" class="btn btn-sm" onClick={() => act("send_message", { text: `(Table request) Add a ${m().name} to this scene for me to fight.` })} disabled={gm().state !== "idle"}>Ask the GM to Add It</button>
          </article>
        )}
      </Show>
      <p class="small muted">Includes material from the SRD 5.1, CC-BY-4.0.</p>
    </div>
  );
}

export function Sidebar(props: { tab: Tab; setTab: (t: Tab) => void; expanded: boolean; setExpanded: (b: boolean) => void }) {
  const status = () => {
    const s = gm();
    if (s.state === "thinking") return { title: "GM Is Thinking…", detail: "Claude Code" };
    if (s.state === "acting") return { title: "GM Is Acting…", detail: `Claude Code · ${s.detail.replace(/_/g, " ")}` };
    if (s.state === "error") return { title: "GM Needs Attention", detail: "See the message in chat" };
    return { title: "GM Ready", detail: "Claude Code" };
  };
  const tabs: [Tab, string, string][] = [["chat", "Chat", "chat"], ["journal", "Journal", "journal"], ["comp", "Compendium", "search"]];
  return (
    <Show when={props.expanded} fallback={
      <nav class="panel" aria-label="Sidebar" style={{ position: "absolute", right: "12px", top: "12px", width: "56px", padding: "7px", display: "flex", "flex-direction": "column", gap: "4px", "z-index": 6 }}>
        <button type="button" class="icon-btn" aria-label="Expand Sidebar" aria-expanded="false" title="Expand (])" onClick={() => props.setExpanded(true)}><Icon name="expand" /></button>
        <For each={tabs}>{([t, l, i]) => <button type="button" class="icon-btn" aria-label={l} title={l} onClick={() => { props.setTab(t); props.setExpanded(true); }}><Icon name={i} /></button>}</For>
        <button type="button" class="icon-btn" aria-label="Settings" title="Settings" onClick={openSettings}><Icon name="sliders" /></button>
      </nav>
    }>
      <aside class="panel" aria-label="Sidebar" style={{ position: "absolute", right: "12px", top: "12px", bottom: "12px", width: "380px", display: "flex", "flex-direction": "column", overflow: "hidden", "z-index": 6 }}>
        <div style={{ display: "flex", "align-items": "center", gap: "8px", padding: "8px 8px 8px 16px", "border-bottom": "1px solid var(--line)" }}>
          <span class={gm().state === "idle" ? "" : "pulse"} aria-hidden="true" style={{ width: "10px", height: "10px", "border-radius": "9999px", background: gm().state === "idle" ? "var(--vital)" : gm().state === "error" ? "var(--wound)" : "var(--brass)" }} />
          <div style={{ flex: 1, "min-width": 0, display: "flex", "flex-direction": "column" }} aria-live="polite">
            <span style={{ "font-weight": 700 }}>{status().title}</span>
            <span class="small muted truncate" translate="no">{status().detail}</span>
          </div>
          <button type="button" class="icon-btn" aria-label="Settings" title="Settings" onClick={openSettings}><Icon name="sliders" /></button>
          <button type="button" class="icon-btn" aria-label="Collapse Sidebar" aria-expanded="true" title="Collapse ([)" onClick={() => props.setExpanded(false)}><Icon name="collapse" /></button>
        </div>
        <div role="tablist" aria-label="Sidebar sections" style={{ display: "flex", padding: "0 8px", "border-bottom": "1px solid var(--line)" }}>
          <For each={tabs}>{([t, l, i]) => <button role="tab" type="button" class="tab" aria-selected={props.tab === t} onClick={() => props.setTab(t)}><Icon name={i} size={16} />{l}</button>}</For>
        </div>
        <Show when={props.tab === "chat"}><Chat /></Show>
        <Show when={props.tab === "journal"}><Journal /></Show>
        <Show when={props.tab === "comp"}><Compendium /></Show>
      </aside>
    </Show>
  );
}
