import { createResource, createSignal, For, Show } from "solid-js";
import { call, errText } from "../lib/ipc";
import { ago } from "../lib/format";
import { openSettings, openWorld, setScreen, showToast } from "../lib/store";
import type { WorldSummary } from "../lib/types";
import { Icon } from "../components/Icon";

export function Launcher() {
  const [worlds, { refetch }] = createResource(() => call<WorldSummary[]>("list_worlds"));
  const [claude] = createResource(() => call<string>("check_claude").then((v) => ({ ok: true, text: v })).catch((e) => ({ ok: false, text: errText(e) })));
  const [confirm, setConfirm] = createSignal<string | null>(null);

  async function del(w: WorldSummary) {
    setConfirm(null);
    try {
      await call("delete_world", { id: w.id });
      await refetch();
      showToast(`Deleted ${w.name}.`, { label: "Undo", run: async () => { await call("restore_world", { id: w.id }); await refetch(); } });
    } catch (e) { showToast(errText(e)); }
  }

  return (
    <div class="page">
      <a class="skip btn btn-primary" href="#worlds">Skip to Worlds</a>
      <header class="page-head">
        <span class="serif" translate="no" style={{ "font-size": "22px", "font-weight": 700, color: "var(--brass)" }}>SoloForge</span>
        <span style={{ flex: 1 }} />
        <button type="button" class="btn btn-sm" onClick={openSettings}>Settings</button>
      </header>
      <div class="wrap">
        <main id="worlds" style={{ flex: "999 1 600px", "min-width": 0, display: "flex", "flex-direction": "column", gap: "24px" }}>
          <div style={{ display: "flex", "align-items": "flex-end", gap: "16px", "flex-wrap": "wrap" }}>
            <div style={{ flex: 1, "min-width": 0, display: "flex", "flex-direction": "column", gap: "6px" }}>
              <h1 class="display">Your Worlds</h1>
              <p class="muted">Each world keeps its own map, journal and the GM’s memory of your story.</p>
            </div>
            <button type="button" class="btn btn-primary" onClick={() => setScreen("wizard")}><Icon name="plus" />New World</button>
          </div>
          <Show when={!worlds.loading} fallback={<p class="muted">Loading worlds…</p>}>
            <Show when={(worlds() ?? []).length > 0} fallback={
              <div style={{ display: "flex", "flex-direction": "column", "align-items": "center", "justify-content": "center", gap: "12px", "min-height": "360px", border: "1px dashed var(--line-strong)", "border-radius": "12px", "text-align": "center", padding: "24px" }}>
                <h2 class="title">No Worlds Yet</h2>
                <p class="muted" style={{ "max-width": "44ch" }}>Pitch a setting, make a hero, and the GM sets the first scene. It takes about 3&nbsp;minutes.</p>
                <button type="button" class="btn btn-primary" onClick={() => setScreen("wizard")}>Create Your First World</button>
              </div>
            }>
              <ul style={{ "list-style": "none", margin: 0, padding: 0, display: "grid", "grid-template-columns": "repeat(auto-fill, minmax(280px, 1fr))", gap: "16px" }}>
                <For each={worlds()}>
                  {(w) => (
                    <li class="world-card">
                      <div class="art" role="img" aria-label={`Cover art for ${w.name}`} style={{ height: "156px" }}>{w.cover ? "" : "World cover art"}</div>
                      <div style={{ padding: "14px 16px 16px", display: "flex", "flex-direction": "column", gap: "4px" }}>
                        <h2 class="title truncate" title={w.name}>{w.name}</h2>
                        <p class="small muted truncate">{w.genre} · {w.hero}</p>
                        <p class="small muted">Session {w.session} · played <time dateTime={new Date(w.updated_at).toISOString()}>{ago(w.updated_at)}</time></p>
                        <Show when={confirm() === w.id} fallback={
                          <div style={{ display: "flex", gap: "8px", "margin-top": "10px" }}>
                            <button type="button" class="btn btn-sm btn-primary" style={{ flex: 1 }} onClick={() => openWorld(w.id)}>Resume Play</button>
                            <button type="button" class="icon-btn" aria-label={`Delete ${w.name}`} title="Delete World" onClick={() => setConfirm(w.id)}><Icon name="trash" /></button>
                          </div>
                        }>
                          <div role="alertdialog" aria-label={`Confirm deleting ${w.name}`} class="card" style={{ "margin-top": "8px", display: "flex", "flex-direction": "column", gap: "8px", "border-color": "var(--wound)" }}>
                            <p class="small">Delete <strong>{w.name}</strong>? Its map, journal and GM memory go too. You can undo for 10&nbsp;seconds.</p>
                            <div style={{ display: "flex", gap: "8px" }}><button type="button" class="btn btn-sm btn-danger" onClick={() => del(w)}>Delete World</button><button type="button" class="btn btn-sm" onClick={() => setConfirm(null)}>Keep It</button></div>
                          </div>
                        </Show>
                      </div>
                    </li>
                  )}
                </For>
              </ul>
            </Show>
          </Show>
        </main>
        <aside aria-labelledby="ready-h" style={{ flex: "1 1 320px", "max-width": "380px", display: "flex", "flex-direction": "column", gap: "16px" }}>
          <section class="panel" style={{ padding: "20px", display: "flex", "flex-direction": "column", gap: "14px" }}>
            <h2 id="ready-h" class="title">Ready to Play</h2>
            <ul style={{ "list-style": "none", margin: 0, padding: 0, display: "flex", "flex-direction": "column", gap: "12px" }} aria-live="polite">
              <li style={{ display: "flex", gap: "10px" }}>
                <span aria-hidden="true" style={{ color: claude()?.ok ? "var(--vital)" : "var(--wound)", "font-weight": 700 }}>{claude.loading ? "…" : claude()?.ok ? "✓" : "✕"}</span>
                <div style={{ "min-width": 0 }}><strong>Game Master</strong><p class="small muted">{claude.loading ? "Checking Claude Code…" : claude()?.text}</p></div>
              </li>
              <li style={{ display: "flex", gap: "10px" }}><span aria-hidden="true" style={{ color: "var(--vital)", "font-weight": 700 }}>✓</span><div style={{ "min-width": 0 }}><strong>Rules</strong><p class="small muted">Solo 5e-inspired · SRD 5.1</p></div></li>
            </ul>
            <button type="button" class="btn btn-sm" onClick={openSettings}>Set Up Image Endpoint</button>
          </section>
          <p class="small muted">Rules include material from the SRD 5.1 under CC-BY-4.0.</p>
        </aside>
      </div>
    </div>
  );
}
