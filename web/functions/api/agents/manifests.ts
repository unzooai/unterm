// Edge proxy at /api/agents/manifests. Returns the signed manifest envelope
// that every Unterm install hits on startup to discover which AI-agent CLIs
// are available, how to install/auth/configure them, and how to bind them
// to identity profiles.
//
// Storage: Cloudflare KV namespace `UNTERM_MANIFESTS`, key `current`
// (the latest signed envelope) plus `archive:<unix-ts>` (history for
// rollback). Authoring lives outside this repo — the maintainer signs
// envelopes locally with `manifest-cli sign` and pushes via wrangler.
//
// Why a function and not a static file: lets the maintainer publish
// new manifests without a master push / Cloudflare Pages rebuild. We
// rev manifests far more often than the website. Also gives us a place
// to attach `If-None-Match` 304s so most installs send 1 HTTP request
// per day and get a 100-byte response.
//
// Security: the envelope is Ed25519-signed; the client has the trusted
// public key baked in and refuses unsigned/expired bodies. A compromise
// of this Pages Function (or our CF account) cannot escalate to remote
// code execution on user machines — the worst case is denial-of-service.
//
// Three guards copied from /api/stats.ts:
//   1. 503 fallback if KV miss / unbound — we'd rather the client use its
//      on-disk cache or baked fallback than serve garbage.
//   2. Short TTL on failures (30 s) so a transient KV outage doesn't pin
//      "no agents available" to the edge for 5 minutes.
//   3. CORS open + locked to GET.

interface Env {
  UNTERM_MANIFESTS?: KVNamespace;
}

const SUCCESS_MAX_AGE = 300; // 5 min — short enough for fresh maintainers,
const FAILURE_MAX_AGE = 30; //   long enough that 100k clients = ~5 KV reads/min
const SWR_MAX_AGE = 86400; // serve stale up to a day if KV is missing

export const onRequestGet: PagesFunction<Env> = async (ctx) => {
  const cacheKey = new Request("https://unterm.app/__agents_manifests_v1", {
    method: "GET",
  });
  const cache = caches.default;

  // Honour If-None-Match before paying for KV / cache read. Pre-check the
  // shared edge cache so identical requests with the same ETag short-circuit.
  const inm = ctx.request.headers.get("if-none-match");
  const cached = await cache.match(cacheKey);
  if (cached && inm && cached.headers.get("etag") === inm) {
    return new Response(null, {
      status: 304,
      headers: { etag: inm, "cache-control": cached.headers.get("cache-control") ?? "" },
    });
  }
  if (cached) {
    // If the client didn't send a matching ETag, hand back the cached body
    // but with the cached ETag still present so the next request short-circuits.
    return cached;
  }

  const res = await build(ctx.env);
  // Only put 200s in the cache. The 503 path is allowed to refetch immediately.
  if (res.status === 200) {
    ctx.waitUntil(cache.put(cacheKey, res.clone()));
  }
  return res;
};

async function build(env: Env): Promise<Response> {
  // KV namespace unbound = local-dev / first-deploy state. Surface a 503
  // explicitly; clients will fall through to their on-disk cache or baked
  // fallback rather than trust an empty body.
  if (!env.UNTERM_MANIFESTS) {
    return new Response("UNTERM_MANIFESTS KV not bound", {
      status: 503,
      headers: failureHeaders(),
    });
  }

  // KV `getWithMetadata` so we can stash the precomputed ETag next to the
  // value at publish time and avoid hashing the full envelope on every hit.
  // If a publisher didn't set metadata, fall back to a content-derived ETag.
  let envelope: ArrayBuffer | null = null;
  let etagFromMeta: string | null = null;
  try {
    const r = await env.UNTERM_MANIFESTS.getWithMetadata<{ etag?: string }>(
      "current",
      { type: "arrayBuffer" },
    );
    envelope = r.value;
    etagFromMeta = r.metadata?.etag ?? null;
  } catch (err) {
    console.warn("[manifests] KV read threw:", err);
  }

  if (!envelope) {
    return new Response("no signed envelope published yet", {
      status: 503,
      headers: failureHeaders(),
    });
  }

  const etag =
    etagFromMeta ??
    `"${await sha256Hex(envelope)}"`;

  return new Response(envelope, {
    status: 200,
    headers: {
      "content-type": "application/json; charset=utf-8",
      "cache-control": `public, max-age=${SUCCESS_MAX_AGE}, s-maxage=${SUCCESS_MAX_AGE}, stale-while-revalidate=${SWR_MAX_AGE}`,
      etag,
      "access-control-allow-origin": "*",
      "access-control-allow-methods": "GET",
      // Surface the signing key id in a response header for ops debugging
      // ("which key signed this envelope?"). Not security-sensitive — the
      // same id is also embedded in the JSON body's `signature.key_id`.
      // (Read it back out of the body if we ever stop emitting this.)
      "x-unterm-envelope-source": "cf-kv",
    },
  });
}

function failureHeaders(): Record<string, string> {
  return {
    "content-type": "text/plain; charset=utf-8",
    "cache-control": `public, max-age=${FAILURE_MAX_AGE}, s-maxage=${FAILURE_MAX_AGE}`,
    "access-control-allow-origin": "*",
    "access-control-allow-methods": "GET",
  };
}

async function sha256Hex(buf: ArrayBuffer): Promise<string> {
  const digest = await crypto.subtle.digest("SHA-256", buf);
  const bytes = new Uint8Array(digest);
  let s = "";
  for (let i = 0; i < bytes.length; i++) {
    s += bytes[i].toString(16).padStart(2, "0");
  }
  return s;
}
