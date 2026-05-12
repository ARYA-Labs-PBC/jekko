#!/usr/bin/env bun

const seed = Bun.argv.slice(2).join(" ").trim();

if (!seed) {
  console.error("usage: bun script/memory-benchmark-seed-commit.ts <public-label-or-private-seed>");
  process.exit(2);
}

const digest = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(seed));
const hex = Array.from(new Uint8Array(digest), (byte) => byte.toString(16).padStart(2, "0")).join("");

console.log(JSON.stringify({ algorithm: "sha256", commitment: hex }, null, 2));
