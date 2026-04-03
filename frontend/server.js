import express from "express";
import httpProxy from "http-proxy";
import { handler } from "./build/handler.js";

const app = express();

// Create proxy for API requests
const proxy = httpProxy.createProxyServer({
  target: "http://backend:8080",
  changeOrigin: true,
  ws: true,
});

// Error handling for proxy
proxy.on("error", (err, req, res) => {
  console.error("[PROXY_ERROR]", err.message);
  res.status(502).json({ error: "Backend service unavailable" });
});

// Proxy middleware that preserves the /api prefix
app.use((req, res, next) => {
  if (req.url.startsWith("/api")) {
    console.log(
      `[PROXY] Forwarding ${req.method} ${req.url} to http://backend:8080`,
    );
    proxy.web(req, res);
  } else {
    next();
  }
});

// SvelteKit handler for everything else
app.use(handler);

const PORT = process.env.PORT || 3000;
app.listen(PORT, "0.0.0.0", () => {
  console.log(`Server listening on http://0.0.0.0:${PORT}`);
  console.log(`Proxying /api to http://backend:8080`);
});
