import { createMemo, createResource, createSignal, For, Match, Show, Switch } from "solid-js";
import { createStore, reconcile, unwrap } from "solid-js/store";
import { call, errText } from "../lib/ipc";
import { applyTheme, screen, setScreen, setSettings, settings, settingsReturn, showToast } from "../lib/store";
import type { Settings as S } from "../lib/types";
import { Icon } from "../components/Icon";

type Sec = "gm" | "img" | "a11y" | "rules";

export function Settings() {
  const [draft, setDraft] = createStore<S>(structuredClone(unwrap(settings()!)));
  const [sec, setSec] = createSignal<Sec>("img");
  const [showKey, setShowKey] = createSignal(false);
  const [test, setTest] = createSignal<{ state: "idle" | "busy" | "ok" | "err"; text: string }>({ state: "idle", text: "" });
  const [confirmLeave, setConfirmLeave] = createSignal(false);
  const [claude, { refetch }] = createResource(() => call<string>("check_claude").then((v) => ({ ok: true, text: v })).catch((e) => ({ ok: false, text: errText(e) })));
  const dirty = createMemo(() => JSON.stringify(draft) !== JSON.stringify(settings()));
  const leave = () => setScreen(settingsReturn() === "settings" ? "launcher" : settingsReturn());

  async function save() {
    try {
      const s = await call<S>("save_settings", { settings: structuredClone(unwrap(draft)) });
      setSettings(s);
      setDraft(reconcile(structuredClone(s)));
      applyTheme(s);
      showToast("Settings saved.", undefined, 3000);
    } catch (e) { showToast(errText(e), undefined, 8000); }
  }
  async function runTest() {
    setTest({ state: "busy", text: "" });
    try {
      const r = await call<{ bytes: number; ms: number }>("test_image", { settings: structuredClone(unwrap(draft)) });
      setTest({ state: "ok", text: `${new Intl.NumberFormat(undefined, { style: "unit", unit: "kilobyte", maximumFractionDigits: 0 }).format(r.bytes / 1024)} image in ${new Intl.NumberFormat(undefined, { maximumFractionDigits: 1 }).format(r.ms / 1000)} s` });
    } catch (e) { setTest({ state: "err", text: errText(e) }); }
  }
  const SECS: [Sec, string][] = [["gm", "Game Master"], ["img", "Image Endpoint"], ["a11y", "Appearance & Accessibility"], ["rules", "Rules"]];
  const SAVE: Record<Sec, string> = { gm: "Save GM Settings", img: "Save Image Settings", a11y: "Save Appearance", rules: "Save Rules" };
  const endpoint = () => `${draft.image.base_url.replace(/\/$/, "")}/${draft.image.api_style === "chat" ? "chat/completions" : "images/generations"}`;
  const check = (label: string, get: () => boolean, set: (v: boolean) => void, name: string) => (
    <label class="check"><input type="checkbox" name={name} checked={get()} onChange={(e) => set(e.currentTarget.checked)} />{label}</label>
  );

  return (
    <div class="page">
      <a class="skip btn btn-primary" href="#settings-body">Skip to Settings</a>
      <header class="page-head">
        <button type="button" class="btn btn-sm" onClick={() => (dirty() ? setConfirmLeave(true) : leave())}><Icon name="back" size={16} />{screen() === "settings" && settingsReturn() === "table" ? "Back to Table" : "Back"}</button>
        <h1 class="title">Settings</h1>
      </header>
      <div class="wrap" style={{ "max-width": "1200px", padding: "32px 40px" }}>
        <nav aria-label="Settings sections" style={{ flex: "1 1 200px", "max-width": "240px" }}>
          <ul style={{ "list-style": "none", margin: 0, padding: 0, display: "flex", "flex-direction": "column", gap: "2px" }}>
            <For each={SECS}>{([k, l]) => <li><button type="button" class="row" aria-current={sec() === k} onClick={() => setSec(k)} style={{ "font-weight": 700 }}>{l}</button></li>}</For>
          </ul>
        </nav>
        <main id="settings-body" style={{ flex: "999 1 520px", "min-width": 0, display: "flex", "flex-direction": "column", gap: "24px", "max-width": "720px" }}>
          <Switch>
            <Match when={sec() === "gm"}>
              <div><h2 class="title">Game Master</h2><p class="muted" style={{ "margin-top": "4px" }}><span translate="no">Claude Code</span> runs your GM with the login you already have. No API key needed.</p></div>
              <div class="card" role="status" style={{ display: "flex", gap: "12px", "align-items": "center", "border-color": claude()?.ok ? "var(--vital)" : "var(--wound)" }}>
                <span style={{ color: claude()?.ok ? "var(--vital)" : "var(--wound)", "font-weight": 700 }}>{claude.loading ? "Checking…" : claude()?.ok ? "✓ Found" : "✕ Not Found"}</span>
                <span class="small muted" style={{ flex: 1, "min-width": 0 }}>{claude()?.text}</span>
                <button type="button" class="btn btn-sm" onClick={() => refetch()}>Check Again</button>
              </div>
              <div style={{ display: "flex", "flex-direction": "column", gap: "6px" }}><label for="cpath" class="label">Claude Code Command</label><input id="cpath" name="claude-command" class="field mono" autocomplete="off" spellcheck={false} translate="no" value={draft.gm.claude_command} onInput={(e) => setDraft("gm", "claude_command", e.currentTarget.value)} /><p class="small muted">Usually just <span class="mono">claude</span>. Enter a full path if it isn’t on your PATH.</p></div>
              <div style={{ display: "flex", "flex-direction": "column", gap: "6px" }}><label for="cmodel" class="label">Model</label>
                <select id="cmodel" name="claude-model" class="field" autocomplete="off" value={draft.gm.model} onChange={(e) => setDraft("gm", "model", e.currentTarget.value)}><option value="">Default (from your Claude Code settings)</option><option value="opus">Opus</option><option value="sonnet">Sonnet</option><option value="haiku">Haiku</option></select></div>
              <fieldset style={{ border: 0, padding: 0, margin: 0 }}><legend class="label" style={{ "margin-bottom": "8px" }}>Narration Style</legend>
                <div style={{ display: "flex", gap: "6px", "flex-wrap": "wrap" }}><For each={["terse", "vivid", "novelistic"]}>{(s) => <button type="button" class="chip" aria-pressed={draft.gm.style === s} onClick={() => setDraft("gm", "style", s)}>{s[0].toUpperCase() + s.slice(1)}</button>}</For></div></fieldset>
              <fieldset style={{ border: 0, padding: 0, margin: 0 }}><legend class="label" style={{ "margin-bottom": "6px" }}>What the GM May Do</legend>
                <p class="small muted" style={{ "margin-bottom": "4px" }}>The GM only ever has the table tools: dice, tokens, sheets, journal and images. No files, shell or web.</p>
                {check("Roll secretly for hidden DCs and enemy saves", () => draft.gm.secret_rolls, (v) => setDraft("gm", "secret_rolls", v), "secret-rolls")}
                {check("Ask before giving items or XP to your hero", () => draft.gm.ask_before_sheet_changes, (v) => setDraft("gm", "ask_before_sheet_changes", v), "ask-sheet")}
              </fieldset>
            </Match>

            <Match when={sec() === "img"}>
              <div><h2 class="title">Image Endpoint</h2><p class="muted" style={{ "margin-top": "4px" }}>Any OpenAI-compatible API. Use it for Nano Banana through OpenRouter, LiteLLM or your own gateway.</p></div>
              <div style={{ display: "flex", "flex-direction": "column", gap: "6px" }}><label for="base" class="label">Base URL</label><input id="base" name="image-base-url" type="url" inputmode="url" class="field mono" autocomplete="off" spellcheck={false} translate="no" placeholder="https://openrouter.ai/api/v1…" value={draft.image.base_url} onInput={(e) => setDraft("image", "base_url", e.currentTarget.value)} /></div>
              <div style={{ display: "flex", gap: "16px", "flex-wrap": "wrap" }}>
                <div style={{ flex: "1 1 260px", "min-width": 0, display: "flex", "flex-direction": "column", gap: "6px" }}>
                  <label for="key" class="label">API Key</label>
                  <div style={{ display: "flex", gap: "6px" }}><input id="key" name="image-api-key" type={showKey() ? "text" : "password"} class="field mono" autocomplete="off" spellcheck={false} translate="no" placeholder="sk-…" value={draft.image.api_key} onInput={(e) => setDraft("image", "api_key", e.currentTarget.value)} aria-describedby="key-help" /><button type="button" class="btn btn-sm" style={{ "min-height": "40px" }} aria-pressed={showKey()} onClick={() => setShowKey(!showKey())}>{showKey() ? "Hide" : "Show"}</button></div>
                  <p id="key-help" class="small muted">Saved in your config folder, readable only by you. Or set <span class="mono" translate="no">SOLOFORGE_IMAGE_API_KEY</span>.</p>
                </div>
                <div style={{ flex: "1 1 260px", "min-width": 0, display: "flex", "flex-direction": "column", gap: "6px" }}><label for="model" class="label">Model</label><input id="model" name="image-model" class="field mono" autocomplete="off" spellcheck={false} translate="no" value={draft.image.model} onInput={(e) => setDraft("image", "model", e.currentTarget.value)} /></div>
              </div>
              <fieldset style={{ border: 0, padding: 0, margin: 0, display: "flex", "flex-direction": "column", gap: "8px" }}><legend class="label" style={{ "margin-bottom": "6px" }}>API Style</legend>
                <div class="group" style={{ display: "grid", "grid-template-columns": "repeat(2, minmax(0, 1fr))", "max-width": "460px", border: "1px solid var(--line-strong)", "border-radius": "6px", overflow: "hidden" }}>
                  <button type="button" class="seg" aria-pressed={draft.image.api_style === "chat"} onClick={() => setDraft("image", "api_style", "chat")}>Chat Completions</button>
                  <button type="button" class="seg" aria-pressed={draft.image.api_style === "images"} onClick={() => setDraft("image", "api_style", "images")}>Images API</button>
                </div>
                <p class="mono small muted" translate="no" style={{ "overflow-wrap": "anywhere" }}>POST {endpoint()}</p>
                <p class="small muted">{draft.image.api_style === "chat" ? "Sends modalities [\"image\", \"text\"]; reference images go in as image_url parts. This is how OpenRouter serves Nano Banana." : "Sends prompt and size and expects b64_json. Reference images use /images/edits."}</p>
              </fieldset>
              <div style={{ display: "flex", "flex-direction": "column", gap: "6px" }}><label for="style" class="label">World Art Style</label><textarea id="style" name="art-style" class="field" rows="2" autocomplete="off" style={{ resize: "vertical" }} value={draft.image.art_style} onInput={(e) => setDraft("image", "art_style", e.currentTarget.value)} /><p class="small muted">Added to every map, portrait and scene prompt.</p></div>
              {check("Keep your hero consistent by sending the portrait as a reference image", () => draft.image.hero_consistency, (v) => setDraft("image", "hero_consistency", v), "consistency")}
              {check("Let the GM generate maps and scene art on its own", () => draft.image.gm_can_generate, (v) => setDraft("image", "gm_can_generate", v), "gm-maps")}
              <div class="card" style={{ display: "flex", gap: "12px", "align-items": "center" }}>
                <div aria-live="polite" style={{ flex: 1, "min-width": 0 }}>
                  <Switch fallback={<p class="small muted">Generates one small test image. Costs one image on your account.</p>}>
                    <Match when={test().state === "busy"}><p class="small">Testing Connection…</p><div class="bar" aria-hidden="true" style={{ "margin-top": "6px" }} /></Match>
                    <Match when={test().state === "ok"}><p style={{ color: "var(--vital)", "font-weight": 700 }}>✓ Test Image Received</p><p class="small muted">{test().text}</p></Match>
                    <Match when={test().state === "err"}><p style={{ color: "var(--wound)", "font-weight": 700 }}>✕ Test Failed</p><p class="small muted">{test().text}</p></Match>
                  </Switch>
                </div>
                <button type="button" class="btn" onClick={runTest} disabled={test().state === "busy"}>Test Connection</button>
              </div>
            </Match>

            <Match when={sec() === "a11y"}>
              <h2 class="title">Appearance &amp; Accessibility</h2>
              <fieldset style={{ border: 0, padding: 0, margin: 0 }}><legend class="label" style={{ "margin-bottom": "8px" }}>Theme</legend>
                <div style={{ display: "flex", gap: "6px", "flex-wrap": "wrap" }}><For each={[["dark", "Midnight"], ["light", "Vellum"], ["system", "Match System"]] as const}>{([k, l]) => <button type="button" class="chip" aria-pressed={draft.appearance.theme === k} onClick={() => setDraft("appearance", "theme", k)}>{l}</button>}</For></div></fieldset>
              <div style={{ display: "flex", "flex-direction": "column", gap: "6px", "max-width": "420px" }}><label for="ts" class="label">Narration Size</label><input id="ts" name="narration-size" type="range" min="15" max="24" autocomplete="off" value={draft.appearance.narration_px} onInput={(e) => setDraft("appearance", "narration_px", Number(e.currentTarget.value))} /><p class="small muted"><span class="mono">{draft.appearance.narration_px}</span>&nbsp;px · Story Mode scales up to match</p></div>
              {check("Reduce motion (also follows your system setting)", () => draft.appearance.reduce_motion, (v) => setDraft("appearance", "reduce_motion", v), "reduce-motion")}
              {check("High-contrast theme (7:1 text)", () => draft.appearance.high_contrast, (v) => setDraft("appearance", "high_contrast", v), "high-contrast")}
              {check("Announce GM narration and rolls to screen readers", () => draft.appearance.announce, (v) => setDraft("appearance", "announce", v), "announce")}
            </Match>

            <Match when={sec() === "rules"}>
              <h2 class="title">Rules</h2>
              <div class="card"><p><strong>Solo 5e-inspired</strong> · levels 1–10</p><p class="small muted">Includes material from the SRD 5.1 under CC-BY-4.0.</p></div>
              {check("Fate points: 3 per long rest", () => draft.rules.fate, (v) => setDraft("rules", "fate", v), "fate")}
              {check("Minions drop on any hit", () => draft.rules.minions, (v) => setDraft("rules", "minions", v), "minions")}
              {check("Oracle random events, driven by Tension", () => draft.rules.oracle_events, (v) => setDraft("rules", "oracle_events", v), "oracle-events")}
              {check("Lethal death saves (off: you’re only “out of action”)", () => draft.rules.lethal, (v) => setDraft("rules", "lethal", v), "lethal")}
            </Match>
          </Switch>

          <Show when={dirty() || confirmLeave()}>
            <div role="status" aria-live="polite" class="panel" style={{ position: "sticky", bottom: "16px", display: "flex", gap: "12px", "align-items": "center", padding: "10px 12px 10px 16px", "flex-wrap": "wrap" }}>
              <span style={{ flex: 1, "min-width": 0 }} class="small">{confirmLeave() ? "Leave without saving? Your changes will be lost." : "You have unsaved changes."}</span>
              <Show when={confirmLeave()} fallback={<button type="button" class="btn btn-sm" onClick={() => setDraft(reconcile(structuredClone(unwrap(settings()!))))}>Discard Changes</button>}>
                <button type="button" class="btn btn-sm btn-danger" onClick={leave}>Leave Without Saving</button>
                <button type="button" class="btn btn-sm" onClick={() => setConfirmLeave(false)}>Keep Editing</button>
              </Show>
              <button type="button" class="btn btn-sm btn-primary" onClick={async () => { await save(); if (confirmLeave()) leave(); }}>{SAVE[sec()]}</button>
            </div>
          </Show>
        </main>
      </div>
    </div>
  );
}
