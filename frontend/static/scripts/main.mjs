import { createTracker } from "./tracker.mjs";

const appRoot = document.getElementById("tana-app");
if (appRoot) {
  let storage = null;
  try {
    storage = window.localStorage;
  } catch {
    // Theme choice remains in memory when browser storage is unavailable.
  }
  const tracker = createTracker({ document, storage });
  void tracker.start();
}
