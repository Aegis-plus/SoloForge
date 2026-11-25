import { describe, expect, it } from "vitest";
import { describeChange, initials, mod, prof, signed } from "./format";

describe("format", () => {
  it("formats modifiers with a true minus sign", () => {
    expect(signed(5)).toBe("+5");
    expect(signed(0)).toBe("+0");
    expect(signed(-1)).toBe("−1");
  });
  it("matches 5e modifiers and proficiency", () => {
    expect(mod(8)).toBe(-1);
    expect(mod(17)).toBe(3);
    expect(prof(4)).toBe(2);
    expect(prof(5)).toBe(3);
  });
  it("describes sheet changes", () => {
    expect(describeChange({ op: "add_item", name: "Silver bell", qty: 1, note: "" })).toBe("+ Silver bell");
    expect(describeChange({ op: "xp", amount: 1250 })).toMatch(/\+ 1.250 XP/);
    expect(initials("Aria Thorne")).toBe("AT");
  });
});
