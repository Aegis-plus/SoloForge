import { createEffect, createSignal, For, Match, on, Show, Switch } from "solid-js";
import { act, gm, imageUrl, pc, streaming, world } from "./_shared";
import { describeChange } from "../lib/format";
import type { ChatEntry } from "../lib/types";
import { Icon, StopIcon } from "./Icon";
import { RollCard } from "./RollCard";

function Paragraphs(props: { text: string; class?: string }) {
  return <div class={`md ${props.class ?? ""}`}><For each={props.text.split(/\n{2,}/)}>{(p) => <p>{p}</p>}</For></div>;
}

function Entry(props: { e: ChatEntry }) {
  const wid = () => world()?.id ?? "";
  return (
    <Switch>
      <Match when={props.e.type === "gm" && props.e}>
        {(e) => <article style={{ display: "flex", "flex-direction": "column", gap: "4px" }}><h2 class="small" style={{ "font-weight": 700, color: "var(--brass)" }}>GM</h2><Paragraphs class="narration serif" text={(e() as { text: string }).text} /></article>}
      </Match>
      <Match when={props.e.type === "player" && props.e}>
        {(e) => <article style={{ display: "flex", "flex-direction": "column", gap: "4px", "align-items": "flex-end" }}><h2 class="small" style={{ "font-weight": 700, color: "var(--sky)" }}>You · {pc()?.name.split(" ")[0]}</h2><p class="card" style={{ "max-width": "290px", "overflow-wrap": "anywhere", "white-space": "pre-wrap" }}>{(e() as { text: string }).text}</p></article>}
      </Match>
      <Match when={props.e.type === "roll" && props.e}>{(e) => <RollCard card={(e() as Extract<ChatEntry, { type: "roll" }>).card} />}</Match>
      <Match when={props.e.type === "tool" && props.e}>{(e) => <p class="small muted">{(e() as { text: string }).text}.</p>}</Match>
      <Match when={props.e.type === "system" && props.e}>
        {(e) => <div class="small muted" style={{ display: "flex", "align-items": "center", gap: "8px", "text-align": "center" }}><span aria-hidden="true" style={{ flex: 1, height: "1px", background: "var(--line)" }} /><span style={{ "max-width": "80%" }}>{(e() as { text: string }).text}</span><span aria-hidden="true" style={{ flex: 1, height: "1px", background: "var(--line)" }} /></div>}
      </Match>
      <Match when={props.e.type === "oracle" && (props.e as Extract<ChatEntry, { type: "oracle" }>)}>
        {(e) => (
          <article class="card" style={{ display: "flex", "flex-direction": "column", gap: "6px" }}>
            <div style={{ display: "flex", "justify-content": "space-between", gap: "8px" }}><strong class="truncate">Oracle · “{e().question}”</strong><span class="chip" style={{ "min-height": "24px" }}>{e().likelihood}</span></div>
            <div style={{ display: "flex", "justify-content": "space-between", "align-items": "baseline" }}><span class="mono small muted">d100 ({e().roll})</span><span class="serif" style={{ "font-size": "24px", "font-weight": 700 }}>{e().answer}</span></div>
            <Show when={e().event}><p class="small" style={{ color: "var(--crit)" }}>Random event · “{e().event}”</p></Show>
          </article>
        )}
      </Match>
      <Match when={props.e.type === "image" && (props.e as Extract<ChatEntry, { type: "image" }>)}>
        {(e) => (
          <article class="card" aria-busy={e().status === "pending"} style={{ display: "flex", "flex-direction": "column", gap: "8px" }}>
            <Switch>
              <Match when={e().status === "pending"}>
                <div class="art" style={{ position: "relative", height: "140px", "border-radius": "6px", overflow: "hidden" }}><div style={{ position: "absolute", inset: 0, background: "var(--scrim)", display: "flex", "align-items": "center", "justify-content": "center", color: "#e8ecf2" }}><strong>Painting the Scene…</strong></div></div>
                <div class="bar" aria-hidden="true" />
              </Match>
              <Match when={e().status === "ready" && e().file}>
                <img src={imageUrl(wid(), e().file)} alt={`Scene art: ${e().prompt}`} width={1024} height={576} loading="lazy" style={{ width: "100%", height: "auto", "border-radius": "6px" }} />
              </Match>
              <Match when={e().status === "failed"}>
                <p style={{ color: "var(--wound)", "font-weight": 700 }}>✕ Image Not Generated</p>
                <p class="small muted">{e().error}</p>
              </Match>
            </Switch>
            <p class="small muted">“{e().prompt}”</p>
          </article>
        )}
      </Match>
    </Switch>
  );
}

export function Chat() {
  let log!: HTMLDivElement;
  let input!: HTMLTextAreaElement;
  const [text, setText] = createSignal("");
  const [tray, setTray] = createSignal<"none" | "dice" | "oracle">("none");
  const [expr, setExpr] = createSignal("1d20");
  const [question, setQuestion] = createSignal("");
  const [likelihood, setLikelihood] = createSignal("even odds");
  const busy = () => gm().state === "thinking" || gm().state === "acting";

  createEffect(on(() => [world()?.chat.length, streaming()], () => queueMicrotask(() => { if (log) log.scrollTop = log.scrollHeight; })));

  async function send(t: string) {
    const msg = t.trim();
    if (!msg && !busy()) {
      await act("send_message", { text: "" });
      return;
    }
    setText("");
    await act("send_message", { text: msg });
    input?.focus();
  }

  return (
    <>
      <div id="chat-log" role="log" aria-live="polite" aria-label="Story so far" tabindex="0" class="scroll" ref={log} style={{ flex: 1, "min-height": 0, padding: "16px", display: "flex", "flex-direction": "column", gap: "14px" }}>
        <Show when={(world()?.chat.length ?? 0) === 0}><p class="small muted">Nothing has happened yet. Say what your hero does, or press Begin Session Zero.</p></Show>
        <For each={world()?.chat ?? []}>{(e) => <Entry e={e} />}</For>
        <Show when={streaming()}>
          <article style={{ display: "flex", "flex-direction": "column", gap: "4px" }}><h2 class="small" style={{ "font-weight": 700, color: "var(--brass)" }}>GM</h2><p class="narration serif" style={{ "white-space": "pre-wrap" }}>{streaming()}<span class="caret" aria-hidden="true" /></p></article>
        </Show>
        <For each={world()?.proposals ?? []}>
          {(p) => (
            <article class="card" style={{ display: "flex", "flex-direction": "column", gap: "8px", "border-color": "var(--brass)" }}>
              <p style={{ "font-weight": 700 }}>The GM Proposes Changes to Your Sheet</p>
              <ul style={{ "list-style": "none", margin: 0, padding: 0, display: "flex", "flex-direction": "column", gap: "4px" }}><For each={p.changes}>{(c) => <li>{describeChange(c)}</li>}</For></ul>
              <p class="small muted">{p.reason}</p>
              <div style={{ display: "flex", gap: "8px" }}>
                <button type="button" class="btn btn-sm btn-primary" style={{ flex: 1 }} onClick={() => act("resolve_proposal", { id: p.id, accept: true })}>Accept {p.changes.length === 1 ? "Change" : `All ${p.changes.length}`}</button>
                <button type="button" class="btn btn-sm" onClick={() => act("resolve_proposal", { id: p.id, accept: false })}>Decline</button>
              </div>
            </article>
          )}
        </For>
      </div>

      <form style={{ position: "relative", padding: "12px 16px 16px", "border-top": "1px solid var(--line)", display: "flex", "flex-direction": "column", gap: "10px" }} onSubmit={(ev) => { ev.preventDefault(); void send(text()); }}>
        <Show when={busy()}><div class="bar" aria-hidden="true" style={{ position: "absolute", left: 0, right: 0, top: "-1px", "border-radius": 0 }} /></Show>
        <Show when={(world()?.story_choices.length ?? 0) > 0 && !busy()}>
          <div role="group" aria-label="Suggested actions from the GM" style={{ display: "flex", gap: "6px", "flex-wrap": "wrap" }}>
            <For each={world()?.story_choices}>{(c) => <button type="button" class="chip" onClick={() => send(c)}>{c}</button>}</For>
          </div>
        </Show>
        <Show when={tray() === "dice"}>
          <div style={{ display: "flex", gap: "6px", "align-items": "center" }}>
            <label for="dice-expr" class="sr-only">Dice expression</label>
            <input id="dice-expr" name="dice-expression" class="field mono" autocomplete="off" spellcheck={false} value={expr()} onInput={(e) => setExpr(e.currentTarget.value)} placeholder="e.g. 2d6+3…" />
            <button type="button" class="btn btn-sm" onClick={async () => { if (await act("roll", { expr: expr(), label: `Roll ${expr()}` })) setTray("none"); }}>Roll {expr() || "Dice"}</button>
          </div>
        </Show>
        <Show when={tray() === "oracle"}>
          <div style={{ display: "flex", "flex-direction": "column", gap: "6px" }}>
            <label for="oracle-q" class="sr-only">Yes or no question</label>
            <input id="oracle-q" name="oracle-question" class="field" autocomplete="off" value={question()} onInput={(e) => setQuestion(e.currentTarget.value)} placeholder="e.g. Is the crypt door locked?…" />
            <div style={{ display: "flex", gap: "6px" }}>
              <label for="oracle-l" class="sr-only">Likelihood</label>
              <select id="oracle-l" name="oracle-likelihood" class="field" autocomplete="off" value={likelihood()} onChange={(e) => setLikelihood(e.currentTarget.value)}>
                <option value="impossible">Nearly Impossible</option><option value="unlikely">Unlikely</option><option value="even odds">Even Odds</option><option value="likely">Likely</option><option value="certain">Nearly Certain</option>
              </select>
              <button type="button" class="btn btn-sm" disabled={!question().trim()} onClick={async () => { await act("ask_oracle", { question: question(), likelihood: likelihood() }); setQuestion(""); setTray("none"); }}>Ask Oracle</button>
            </div>
          </div>
        </Show>
        <label for="say" class="sr-only">Message to the GM</label>
        <textarea id="say" ref={input} name="message" class="field" rows="2" autocomplete="off" placeholder="Describe what you do, e.g. “I shoot the goblin”…" style={{ resize: "none" }} value={text()} onInput={(e) => setText(e.currentTarget.value)} onKeyDown={(e) => { if (e.key === "Enter" && !e.shiftKey && !e.isComposing) { e.preventDefault(); void send(text()); } }} />
        <div style={{ display: "flex", gap: "6px", "align-items": "center" }}>
          <button type="button" class="btn btn-sm" aria-pressed={tray() === "dice"} onClick={() => setTray(tray() === "dice" ? "none" : "dice")}><Icon name="dice" size={16} />Roll Dice</button>
          <button type="button" class="btn btn-sm" aria-pressed={tray() === "oracle"} onClick={() => setTray(tray() === "oracle" ? "none" : "oracle")}>Ask Oracle</button>
          <span style={{ flex: 1 }} />
          <Show when={busy()}>
            <button type="button" class="icon-btn" aria-label="Stop the GM" title="Stop (Esc)" style={{ color: "var(--wound)", "border-color": "var(--wound)" }} onClick={() => act("stop_gm")}><StopIcon /></button>
          </Show>
          <button type="submit" class="btn btn-sm btn-primary">{busy() ? "Queue Message" : text().trim() ? "Send" : (world()?.chat.some((c) => c.type === "gm") ? "Continue Story" : "Begin Session Zero")}</button>
        </div>
      </form>
    </>
  );
}
