// Tauri has no Node.js server, so the frontend is a static single-page app.
// We use @sveltejs/adapter-static with an index.html fallback (SPA mode) and
// disable SSR. Prerendering the shell keeps the first paint fully local — no
// server, no network. See https://v2.tauri.app/start/frontend/sveltekit/
export const ssr = false;
export const prerender = true;
