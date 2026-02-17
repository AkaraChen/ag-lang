import { serve as honoServe } from "@hono/node-server";

export function serve(app, options = {}) {
  const port = options.port || 3000;

  const server = honoServe({
    fetch: app.fetch.bind(app),
    port,
  }, (info) => {
    console.log(`AgentScript server running at http://localhost:${info.port}`);
  });

  const shutdown = () => {
    server.close();
    process.exit(0);
  };

  process.on("SIGINT", shutdown);
  process.on("SIGTERM", shutdown);

  return server;
}
