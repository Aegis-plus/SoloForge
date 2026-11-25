// Browser-only stand-in for the Rust engine (vite dev, Playwright, screenshots).
// It is NOT used inside the Tauri app.
import monsters from "../../assets/compendium/monsters.json";
import items from "../../assets/compendium/items.json";
import spells from "../../assets/compendium/spells.json";
import type { Actor, ChatEntry, RollCard, Settings, World, WorldSummary } from "./types";

export const mockBus = new EventTarget();
const emit = (name: string, detail: unknown) => mockBus.dispatchEvent(new CustomEvent(name, { detail }));

let n = 0;
const id = () => `m${(++n).toString(36)}${Date.now().toString(36).slice(-4)}`;
const d = (s: number) => 1 + Math.floor(Math.random() * s);
const now = () => Date.now();
const sign = (x: number) => (x >= 0 ? `+${x}` : `−${-x}`);
const mod = (s: number) => Math.floor((s - 10) / 2);

function hero(): Actor {
  return {
    id: "aria", name: "Aria Thorne", kind: "pc", disposition: "friendly", level: 3, class: "Ranger", ancestry: "Wood Elf",
    background: "Raised by the wardens who failed to stop the fire.",
    abilities: { str: 10, dex: 17, con: 14, int: 10, wis: 14, cha: 8 }, save_profs: ["str", "dex"],
    skill_profs: ["stealth", "perception", "survival", "athletics", "nature", "animal_handling"],
    max_hp: 31, hp: 24, temp_hp: 0, ac: 15, speed: 35,
    attacks: [
      { name: "Longbow", to_hit: 7, damage: "1d8+3", damage_type: "piercing", range: "150/600 ft" },
      { name: "Shortsword", to_hit: 5, damage: "1d6+3", damage_type: "piercing", range: "melee" },
    ],
    spells: [
      { name: "Hunter’s Mark", level: 1, summary: "Bonus action · 90 ft · +1d6 to the marked target" },
      { name: "Cure Wounds", level: 1, summary: "Action · touch · heal 1d8+2" },
    ],
    spell_slots: [2], spell_slots_max: [3], concentrating: "Hunter’s Mark",
    inventory: [
      { name: "Studded leather (worn)", qty: 1, note: "" }, { name: "Longbow", qty: 1, note: "" }, { name: "Arrows", qty: 18, note: "" },
      { name: "Shortsword", qty: 1, note: "" }, { name: "Healing potion", qty: 2, note: "2d4+2" }, { name: "Rope, hempen (50 ft)", qty: 1, note: "" },
    ],
    gold: 23, conditions: [], fate: 2, xp: 1250, hit_dice: 3, portrait: null,
    appearance: "Lean wood elf in a moss-green hooded cloak.", minion: false, cr: "", traits: [], notes: "",
  };
}

function goblin(name: string, hp = 7): Actor {
  return { ...hero(), id: name.toLowerCase().replace(/\s/g, "-"), name, kind: "monster", disposition: "hostile", class: "Small humanoid", ancestry: "", level: 1,
    abilities: { str: 8, dex: 14, con: 10, int: 10, wis: 8, cha: 8 }, save_profs: [], skill_profs: [], max_hp: 7, hp, ac: 15, speed: 30,
    attacks: [{ name: "Scimitar", to_hit: 4, damage: "1d6+2", damage_type: "slashing", range: "melee" }], spells: [], spell_slots: [], spell_slots_max: [],
    concentrating: null, inventory: [], gold: 0, conditions: hp <= 0 ? ["Down"] : [], fate: 0, xp: 0, minion: true, cr: "1/4", traits: ["Nimble Escape."] };
}

function sampleWorld(): World {
  const W = 22, H = 15;
  const revealed = Array.from({ length: W * H }, (_, i) => { const x = i % W, y = Math.floor(i / W); return !((x >= 16 && y >= 10) || (x >= 17 && y <= 3)); });
  const t = now();
  const chat: ChatEntry[] = [
    { id: id(), ts: t - 600000, type: "system", text: "Session 4 begins." },
    { id: id(), ts: t - 500000, type: "gm", text: "Rain hisses through the broken roof. Two goblins crouch by a brazier near the altar, arguing over a silver bell." },
    { id: id(), ts: t - 400000, type: "player", text: "I creep along the pews toward the altar, keeping low." },
    { id: id(), ts: t - 390000, type: "roll", card: { title: "Stealth Check", actor: "Aria Thorne", lines: [{ label: "Stealth Check", detail: "1d20 (14) + 5", total: 19, kind: "check" }], outcome: "success", note: "", by_gm: false } },
    { id: id(), ts: t - 380000, type: "tool", text: "GM rolled Perception Check for Goblin A secretly" },
    { id: id(), ts: t - 370000, type: "tool", text: "GM revealed 14 cells" },
    { id: id(), ts: t - 300000, type: "roll", card: { title: "Longbow → Goblin A", actor: "Aria Thorne", lines: [{ label: "To Hit", detail: "1d20 (20) + 7 vs AC 15", total: 27, kind: "to_hit" }, { label: "Damage", detail: "2d8 (6, 3) + 3 piercing", total: 12, kind: "damage" }], outcome: "crit", note: "Goblin A: 7 → 0 HP · Down", by_gm: false } },
    { id: id(), ts: t - 200000, type: "gm", text: "Your arrow takes the first goblin clean through the throat. The bell clatters across the flagstones, and its partner shrieks and scrambles for it." },
  ];
  return {
    id: "vale", name: "The Ashen Vale", pitch: "A valley that burned 40 years ago and never grew back.", tone: ["Gritty", "Mystery"], rules_dial: "balanced",
    created_at: t - 86400000 * 9, updated_at: t - 7200000, session: 4, gm_session_id: "demo",
    actors: [hero(), goblin("Goblin A", 0), goblin("Goblin B")],
    scenes: [{ id: "chapel", name: "Ruined Chapel", description: "Rain, a brazier, an altar.", width: W, height: H, background: null, map_status: "none",
      tokens: [{ id: "t-aria", actor_id: "aria", x: 4, y: 9, hidden: false }, { id: "t-ga", actor_id: "goblin-a", x: 14, y: 4, hidden: false }, { id: "t-gb", actor_id: "goblin-b", x: 16, y: 5, hidden: false }],
      fog: true, revealed }],
    active_scene: "chapel",
    combat: { round: 2, turn: 0, name: "Chapel Ambush", order: [{ actor_id: "aria", initiative: 19, hidden: false }, { actor_id: "goblin-b", initiative: 12, hidden: false }, { actor_id: "goblin-a", initiative: 9, hidden: false }] },
    journal: [
      { id: "q1", title: "The Bell That Won’t Stop Ringing", category: "quest", active: true, by_player: false, image: null, updated_at: t,
        body: "Every night at the third hour, the chapel bell rings over Emberford, though the chapel has stood empty since the fire. Reeve Daro has offered 50 gp to anyone who silences it.",
        objectives: [{ text: "Reach the ruined chapel", done: true }, { text: "Find out who rings the bell", done: false }, { text: "Return to Reeve Daro in Emberford", done: false }] },
      { id: "n1", title: "Mother Ilse, the Last Abbess", category: "npc", active: false, by_player: false, image: null, updated_at: t, body: "She tended the chapel before the fire. Nobody saw her leave.", objectives: [] },
      { id: "l1", title: "The Burning of the Ashen Vale", category: "lore", active: false, by_player: false, image: null, updated_at: t, body: "Forty years ago the valley burned in a single night.", objectives: [] },
    ],
    chat, tension: 6, pending_events: [], story_mode: false,
    story_choices: ["Loose Another Arrow", "Grab the Bell", "Hide Behind the Pews"],
    proposals: [{ id: "p1", actor_id: "aria", reason: "loot", changes: [{ op: "add_item", name: "Silver bell", qty: 1, note: "quest item" }, { op: "xp", amount: 50 }] }],
  };
}

let world: World | null = null;
let worlds: World[] = [sampleWorld(), { ...sampleWorld(), id: "salt", name: "Salt & Brass", tone: ["Swashbuckling"], session: 1, updated_at: now() - 6 * 86400000 },
  { ...sampleWorld(), id: "hollow", name: "The Hollow Crown of Vessingmoor-under-Lake", tone: ["Gothic"], session: 2, updated_at: now() - 20 * 86400000 }];
const trash = new Map<string, World>();
let settings: Settings = {
  gm: { claude_command: "claude", model: "", style: "vivid", secret_rolls: true, ask_before_sheet_changes: true },
  image: { base_url: "https://openrouter.ai/api/v1", api_key: "••••••••3f9c", model: "google/gemini-2.5-flash-image", api_style: "chat", art_style: "Painterly dark-fantasy illustration, cool moonlight and warm lantern light, ink outlines.", hero_consistency: true, gm_can_generate: true },
  appearance: { theme: "dark", reduce_motion: false, high_contrast: false, narration_px: 17, announce: true },
  rules: { fate: true, minions: true, oracle_events: true, lethal: false },
};
let busy = false;
let stopFlag = false;
let testCount = 0;

const changed = () => { if (world) { world.updated_at = now(); emit("world://changed", structuredClone(world)); } };
type NewEntry = ChatEntry extends infer E ? (E extends ChatEntry ? Omit<E, "id" | "ts"> : never) : never;
const push = (e: NewEntry) => { world!.chat.push({ id: id(), ts: now(), ...e } as ChatEntry); };
const pc = () => world!.actors.find((a) => a.kind === "pc")!;

function rollExpr(expr: string): { total: number; detail: string } {
  const m = /^\s*(\d*)d(\d+)\s*([+-]\s*\d+)?\s*$/i.exec(expr);
  if (!m) throw `“${expr}” isn’t a dice expression I can roll. Try something like 2d6+3.`;
  const count = Number(m[1] || 1), sides = Number(m[2]), k = Number((m[3] || "0").replace(/\s/g, ""));
  const dice = Array.from({ length: count }, () => d(sides));
  const total = dice.reduce((a, b) => a + b, 0) + k;
  return { total, detail: `${count}d${sides} (${dice.join(", ")})${k ? ` ${k > 0 ? "+" : "−"} ${Math.abs(k)}` : ""}` };
}

const SKILLS: Record<string, keyof Actor["abilities"]> = { acrobatics: "dex", animal_handling: "wis", arcana: "int", athletics: "str", deception: "cha", history: "int", insight: "wis", intimidation: "cha", investigation: "int", medicine: "wis", nature: "int", perception: "wis", performance: "cha", persuasion: "cha", religion: "int", sleight_of_hand: "dex", stealth: "dex", survival: "wis" };
const title = (s: string) => s.replace(/_/g, " ").replace(/\b\w/g, (c) => c.toUpperCase());

async function streamGm(text: string) {
  emit("gm://status", { state: "thinking", detail: "" });
  await new Promise((r) => setTimeout(r, 400));
  emit("gm://status", { state: "acting", detail: "get_state" });
  const reply = text.toLowerCase().includes("bell")
    ? "You dive for the bell as the goblin does. Your fingers close on cold silver first; it rings once, a sound far too loud for its size, and somewhere above you something answers."
    : "The goblin skids on the wet flagstones, looks at its fallen friend, then at you. It drops its scimitar and bolts for the stair behind the altar. It's your move.";
  for (const word of reply.split(/(\s+)/)) {
    if (stopFlag) break;
    emit("gm://delta", { text: word });
    await new Promise((r) => setTimeout(r, 18));
  }
  if (stopFlag) { push({ type: "system", text: "You stopped the GM mid-turn. Changes it already made to the table stay." }); }
  else { push({ type: "gm", text: reply }); world!.story_choices = ["Chase It Up the Stair", "Search the Altar", "Ring the Bell"]; }
  emit("gm://message-done", {});
  changed();
  emit("gm://status", { state: "idle", detail: "" });
  busy = false; stopFlag = false;
}

export async function mockInvoke(cmd: string, a: Record<string, unknown>): Promise<unknown> {
  const need = () => { if (!world) throw "No world is open. Open one from Your Worlds."; return world; };
  switch (cmd) {
    case "app_info": return { version: "0.1.0-mock", images_root: "", mcp_port: 0 };
    case "list_worlds": return worlds.map<WorldSummary>((w) => ({ id: w.id, name: w.name, genre: w.tone[0] ?? "Adventure", hero: `${w.actors[0].name}, level ${w.actors[0].level} ${w.actors[0].class}`, session: w.session, updated_at: w.updated_at, cover: null })).sort((x, y) => y.updated_at - x.updated_at);
    case "open_world": { world = structuredClone(worlds.find((w) => w.id === a.id) ?? worlds[0]); changed(); return world; }
    case "close_world": world = null; return null;
    case "get_world": return world;
    case "delete_world": { const w = worlds.find((x) => x.id === a.id); if (w) { trash.set(w.id, w); worlds = worlds.filter((x) => x.id !== a.id); } return null; }
    case "restore_world": { const w = trash.get(a.id as string); if (w) { worlds.push(w); trash.delete(w.id); } return null; }
    case "create_world": {
      const spec = a.spec as { name: string; pitch: string; tone: string[]; hero: { name: string; class: string; ancestry: string } };
      if (!spec.name?.trim()) throw "Give your world a name to continue, e.g. “The Drowned Marches”.";
      const w = sampleWorld();
      Object.assign(w, { id: id(), name: spec.name, pitch: spec.pitch, tone: spec.tone, session: 1, combat: null, proposals: [], journal: [], chat: [{ id: id(), ts: now(), type: "system", text: "World created. Press Begin Session Zero and the GM sets the first scene." }] });
      Object.assign(w.actors[0], { name: spec.hero.name, class: spec.hero.class, ancestry: spec.hero.ancestry, level: 1, xp: 0, fate: 3 });
      w.actors = [w.actors[0]]; w.scenes[0].tokens = [w.scenes[0].tokens[0]]; w.scenes[0].name = "Session Zero";
      worlds.push(w); world = structuredClone(w); changed(); return world;
    }
    case "send_message": {
      const w = need(); const text = String(a.text ?? "");
      if (text.trim()) push({ type: "player", text: text.trim() }); w.story_choices = []; changed();
      if (busy) return "queued";
      busy = true; void streamGm(text); return "started";
    }
    case "stop_gm": stopFlag = busy; return busy;
    case "gm_busy": return busy;
    case "roll": { need(); const r = rollExpr(String(a.expr)); const card: RollCard = { title: String(a.label || `Roll ${a.expr}`), actor: pc().name, lines: [{ label: String(a.label || "Roll"), detail: r.detail, total: r.total, kind: "plain" }], outcome: "none", note: "", by_gm: false }; push({ type: "roll", card }); changed(); return card; }
    case "check": {
      need(); const kind = String(a.kind).toLowerCase().replace(/\s/g, "_");
      const p = pc(); const isSave = kind.endsWith("_save"); const ab = (isSave ? kind.replace("_save", "") : SKILLS[kind] ?? kind) as keyof Actor["abilities"];
      const prof = isSave ? p.save_profs.includes(ab) : p.skill_profs.includes(kind);
      const bonus = mod(p.abilities[ab] ?? 10) + (prof ? 2 : 0); const nat = d(20);
      const label = isSave ? `${title(ab)} Save` : SKILLS[kind] ? `${title(kind)} Check` : `${title(kind)} Check`;
      const card: RollCard = { title: label, actor: p.name, lines: [{ label, detail: `1d20 (${nat}) ${bonus >= 0 ? "+" : "−"} ${Math.abs(bonus)}`, total: nat + bonus, kind: "check" }], outcome: nat === 20 ? "crit" : nat === 1 ? "fumble" : "none", note: "", by_gm: false };
      push({ type: "roll", card }); changed(); return card;
    }
    case "attack": {
      const w = need(); const p = pc(); const atk = p.attacks.find((x) => x.name === a.attack) ?? p.attacks[0];
      const t = w.actors.find((x) => x.id === a.target || x.name === a.target); if (!t) throw "Pick a target token first.";
      const nat = d(20), hit = nat === 20 || (nat !== 1 && nat + atk.to_hit >= t.ac);
      const lines = [{ label: "To Hit", detail: `1d20 (${nat}) + ${atk.to_hit} vs AC ${t.ac}`, total: nat + atk.to_hit, kind: "to_hit" }];
      let note = "";
      if (hit) { const r = rollExpr(nat === 20 ? atk.damage.replace(/^(\d*)d/, (_, c) => `${Number(c || 1) * 2}d`) : atk.damage); lines.push({ label: "Damage", detail: `${r.detail} ${atk.damage_type}`, total: r.total, kind: "damage" }); const before = t.hp; t.hp = t.minion ? 0 : Math.max(0, t.hp - r.total); note = `${t.name}: ${before} → ${t.hp} HP${t.hp === 0 ? " · Down" : ""}`; }
      const card: RollCard = { title: `${atk.name} → ${t.name}`, actor: p.name, lines, outcome: nat === 20 ? "crit" : nat === 1 ? "fumble" : hit ? "success" : "fail", note, by_gm: false };
      push({ type: "roll", card }); changed(); return card;
    }
    case "move_token": {
      const w = need(); const sc = w.scenes.find((s) => s.id === w.active_scene)!; const tk = sc.tokens.find((t) => t.id === a.tokenId);
      if (!tk) throw "That token isn’t on this map."; if (tk.actor_id !== pc().id) throw "You can only move your own token. Ask the GM to move others.";
      const x = Number(a.x), y = Number(a.y); if (x < 0 || y < 0 || x >= sc.width || y >= sc.height) throw "That square is off the map.";
      tk.x = x; tk.y = y;
      for (let yy = y - 6; yy <= y + 6; yy++) for (let xx = x - 6; xx <= x + 6; xx++) if (xx >= 0 && yy >= 0 && xx < sc.width && yy < sc.height && (xx - x) ** 2 + (yy - y) ** 2 <= 36) sc.revealed[yy * sc.width + xx] = true;
      changed(); return null;
    }
    case "end_turn": { const w = need(); if (!w.combat) throw "No combat is running."; const c = w.combat; do { c.turn = (c.turn + 1) % c.order.length; if (c.turn === 0) c.round++; } while ((w.actors.find((x) => x.id === c.order[c.turn].actor_id)?.hp ?? 0) <= 0); changed(); return null; }
    case "ask_oracle": { need(); const r = d(100); const th = ({ impossible: 10, unlikely: 30, likely: 70, certain: 90 } as Record<string, number>)[String(a.likelihood)] ?? 50; push({ type: "oracle", question: String(a.question), likelihood: title(String(a.likelihood || "even odds")), roll: r, answer: r <= th ? (r <= th / 5 ? "Yes, and…" : "Yes") : r > 100 - (100 - th) / 5 ? "No, and…" : r <= th + (100 - th) / 5 ? "No, but…" : "No", event: r % 11 === 0 && r < 100 ? "An old enemy acts — betray a secret" : null }); changed(); return null; }
    case "resolve_proposal": { const w = need(); const p = w.proposals.find((x) => x.id === a.id); w.proposals = w.proposals.filter((x) => x.id !== a.id); if (p && a.accept) for (const ch of p.changes) { if (ch.op === "add_item") pc().inventory.push({ name: ch.name, qty: ch.qty, note: ch.note }); if (ch.op === "xp") pc().xp += ch.amount; } changed(); return null; }
    case "rest": { need(); const p = pc(); if (a.long) { p.hp = p.max_hp; p.fate = 3; p.spell_slots = [...p.spell_slots_max]; } else p.hp = Math.min(p.max_hp, p.hp + Math.floor(p.max_hp / 4)); const msg = a.long ? `${p.name} takes a long rest: full HP, spell slots and 3 Fate.` : `${p.name} takes a short rest.`; push({ type: "system", text: msg }); changed(); return msg; }
    case "set_story_mode": need().story_mode = Boolean(a.on); changed(); return null;
    case "add_note": need().journal.push({ id: id(), title: String(a.title), category: "note", body: String(a.body), image: null, objectives: [], active: false, updated_at: now(), by_player: true }); changed(); return null;
    case "update_notes": pc().notes = String(a.notes); changed(); return null;
    case "get_settings": return structuredClone(settings);
    case "save_settings": settings = structuredClone(a.settings as Settings); return settings;
    case "test_image": { testCount++; await new Promise((r) => setTimeout(r, 600)); if (testCount % 2) throw "The image endpoint returned 401 Unauthorized. Paste a fresh API key in Settings › Image Endpoint, then try again."; return { bytes: 48213, ms: 6400 }; }
    case "check_claude": return "2.1.332 (Claude Code)";
    case "generate_image": { need(); if (a.kind === "art") { push({ type: "image", file: null, prompt: String(a.prompt), status: "pending", error: "" }); changed(); } return null; }
    case "get_compendium": return { monsters, items, spells };
    case "class_list": return [
      { name: "Fighter", hit_die: 10, blurb: "Armour, weapons, Second Wind.", solo: "Second Wind recharges on a short rest and also ends 1 condition." },
      { name: "Rogue", hit_die: 8, blurb: "Sneak Attack, skills, Cunning Action.", solo: "Sneak Attack works whenever the scene distracts your target." },
      { name: "Cleric", hit_die: 8, blurb: "Divine magic, healing, armour.", solo: "Healing Word on yourself for free, once per short rest." },
      { name: "Wizard", hit_die: 6, blurb: "Wide spellbook, rituals.", solo: "Arcane Ward from level 1: temp HP equal to 2 × level + INT." },
      { name: "Ranger", hit_die: 10, blurb: "Bows, tracking, Hunter’s Mark.", solo: "Favored Foe: +1d4 once per turn against your marked target." },
      { name: "Paladin", hit_die: 10, blurb: "Heavy armour, smites, auras.", solo: "Lay on Hands pool doubles and costs a bonus action." },
    ];
    default: throw `mock: unknown command ${cmd}`;
  }
}
export { sign };
