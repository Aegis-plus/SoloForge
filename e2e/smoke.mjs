// End-to-end smoke test of the UI against the in-browser mock backend.
// Run: npm run e2e   (starts Vite on :1420, drives Chromium with Playwright)
import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { createRequire } from "node:module";

const require = createRequire(import.meta.url);
const { chromium } = require("playwright");
const vite = spawn("npx", ["vite", "--port", "1421", "--strictPort"], { stdio: "pipe" });
const URL = "http://127.0.0.1:1421/";

async function waitForServer() {
  for (let i = 0; i < 60; i++) {
    try { if ((await fetch(URL)).ok) return; } catch {}
    await new Promise((r) => setTimeout(r, 250));
  }
  throw new Error("vite did not start");
}

let failed = false;
const step = async (name, fn) => {
  try { await fn(); console.log(`✓ ${name}`); }
  catch (e) { failed = true; console.error(`✕ ${name}\n  ${e.message}`); }
};

try {
  await waitForServer();
  const browser = await chromium.launch();
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
  const errors = [];
  page.on("pageerror", (e) => errors.push(e.message));
  await page.goto(URL);

  await step("launcher lists worlds and deletes with undo", async () => {
    const cards = page.getByRole("button", { name: "Resume Play" });
    await cards.first().waitFor();
    const before = await cards.count();
    await page.getByRole("button", { name: /Delete Salt & Brass/ }).click();
    await page.getByRole("button", { name: "Delete World" }).click();
    await page.getByText("Deleted Salt & Brass.").waitFor();
    assert.equal(await cards.count(), before - 1);
    await page.getByRole("button", { name: "Undo" }).click();
    await page.waitForTimeout(200);
    assert.equal(await cards.count(), before);
  });

  await step("keyboard-only token move", async () => {
    await page.getByRole("button", { name: "Resume Play" }).first().click();
    const token = page.getByRole("button", { name: /Aria Thorne, your token/ });
    await token.focus();
    await page.keyboard.press("ArrowRight");
    await page.keyboard.press("ArrowRight");
    await page.getByRole("button", { name: /Move Here · 10\s+ft/ }).waitFor();
    await page.keyboard.press("Enter");
    await page.waitForTimeout(150);
    const left = await token.evaluate((el) => parseInt(el.style.left, 10));
    assert.equal(left, 6 * 40 + 4);
  });

  await step("character drawer opens, rolls a skill, closes with Esc", async () => {
    await page.getByRole("button", { name: /Open Character Sheet/ }).click();
    await page.getByRole("dialog", { name: "Aria Thorne" }).waitFor();
    await page.getByRole("tab", { name: "Skills" }).click();
    await page.getByRole("button", { name: /Roll Stealth/ }).click();
    await page.keyboard.press("Escape");
    await page.getByRole("article", { name: /Stealth Check/ }).last().waitFor();
  });

  await step("GM reply streams into the chat log", async () => {
    await page.getByRole("textbox", { name: "Message to the GM" }).fill("I grab the bell");
    await page.keyboard.press("Enter");
    await page.getByRole("button", { name: "Stop the GM" }).waitFor();
    await page.getByRole("log").getByText(/You dive for the bell/).first().waitFor({ timeout: 8000 });
    await page.getByRole("button", { name: "Stop the GM" }).waitFor({ state: "detached", timeout: 8000 });
  });

  await step("story mode and back", async () => {
    await page.getByRole("button", { name: "Story Mode" }).click();
    await page.getByRole("button", { name: "Back to Map" }).click();
    await page.getByRole("region", { name: "Hotbar" }).waitFor();
  });

  await step("settings shows an actionable error, then success", async () => {
    await page.getByRole("button", { name: "Settings" }).first().click();
    await page.getByRole("button", { name: "Test Connection" }).click();
    await page.getByText(/Paste a fresh API key/).waitFor();
    await page.getByRole("button", { name: "Test Connection" }).click();
    await page.getByText("✓ Test Image Received").waitFor();
  });

  await step("wizard blocks an empty world name and focuses it", async () => {
    await page.getByRole("button", { name: /Back/ }).first().click();
    await page.getByRole("button", { name: "SoloForge" }).first().click();
    await page.getByRole("button", { name: "New World" }).click();
    await page.getByRole("button", { name: "Next: Ancestry" }).click();
    await page.getByRole("alert").getByText(/Give your world a name/).waitFor();
    assert.equal(await page.evaluate(() => document.activeElement?.id), "wname");
  });

  assert.deepEqual(errors, [], `page errors: ${errors.join("; ")}`);
  await browser.close();
} catch (e) {
  failed = true;
  console.error(e);
} finally {
  vite.kill();
}
process.exit(failed ? 1 : 0);
