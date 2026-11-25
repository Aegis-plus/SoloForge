import type { Abilities, Ability, Actor, SheetChange } from "./types";

export const ABILITIES: Ability[] = ["str", "dex", "con", "int", "wis", "cha"];
export const ABILITY_NAMES: Record<Ability, string> = { str: "Strength", dex: "Dexterity", con: "Constitution", int: "Intelligence", wis: "Wisdom", cha: "Charisma" };
export const SKILLS: [string, string, Ability][] = [
  ["acrobatics", "Acrobatics", "dex"], ["animal_handling", "Animal Handling", "wis"], ["arcana", "Arcana", "int"], ["athletics", "Athletics", "str"],
  ["deception", "Deception", "cha"], ["history", "History", "int"], ["insight", "Insight", "wis"], ["intimidation", "Intimidation", "cha"],
  ["investigation", "Investigation", "int"], ["medicine", "Medicine", "wis"], ["nature", "Nature", "int"], ["perception", "Perception", "wis"],
  ["performance", "Performance", "cha"], ["persuasion", "Persuasion", "cha"], ["religion", "Religion", "int"], ["sleight_of_hand", "Sleight of Hand", "dex"],
  ["stealth", "Stealth", "dex"], ["survival", "Survival", "wis"],
];

export const mod = (score: number) => Math.floor((score - 10) / 2);
/** "+5" / "−1" with a true minus sign. */
export const signed = (n: number) => (n >= 0 ? `+${n}` : `−${Math.abs(n)}`);
export const prof = (level: number) => 2 + Math.floor((Math.max(1, level) - 1) / 4);

export function skillBonus(a: Actor, key: string, ab: Ability) {
  return mod(a.abilities[ab]) + (a.skill_profs.includes(key) ? prof(a.level) : 0);
}
export function saveBonus(a: Actor, ab: Ability) {
  return mod(a.abilities[ab]) + (a.save_profs.includes(ab) ? prof(a.level) : 0);
}

const nf = new Intl.NumberFormat(undefined);
export const num = (n: number) => nf.format(n);
const rtf = new Intl.RelativeTimeFormat(undefined, { numeric: "auto" });
export function ago(ms: number) {
  const mins = Math.round((ms - Date.now()) / 60000);
  if (Math.abs(mins) < 60) return rtf.format(mins, "minute");
  const h = Math.round(mins / 60);
  if (Math.abs(h) < 24) return rtf.format(h, "hour");
  return rtf.format(Math.round(h / 24), "day");
}

export const XP_TABLE = [0, 300, 900, 2700, 6500, 14000, 23000, 34000, 48000, 64000];

export function initials(name: string) {
  return name.split(/\s+/).filter(Boolean).slice(0, 2).map((w) => w[0]?.toUpperCase()).join("");
}

export function describeChange(c: SheetChange) {
  switch (c.op) {
    case "add_item": return `+ ${c.name}${c.qty > 1 ? ` ×${c.qty}` : ""}${c.note ? ` (${c.note})` : ""}`;
    case "remove_item": return `− ${c.qty} ${c.name}`;
    case "xp": return `+ ${num(c.amount)} XP`;
    case "gold": return `${signed(c.amount)} gp`;
    case "condition": return `${c.add ? "Gains" : "Loses"} ${c.name}`;
  }
}

/** Point-buy cost of a score (8–15). */
export const POINT_COST: Record<number, number> = { 8: 0, 9: 1, 10: 2, 11: 3, 12: 4, 13: 5, 14: 7, 15: 9 };
export const ANCESTRY_BONUS: Record<string, Partial<Abilities>> = {
  Human: { str: 1, dex: 1, con: 1, int: 1, wis: 1, cha: 1 },
  "Wood Elf": { dex: 2, wis: 1 },
  Dwarf: { con: 2, wis: 1 },
  Halfling: { dex: 2, cha: 1 },
  "Half-Orc": { str: 2, con: 1 },
  Custom: { dex: 1, con: 1, wis: 1 },
};
