import { createMemo, createSignal, For, Show } from "solid-js";
import { act, gm, imageUrl, pc, streaming, world } from "../components/_shared";
import { Icon } from "../components/Icon";
import { initials } from "../lib/format";
import { openSettings } from "../lib/store";
import type { ChatEntry } from "../lib/types";

export function Story() {
  const [pick, setPick] = createSignal<string | null>(null);
  const [own, setOwn] = createSignal("");
  const busy = () => gm().state === "thinking" || gm().state === "acting";
  const art = createMemo(() => [...(world()?.chat ?? [])].reverse().find((c): c is Extract<ChatEntry, { type: "image" }> => c.type === "image"));
  const narration = createMemo(() => {
    const chat = world()?.chat ?? [];
    const lastPlayer = chat.map((c) => c.type).lastIndexOf("player");
    const gmAfter = chat.slice(lastPlayer + 1).filter((c): c is Extract<ChatEntry, { type: "gm" }> => c.type === "gm");
    const src = gmAfter.length ? gmAfter : chat.filter((c): c is Extract<ChatEntry, { type: "gm" }> => c.type === "gm").slice(-1);
    return src.map((c) => c.text).join("\n\n");
  });
  const quest = () => world()?.journal.find((j) => j.active && j.category === "quest");
  async function send() {
    const text = own().trim() || pick() || "";
    setOwn(""); setPick(null);
    await act("send_message", { text });
  }
  return (
    <div class="page">
      <a class="skip btn btn-primary" href="#story-text">Skip to Story</a>
      <header class="page-head" style={{ padding: "12px 24px", "flex-wrap": "wrap" }}>
        <button type="button" class="btn btn-sm" onClick={() => act("set_story_mode", { on: false })}><Icon name="back" size={16} />Back to Map</button>
        <span class="serif" translate="no" style={{ "font-size": "20px", "font-weight": 700, color: "var(--brass)" }}>SoloForge</span>
        <span class="muted truncate">{world()?.name} · Session {world()?.session}</span>
        <span style={{ flex: 1 }} />
        <span class="chip" aria-live="polite"><span aria-hidden="true" style={{ width: "8px", height: "8px", "border-radius": "9999px", background: busy() ? "var(--brass)" : "var(--vital)" }} />{busy() ? "GM Is Thinking…" : "GM Ready"}</span>
        <button type="button" class="icon-btn" aria-label="Settings" onClick={openSettings}><Icon name="sliders" /></button>
      </header>

      <div class="wrap" style={{ padding: "32px 40px", "max-width": "1280px" }}>
        <main id="story-text" style={{ flex: "999 1 560px", "min-width": 0, display: "flex", "flex-direction": "column", gap: "20px" }}>
          <Show when={art()}>
            {(a) => (
              <figure style={{ margin: 0, display: "flex", "flex-direction": "column", gap: "8px" }}>
                <Show when={a().status === "ready" && a().file} fallback={<div class="art" aria-busy={a().status === "pending"} style={{ width: "100%", "aspect-ratio": "16 / 7", "border-radius": "12px" }}>{a().status === "pending" ? "Painting the Scene…" : "Scene art unavailable"}</div>}>
                  <img src={imageUrl(world()?.id ?? "", a().file)} alt={`Scene art: ${a().prompt}`} width={1600} height={700} style={{ width: "100%", height: "auto", "aspect-ratio": "16 / 7", "object-fit": "cover", "border-radius": "12px" }} />
                </Show>
                <figcaption class="small muted truncate">“{a().prompt}”</figcaption>
              </figure>
            )}
          </Show>
          <h1 class="display">{world()?.scenes.find((s) => s.id === world()?.active_scene)?.name ?? world()?.name}</h1>
          <div aria-live="polite" style={{ display: "flex", "flex-direction": "column", gap: "14px", "max-width": "64ch" }}>
            <For each={(streaming() || narration()).split(/\n{2,}/).filter(Boolean)}>{(p) => <p class="serif" style={{ "font-size": "calc(var(--narration-px, 17px) + 4px)", "line-height": 1.6 }}>{p}</p>}</For>
            <Show when={!streaming() && !narration()}><p class="muted">The GM hasn’t set this scene yet. Send an action, or go back to the map.</p></Show>
          </div>
          <section aria-labelledby="choices-h" style={{ display: "flex", "flex-direction": "column", gap: "10px", "margin-top": "8px" }}>
            <h2 id="choices-h" class="label">{(world()?.story_choices.length ?? 0) ? "The GM Offers · or Write Your Own" : "What Do You Do?"}</h2>
            <For each={world()?.story_choices ?? []}>{(c) => <button type="button" class="choice" aria-pressed={pick() === c} onClick={() => setPick(pick() === c ? null : c)}><strong>{c}</strong></button>}</For>
            <label for="own" class="sr-only">Your own action</label>
            <textarea id="own" name="own-action" class="field" rows="2" autocomplete="off" placeholder="Or describe your own action, e.g. “I blow out the lantern and wait”…" style={{ resize: "none" }} value={own()} onInput={(e) => setOwn(e.currentTarget.value)} onKeyDown={(e) => { if (e.key === "Enter" && !e.shiftKey && !e.isComposing) { e.preventDefault(); void send(); } }} />
            <div style={{ display: "flex", gap: "8px", "align-items": "center", "flex-wrap": "wrap" }}>
              <span style={{ flex: 1 }} />
              <span class="small muted">{pick() && !own().trim() ? `You chose: ${pick()}` : busy() ? "Your action waits until the GM finishes" : ""}</span>
              <Show when={busy()}><button type="button" class="btn btn-danger" onClick={() => act("stop_gm")}>Stop GM</button></Show>
              <button type="button" class="btn btn-primary" onClick={send}>{own().trim() ? "Send Action" : pick() ? "Send Choice" : "Continue Story"}</button>
            </div>
          </section>
        </main>

        <aside aria-label="Hero" style={{ flex: "1 1 280px", "max-width": "320px", display: "flex", "flex-direction": "column", gap: "12px" }}>
          <Show when={pc()}>
            {(a) => (
              <div class="panel" style={{ padding: "16px", display: "flex", "flex-direction": "column", gap: "12px" }}>
                <div style={{ display: "flex", gap: "12px", "align-items": "center" }}>
                  <div class="art" style={{ width: "56px", height: "56px", flex: "0 0 56px", "border-radius": "9999px", "box-shadow": "0 0 0 3px var(--sky)", overflow: "hidden" }}>
                    <Show when={imageUrl(world()?.id ?? "", a().portrait)} fallback={<span aria-hidden="true">{initials(a().name)}</span>}>{(u) => <img src={u()} alt={`Portrait of ${a().name}`} width={56} height={56} style={{ width: "100%", height: "100%", "object-fit": "cover" }} />}</Show>
                  </div>
                  <div style={{ "min-width": 0 }}><div class="truncate" style={{ "font-weight": 700 }} translate="no">{a().name}</div><div class="small muted">Level {a().level} {a().class}</div></div>
                </div>
                <div style={{ display: "flex", "justify-content": "space-between" }}><span class="label">HP</span><span class="mono"><strong>{a().hp}</strong><span class="muted"> / {a().max_hp}</span></span></div>
                <div style={{ display: "flex", "justify-content": "space-between" }}><span class="label">Fate</span><span class="mono" style={{ color: "var(--crit)", "font-weight": 700 }}>{a().fate} / 3</span></div>
                <div style={{ display: "flex", "justify-content": "space-between" }}><span class="label">Tension</span><span class="mono">{world()?.tension} / 9</span></div>
              </div>
            )}
          </Show>
          <Show when={quest()}>{(q) => <div class="panel" style={{ padding: "16px", display: "flex", "flex-direction": "column", gap: "8px" }}><h2 class="label">Active Quest</h2><p style={{ "font-weight": 700 }}>{q().title}</p><Show when={q().objectives.find((o) => !o.done)}>{(o) => <p class="small muted">Next: {o().text}</p>}</Show></div>}</Show>
          <p class="small muted">Story Mode hides the map for scenes without combat. The GM switches back when initiative is rolled.</p>
        </aside>
      </div>
    </div>
  );
}
