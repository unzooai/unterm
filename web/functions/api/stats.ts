// Edge proxy at /api/stats. Browsers hit YOUR domain, never api.github.com.
//
// Why this exists, in one sentence: the homepage's star/download chips need
// to refresh more often than every Cloudflare deploy, but every visitor
// hitting the GitHub API directly burns the unauth rate limit (60/hr/IP)
// and CORS-fails on Safari Private. Pages Function in front, Cloudflare's
// per-region edge cache behind, GitHub gets contacted once per region per
// 5 minutes regardless of traffic.
//
// Two footguns this guards against (we've eaten them before):
//
//   1. null vs 0. On any failure path we return `null`, NOT 0. Clients
//      then check `> 0` rather than `!= null`, so a single GitHub 503
//      cannot pin "⭐ 0 · 0 downloads" onto the homepage for 5 minutes.
//
//   2. Failure cache TTL is short (30 s), not 300 s. A bad answer pinned
//      to the edge for 5 minutes is far worse than re-asking too soon.
//
// Cloudflare Pages Functions runtime gives us a Workers-compatible
// `caches.default` and the `cf` fetch options for upstream caching.

interface Env {
  // Optional: a fine-grained PAT lifts the unauth rate limit. Set in the
  // Cloudflare Pages dashboard → Settings → Environment variables.
  GITHUB_TOKEN?: string;
}

const REPO = "unzooai/unterm";
const SUCCESS_MAX_AGE = 300; // 5 min
const FAILURE_MAX_AGE = 30; // 30 s

interface StatsBody {
  stars: number | null;
  downloads: number | null;
  release: string | null;
  /** ISO timestamp the proxy answered, useful for debugging cache age. */
  fetched_at: string;
}

export const onRequestGet: PagesFunction<Env> = async (ctx) => {
  // Shared edge cache key — versioned so we can bust by deploying a new
  // function build if we ever change the response shape.
  const cacheKey = new Request("https://unterm.app/__stats_v1", {
    method: "GET",
  });
  const cache = caches.default;
  const cached = await cache.match(cacheKey);
  if (cached) return cached;

  const res = await buildResponse(ctx.env);

  // Only cache if status is "trustworthy" — a successful body or a freshly
  // null'd failure body — never partial / undefined shapes. Both are short
  // enough that a transient backend hiccup heals fast.
  ctx.waitUntil(cache.put(cacheKey, res.clone()));
  return res;
};

async function buildResponse(env: Env): Promise<Response> {
  const headers: Record<string, string> = {
    "User-Agent": "unterm-site-edge",
    Accept: "application/vnd.github+json",
  };
  if (env.GITHUB_TOKEN) headers.Authorization = `Bearer ${env.GITHUB_TOKEN}`;

  try {
    const [repoRes, relRes] = await Promise.all([
      fetch(`https://api.github.com/repos/${REPO}`, {
        headers,
        cf: { cacheTtl: SUCCESS_MAX_AGE, cacheEverything: true },
      }),
      fetch(`https://api.github.com/repos/${REPO}/releases?per_page=100`, {
        headers,
        cf: { cacheTtl: SUCCESS_MAX_AGE, cacheEverything: true },
      }),
    ]);

    if (!repoRes.ok || !relRes.ok) {
      console.warn(
        `[stats] github non-ok: repo=${repoRes.status} releases=${relRes.status}`,
      );
      return failureResponse();
    }

    const repo: any = await repoRes.json();
    const releases: any[] = await relRes.json();

    const downloads = Array.isArray(releases)
      ? releases.reduce(
          (sum, r) =>
            sum +
            (Array.isArray(r.assets)
              ? r.assets.reduce(
                  (s: number, a: any) => s + (a.download_count ?? 0),
                  0,
                )
              : 0),
          0,
        )
      : 0;

    const body: StatsBody = {
      stars: typeof repo.stargazers_count === "number" ? repo.stargazers_count : null,
      // Treat zero releases as `null` rather than `0` so the chip still
      // SSR-fallbacks instead of confidently saying "0 downloads."
      downloads: releases.length > 0 ? downloads : null,
      release: releases?.[0]?.tag_name ?? null,
      fetched_at: new Date().toISOString(),
    };
    return new Response(JSON.stringify(body), {
      status: 200,
      headers: {
        "content-type": "application/json; charset=utf-8",
        "cache-control": `public, max-age=${SUCCESS_MAX_AGE}, s-maxage=${SUCCESS_MAX_AGE}`,
        // CORS: only the unterm.app origin needs this; browsers that pre-flight
        // will accept either way. Locked to GET for safety.
        "access-control-allow-origin": "*",
        "access-control-allow-methods": "GET",
      },
    });
  } catch (err) {
    console.warn("[stats] fetch threw:", err);
    return failureResponse();
  }
}

function failureResponse(): Response {
  const body: StatsBody = {
    stars: null,
    downloads: null,
    release: null,
    fetched_at: new Date().toISOString(),
  };
  return new Response(JSON.stringify(body), {
    // 200 not 503 on purpose: clients should always read the body to
    // honor the null/null contract, not branch on HTTP status.
    status: 200,
    headers: {
      "content-type": "application/json; charset=utf-8",
      "cache-control": `public, max-age=${FAILURE_MAX_AGE}, s-maxage=${FAILURE_MAX_AGE}`,
      "access-control-allow-origin": "*",
      "access-control-allow-methods": "GET",
    },
  });
}
