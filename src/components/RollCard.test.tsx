import { render, screen } from "@solidjs/testing-library";
import { describe, expect, it } from "vitest";
import { RollCard } from "./RollCard";

describe("RollCard", () => {
  it("shows totals, breakdown and an accessible summary for a crit", () => {
    render(() => <RollCard card={{ title: "Longbow → Goblin A", actor: "Aria", outcome: "crit", note: "Goblin A: 7 → 0 HP · Down", by_gm: false,
      lines: [{ label: "To Hit", detail: "1d20 (20) + 7 vs AC 15", total: 27, kind: "to_hit" }, { label: "Damage", detail: "2d8 (6, 3) + 3 piercing", total: 12, kind: "damage" }] }} />);
    expect(screen.getByRole("article", { name: /Longbow → Goblin A, To Hit 27, Damage 12, Natural 20/ })).toBeTruthy();
    expect(screen.getByText("2d8 (6, 3) + 3 piercing")).toBeTruthy();
    expect(screen.getByText("Goblin A: 7 → 0 HP · Down")).toBeTruthy();
  });
});
