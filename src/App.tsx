import { Match, Show, Switch } from "solid-js";
import { screen, settings, toast, setToast } from "./lib/store";
import { Launcher } from "./screens/Launcher";
import { Wizard } from "./screens/Wizard";
import { Table } from "./screens/Table";
import { Settings } from "./screens/Settings";

export function App() {
  return (
    <>
      <Switch>
        <Match when={screen() === "launcher"}><Launcher /></Match>
        <Match when={screen() === "wizard"}><Wizard /></Match>
        <Match when={screen() === "table"}><Table /></Match>
        <Match when={screen() === "settings" && settings()}><Settings /></Match>
      </Switch>
      <div aria-live="polite" role="status">
        <Show when={toast()}>
          {(t) => (
            <div class="panel toast">
              <span style={{ flex: 1, "min-width": 0 }}>{t().text}</span>
              <Show when={t().action}>{(a) => <button type="button" class="btn btn-sm" onClick={() => { a().run(); setToast(null); }}>{a().label}</button>}</Show>
              <button type="button" class="icon-btn" aria-label="Dismiss" onClick={() => setToast(null)} style={{ width: "32px", height: "32px" }}>✕</button>
            </div>
          )}
        </Show>
      </div>
    </>
  );
}
