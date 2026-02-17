#!/usr/bin/env node

import { execSync } from "node:child_process";
import { existsSync, watch } from "node:fs";
import { resolve, dirname } from "node:path";
import { serve } from "./index.js";

const args = process.argv.slice(2);
let entryFile = null;
let port = 3000;
let devMode = false;

for (let i = 0; i < args.length; i++) {
  if (args[i] === "--port" && i + 1 < args.length) {
    port = parseInt(args[i + 1], 10);
    i++;
  } else if (args[i] === "--dev") {
    devMode = true;
  } else if (!args[i].startsWith("-")) {
    entryFile = args[i];
  }
}

if (!entryFile) {
  console.error("Usage: agentscript-serve <entry.ag> [--port <port>] [--dev]");
  process.exit(1);
}

if (!existsSync(entryFile)) {
  console.error(`Error: file not found: ${entryFile}`);
  process.exit(1);
}

const outputFile = entryFile.replace(/\.ag$/, ".js");

function compile() {
  try {
    execSync(`asc build ${entryFile} -o ${outputFile}`, { stdio: "pipe" });
    return true;
  } catch (err) {
    console.error(err.stderr?.toString() || "Compilation failed");
    return false;
  }
}

async function loadAndServe() {
  const fullPath = resolve(outputFile);
  // Bust module cache for dev mode reloads
  const modulePath = `file://${fullPath}?t=${Date.now()}`;
  const mod = await import(modulePath);

  if (!mod.default || typeof mod.default.fetch !== "function") {
    console.error("Error: No App instance found as default export");
    process.exit(1);
  }

  return serve(mod.default, { port });
}

// Initial compile + start
if (!compile()) {
  process.exit(1);
}

let server = await loadAndServe();

if (devMode) {
  const dir = dirname(resolve(entryFile));
  let restarting = false;

  watch(dir, { recursive: true }, async (event, filename) => {
    if (!filename?.endsWith(".ag") || restarting) return;
    restarting = true;

    console.log(`\nFile changed: ${filename}, recompiling...`);

    if (compile()) {
      try {
        server.close();
        server = await loadAndServe();
      } catch (err) {
        console.error("Restart failed:", err.message);
      }
    }

    restarting = false;
  });
}
