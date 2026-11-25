// Mirrors crates/core/src/world/model.rs and settings.rs.

export type Ability = "str" | "dex" | "con" | "int" | "wis" | "cha";
export type Abilities = Record<Ability, number>;
export type ImageStatus = "none" | "pending" | "ready" | "failed";

export interface Attack { name: string; to_hit: number; damage: string; damage_type: string; range: string }
export interface Item { name: string; qty: number; note: string }
export interface Spell { name: string; level: number; summary: string }

export interface Actor {
  id: string; name: string;
  kind: "pc" | "companion" | "npc" | "monster";
  disposition: "friendly" | "neutral" | "hostile";
  level: number; class: string; ancestry: string; background: string;
  abilities: Abilities; save_profs: Ability[]; skill_profs: string[];
  max_hp: number; hp: number; temp_hp: number; ac: number; speed: number;
  attacks: Attack[]; spells: Spell[]; spell_slots: number[]; spell_slots_max: number[];
  concentrating: string | null; inventory: Item[]; gold: number; conditions: string[];
  fate: number; xp: number; hit_dice: number; portrait: string | null; appearance: string;
  minion: boolean; cr: string; traits: string[]; notes: string;
}

export interface Token { id: string; actor_id: string; x: number; y: number; hidden: boolean }
export interface Scene {
  id: string; name: string; description: string; width: number; height: number;
  background: string | null; map_status: ImageStatus; tokens: Token[]; fog: boolean; revealed: boolean[];
}
export interface Combatant { actor_id: string; initiative: number; hidden: boolean }
export interface Combat { round: number; turn: number; order: Combatant[]; name: string }
export interface Objective { text: string; done: boolean }
export interface JournalEntry {
  id: string; title: string; category: string; body: string; image: string | null;
  objectives: Objective[]; active: boolean; updated_at: number; by_player: boolean;
}
export type SheetChange =
  | { op: "add_item"; name: string; qty: number; note: string }
  | { op: "remove_item"; name: string; qty: number }
  | { op: "xp"; amount: number }
  | { op: "gold"; amount: number }
  | { op: "condition"; name: string; add: boolean };
export interface SheetProposal { id: string; actor_id: string; reason: string; changes: SheetChange[] }

export interface RollLine { label: string; detail: string; total: number; kind: string }
export interface RollCard { title: string; actor: string; lines: RollLine[]; outcome: string; note: string; by_gm: boolean }

export type ChatEntry = { id: string; ts: number } & (
  | { type: "gm"; text: string }
  | { type: "player"; text: string }
  | { type: "roll"; card: RollCard }
  | { type: "tool"; text: string }
  | { type: "image"; file: string | null; prompt: string; status: ImageStatus; error: string }
  | { type: "system"; text: string }
  | { type: "oracle"; question: string; likelihood: string; roll: number; answer: string; event: string | null }
);

export interface World {
  id: string; name: string; pitch: string; tone: string[]; rules_dial: string;
  created_at: number; updated_at: number; session: number; gm_session_id: string | null;
  actors: Actor[]; scenes: Scene[]; active_scene: string | null; combat: Combat | null;
  journal: JournalEntry[]; chat: ChatEntry[]; tension: number; pending_events: string[];
  proposals: SheetProposal[]; story_mode: boolean; story_choices: string[];
}

export interface WorldSummary { id: string; name: string; genre: string; hero: string; session: number; updated_at: number; cover: string | null }

export interface HeroSpec { name: string; ancestry: string; class: string; scores: Abilities; appearance: string; background: string }
export interface NewWorld { name: string; pitch: string; tone: string[]; rules_dial: string; hero: HeroSpec; companion: boolean }

export interface Settings {
  gm: { claude_command: string; model: string; style: string; secret_rolls: boolean; ask_before_sheet_changes: boolean };
  image: { base_url: string; api_key: string; model: string; api_style: "chat" | "images"; art_style: string; hero_consistency: boolean; gm_can_generate: boolean };
  appearance: { theme: "dark" | "light" | "system"; reduce_motion: boolean; high_contrast: boolean; narration_px: number; announce: boolean };
  rules: { fate: boolean; minions: boolean; oracle_events: boolean; lethal: boolean };
}

export interface MonsterEntry { id: string; name: string; kind: string; cr: string; ac: number; hp: number; hp_dice: string; speed: number; abilities: number[]; minion: boolean; attacks: Attack[]; traits: string[] }
export interface SimpleEntry { id: string; name: string; type?: string; level?: number | null; school?: string; summary: string; cost?: string }
export interface Compendium { monsters: MonsterEntry[]; items: SimpleEntry[]; spells: SimpleEntry[] }

export interface GmStatus { state: "idle" | "thinking" | "acting" | "error"; detail: string }
