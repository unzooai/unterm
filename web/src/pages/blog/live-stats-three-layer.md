---
layout: ../../layouts/Doc.astro
title: Live ⭐ + ↓ on a static product site, three layers
subtitle: How to put a real-time GitHub star count and aggregate download count on a marketing page without a single point of failure. Three small layers, two specific footguns we've eaten before.
kicker: Blog / Live stats, three layers
date: 2026-05-02 · Reference impl in [github.com/unzooai/unterm/tree/master/web](https://github.com/unzooai/unterm/tree/master/web) · MIT
---

## Why naive doesn't work

You'd think you could just slap an `await fetch("https://api.github.com/repos/<you>/<repo>")` into your hero section and be done. Three reasons that breaks:

1. **Rate limit.** The unauth GitHub API is 60 requests per hour per IP. A modestly-trafficked landing page eats that in seconds — and the user behind a CGNAT'd home network sees `403 forbidden` on first paint while their neighbor's request was the 61st of the hour.
2. **CORS gotchas.** api.github.com sends sane CORS headers, but Safari Private mode, some corporate proxies, and content blockers strip them. The fetch errors out and your hero shows `undefined`.
3. **First-paint latency.** If the number arrives 600 ms after the page is interactive, social-media preview cards (Twitter, Slack, RSS) all see it blank. So do crawlers. So does the user on a 3G connection.

## The three layers

The architecture answers each failure mode with its own layer:

| Layer | What it solves | Falls back to |
|---|---|---|
| **1. Build-time SSR** | Numbers are baked into the HTML. Crawlers, OG cards, RSS, Reader Mode, JS-disabled all see them. | — |
| **2. Edge proxy `/api/stats`** | Visitors only ever talk to your domain. Each region's edge cache talks to GitHub once every 5 minutes, regardless of traffic. | SSR's stale numbers |
| **3. Client refresh** | A page deployed last week still shows today's numbers. The hero updates after first paint, no flicker. | SSR's stale numbers |

Each upper layer is independent — if Layer 2 is down, Layer 1's numbers are still on screen. If Layer 3 fails, Layer 1 (or 2) stays. There's no single point of failure in the path that ends with "user sees a blank chip".

## Layer 1 — Build-time SSR

Run during the static-site build. We're using Astro 4 here but the pattern is the same in Next/Nuxt/SvelteKit/Hugo — anywhere you can run code at build time and bake the result into HTML.

```ts
// web/src/lib/stats.ts
const REPO = "unzooai/unterm";

export interface Stats {
  // null means "no trustworthy answer yet" — render as em dash, not 0.
  // Distinguishing null vs 0 is load-bearing: a transient GitHub 503 must
  // not pin "⭐ 0 stars" onto the homepage for the next 5 minutes.
  stars: number | null;
  downloads: number | null;
  release: string;
}

let cache: Promise<Stats> | null = null;
export function fetchStats(): Promise<Stats> {
  if (!cache) cache = doFetch();
  return cache;
}

async function doFetch(): Promise<Stats> {
  try {
    const [r1, r2] = await Promise.all([
      fetch(`https://api.github.com/repos/${REPO}`),
      fetch(`https://api.github.com/repos/${REPO}/releases?per_page=100`),
    ]);
    if (!r1.ok || !r2.ok) return FALLBACK;
    const repo = await r1.json();
    const releases = await r2.json();
    const downloads = releases.reduce((s: number, r: any) =>
      s + r.assets.reduce((a: number, x: any) => a + (x.download_count ?? 0), 0), 0);
    return {
      stars: repo.stargazers_count ?? null,
      downloads: releases.length > 0 ? downloads : null,
      release: releases[0]?.tag_name ?? "v0.12.3",
    };
  } catch {
    return FALLBACK;
  }
}

const FALLBACK: Stats = { stars: null, downloads: null, release: "v0.12.3" };
```

Then in your Astro page frontmatter:

```astro
---
import { fetchStats, formatCount } from "../lib/stats";
const stats = await fetchStats();
---
<span data-stat="stars">{formatCount(stats.stars)}</span> stars
<span data-stat="downloads">{formatCount(stats.downloads)}</span> downloads
```

The `cache` module-level promise memoizes within a single build, so all 9 locale pages we generate share one network round-trip rather than hammering GitHub nine times. The `data-stat` attribute is the hook Layer 3 will use to update in place.

## Layer 2 — Edge proxy

A Cloudflare Pages Function (or Vercel Edge Function, or Netlify Edge Function) serves `/api/stats` from the same domain as the site. Browsers never call api.github.com directly, so:

- The unauth rate limit is consumed once per 5-minute window per region, not once per visitor.
- CORS is a non-issue — same origin.
- If you want to add an authenticated GitHub PAT to lift the rate limit, it lives as one env var on the edge — never in browser code.

```ts
// web/functions/api/stats.ts (Cloudflare Pages Function)
const SUCCESS_MAX_AGE = 300; // 5 min
const FAILURE_MAX_AGE = 30;  // 30 s — IMPORTANT, see footgun #2

export const onRequestGet: PagesFunction<{ GITHUB_TOKEN?: string }> = async (ctx) => {
  const cacheKey = new Request("https://internal/__stats_v1", { method: "GET" });
  const cache = caches.default;
  const cached = await cache.match(cacheKey);
  if (cached) return cached;

  const res = await buildResponse(ctx.env);
  ctx.waitUntil(cache.put(cacheKey, res.clone()));
  return res;
};

async function buildResponse(env): Promise<Response> {
  const headers: Record<string, string> = {
    "User-Agent": "your-site",
    Accept: "application/vnd.github+json",
    ...(env.GITHUB_TOKEN ? { Authorization: `Bearer ${env.GITHUB_TOKEN}` } : {}),
  };
  try {
    const [r1, r2] = await Promise.all([
      fetch("https://api.github.com/repos/<you>/<repo>", { headers,
        cf: { cacheTtl: SUCCESS_MAX_AGE, cacheEverything: true } }),
      fetch("https://api.github.com/repos/<you>/<repo>/releases?per_page=100",
        { headers, cf: { cacheTtl: SUCCESS_MAX_AGE, cacheEverything: true } }),
    ]);
    if (!r1.ok || !r2.ok) return failureResponse();
    // ... reduce downloads, build body ...
    return new Response(JSON.stringify(body), {
      status: 200,
      headers: {
        "content-type": "application/json",
        "cache-control": `public, max-age=${SUCCESS_MAX_AGE}, s-maxage=${SUCCESS_MAX_AGE}`,
      },
    });
  } catch {
    return failureResponse();
  }
}

function failureResponse(): Response {
  return new Response(
    JSON.stringify({ stars: null, downloads: null, release: null }),
    {
      status: 200,  // NOT 503 — see footgun #1
      headers: {
        "content-type": "application/json",
        "cache-control": `public, max-age=${FAILURE_MAX_AGE}, s-maxage=${FAILURE_MAX_AGE}`,
      },
    });
}
```

## Layer 3 — Client refresh

A tiny inline script that runs after page interactive, fetches `/api/stats`, and updates the chips in place — but only if the answer is `> 0`:

```js
(function () {
  function fmt(n) {
    if (n === null || !Number.isFinite(n)) return "—";
    if (n < 1000) return String(n);
    if (n < 10000) return (n / 1000).toFixed(1).replace(/\.0$/, "") + "k";
    return Math.round(n / 1000) + "k";
  }
  function paint(key, value) {
    if (typeof value !== "number" || value <= 0) return;  // FOOTGUN #1
    document.querySelectorAll('[data-stat="' + key + '"]')
      .forEach(el => el.textContent = fmt(value));
  }
  fetch("/api/stats")
    .then(r => r.ok ? r.json() : null)
    .then(d => { if (d) { paint("stars", d.stars); paint("downloads", d.downloads); } })
    .catch(() => {});
})();
```

## The two footguns we've eaten

### Footgun #1 — null vs 0 must be distinguished

On any failure path — Layer 1's build fetch, Layer 2's edge fetch — return `null`, not `0`. Then the client-side painter checks `> 0` rather than `!= null` before overwriting. Otherwise: a single GitHub 503 happens, your edge proxy returns `{stars: 0}`, that gets cached for 5 minutes across every region, and your homepage proudly displays "⭐ 0 stars · 0 downloads" while everyone wonders if you abandoned the project. The fix took 4 minutes; the trust took longer.

### Footgun #2 — failure responses need a short cache TTL

Success: cache for 300 seconds (or whatever your refresh budget is). Failure: cache for 30 seconds, or don't cache at all. A bad answer pinned to the edge for 5 minutes is far worse than re-asking too soon. The `SUCCESS_MAX_AGE` / `FAILURE_MAX_AGE` split in the Layer 2 snippet above does exactly this.

## Other framework adapters

The pattern's the same — build-time + edge proxy + client refresh — only the function signatures differ.

- **Next.js**: build-time = `getStaticProps` / Server Component. Edge proxy = `app/api/stats/route.ts` with `export const runtime = "edge"`.
- **Vercel**: drop the proxy file at `api/stats.ts` with `export const config = { runtime: "edge" }`. Caching via `Cache-Control` headers; Vercel respects them.
- **Netlify**: `netlify/edge-functions/stats.ts` with the same headers. `Deno.env` for the optional GitHub token.
- **SvelteKit**: `+server.ts` in the route folder, mark with `export const config = { runtime: "edge" }`.
- **Hugo / Jekyll / Eleventy**: only Layer 1 + Layer 3, no Layer 2 (these are pure static; you'd need a separate edge function on whatever CDN you front them with).

## One bonus pattern

A desktop app's auto-update check should hit your `/api/stats` (or a sibling `/api/latest-release`) for the same reason: don't make a million installs hammer api.github.com directly. Same edge cache, same rate limit savings. Unterm's own update poller does this.

---

Reference implementation lives in [github.com/unzooai/unterm/tree/master/web](https://github.com/unzooai/unterm/tree/master/web) — `functions/api/stats.ts` for Layer 2, `src/lib/stats.ts` for Layer 1, `src/layouts/Base.astro` for the inline Layer 3 script. MIT-licensed, copy what you need.
