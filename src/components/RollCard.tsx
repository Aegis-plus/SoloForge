import { For, Show } from "solid-js";
import type { RollCard as Card } from "../lib/types";

const CHIP: Record<string, { label: string; color: string }> = {
  success: { label: "✓ Success", color: "var(--vital)" },
  fail: { label: "✕ Fail", color: "var(--wound)" },
  crit: { label: "★ Natural 20", color: "var(--crit)" },
  fumble: { label: "Natural 1", color: "var(--wound)" },
};

export function RollCard(props: { card: Card }) {
  const chip = () => CHIP[props.card.outcome];
  const aria = () => `${props.card.by_gm ? "GM roll" : "Roll"}: ${props.card.title}, ${props.card.lines.map((l) => `${l.label} ${l.total}`).join(", ")}${chip() ? `, ${chip()!.label.replace(/[✓✕★]\s?/, "")}` : ""}`;
  return (
    <article class="card" aria-label={aria()} style={{ display: "flex", "flex-direction": "column", gap: "4px", "border-color": props.card.outcome === "crit" ? "var(--crit)" : props.card.outcome === "fumble" ? "var(--wound)" : undefined }}>
      <div style={{ display: "flex", "justify-content": "space-between", gap: "8px", "align-items": "center" }}>
        <span class="truncate" style={{ "font-weight": 700 }}>{props.card.title}</span>
        <Show when={chip()}>
          {(c) => <span class="chip" style={{ "min-height": "24px", "border-color": c().color, color: c().color, background: "transparent" }}>{c().label}</span>}
        </Show>
      </div>
      <For each={props.card.lines}>
        {(l) => (
          <div style={{ display: "flex", "justify-content": "space-between", "align-items": "baseline", gap: "8px" }}>
            <span class="mono small muted" style={{ "min-width": 0, "overflow-wrap": "anywhere" }}>{l.detail}</span>
            <span class="mono" style={{ "font-size": "26px", "line-height": "30px", "font-weight": 700, color: l.kind === "damage" ? "var(--wound)" : props.card.outcome === "crit" && l.kind === "to_hit" ? "var(--crit)" : undefined }}>{l.total}</span>
          </div>
        )}
      </For>
      <Show when={props.card.note}><span class="small muted">{props.card.note}</span></Show>
      <Show when={props.card.by_gm}><span class="small muted">Rolled by the GM{props.card.actor && props.card.actor !== "GM" ? ` for ${props.card.actor}` : ""}</span></Show>
    </article>
  );
}
