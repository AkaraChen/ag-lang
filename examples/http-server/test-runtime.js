import { serve } from "@hono/node-server";
import { setup } from "./app.js";

const app = setup();

const server = serve({ fetch: app.fetch.bind(app), port: 9123 }, async (info) => {
  console.log(`Test server on port ${info.port}`);

  const results = [];
  let passed = 0;
  let failed = 0;

  async function test(name, fn) {
    try {
      await fn();
      results.push(`PASS: ${name}`);
      passed++;
    } catch (e) {
      results.push(`FAIL: ${name} — ${e.message}`);
      failed++;
    }
  }

  function assertEq(a, b, msg) {
    if (JSON.stringify(a) !== JSON.stringify(b))
      throw new Error(msg || `expected ${JSON.stringify(b)}, got ${JSON.stringify(a)}`);
  }

  const BASE = `http://localhost:${info.port}`;

  // 3.1 GET /
  await test("3.1 GET / returns server info", async () => {
    const res = await fetch(`${BASE}/`);
    assertEq(res.status, 200);
    const json = await res.json();
    assertEq(json.name, "AgentScript Example Server");
    assertEq(json.version, "0.1.0");
    assertEq(Array.isArray(json.endpoints), true);
    assertEq(json.endpoints.length, 4);
  });

  // 3.2 POST /echo object
  await test("3.2 POST /echo object", async () => {
    const body = { hello: "world", n: 42 };
    const res = await fetch(`${BASE}/echo`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(body),
    });
    assertEq(res.status, 200);
    assertEq(await res.json(), body);
  });

  // 3.3 POST /echo array
  await test("3.3 POST /echo array", async () => {
    const body = [1, "two", { three: 3 }];
    const res = await fetch(`${BASE}/echo`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(body),
    });
    assertEq(res.status, 200);
    assertEq(await res.json(), body);
  });

  // 3.4 POST /calc add
  await test("3.4 POST /calc add", async () => {
    const res = await fetch(`${BASE}/calc`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ op: "add", a: 10, b: 3 }),
    });
    assertEq((await res.json()).result, 13);
  });

  // 3.5 POST /calc subtract
  await test("3.5 POST /calc subtract", async () => {
    const res = await fetch(`${BASE}/calc`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ op: "subtract", a: 10, b: 3 }),
    });
    assertEq((await res.json()).result, 7);
  });

  // 3.6 POST /calc multiply
  await test("3.6 POST /calc multiply", async () => {
    const res = await fetch(`${BASE}/calc`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ op: "multiply", a: 4, b: 5 }),
    });
    assertEq((await res.json()).result, 20);
  });

  // 3.7 POST /calc divide
  await test("3.7 POST /calc divide", async () => {
    const res = await fetch(`${BASE}/calc`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ op: "divide", a: 15, b: 4 }),
    });
    assertEq((await res.json()).result, 3.75);
  });

  // 3.8 POST /calc divide by zero
  await test("3.8 POST /calc divide by zero", async () => {
    const res = await fetch(`${BASE}/calc`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ op: "divide", a: 10, b: 0 }),
    });
    assertEq((await res.json()).result, 0);
  });

  // 3.9 POST /calc unknown op
  await test("3.9 POST /calc unknown op", async () => {
    const res = await fetch(`${BASE}/calc`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ op: "modulo", a: 10, b: 3 }),
    });
    assertEq((await res.json()).result, 0);
  });

  // 3.10 GET /greet/Alice
  await test("3.10 GET /greet/Alice", async () => {
    const res = await fetch(`${BASE}/greet/Alice`);
    assertEq((await res.json()).message, "Hello, Alice!");
  });

  // 3.11 GET /greet/World
  await test("3.11 GET /greet/World", async () => {
    const res = await fetch(`${BASE}/greet/World`);
    assertEq((await res.json()).message, "Hello, World!");
  });

  // Print results
  console.log("\n--- Test Results ---");
  for (const r of results) console.log(r);
  console.log(`\n${passed} passed, ${failed} failed`);

  server.close();
  process.exit(failed > 0 ? 1 : 0);
});
