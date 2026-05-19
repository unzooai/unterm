// Edge endpoint at /api/agents/trust. Returns the *current* trusted-keys
// list (Ed25519 public keys + key ids + optional expiry) so Unterm clients
// can detect "your binary is older than the current signing key" without
// having to ship a new release for every key rotation.
//
// This is informational only — clients do NOT trust new keys learned from
// this endpoint. The hard trust anchor stays the baked-in trusted_keys.json
// inside the binary. We just surface a friendly "your build only knows
// keys X, Y; the current signing key is Z; upgrade Unterm" message.

interface Env {
  UNTERM_MANIFESTS?: KVNamespace;
}

const MAX_AGE = 3600; // 1h — keys rotate rarely.

export const onRequestGet: PagesFunction<Env> = async (ctx) => {
  if (!ctx.env.UNTERM_MANIFESTS) {
    return jsonResponse({ keys: [] }, 503);
  }
  const raw = await ctx.env.UNTERM_MANIFESTS.get("trusted_keys", "text");
  if (!raw) {
    return jsonResponse({ keys: [] }, 200);
  }
  return new Response(raw, {
    status: 200,
    headers: {
      "content-type": "application/json; charset=utf-8",
      "cache-control": `public, max-age=${MAX_AGE}, s-maxage=${MAX_AGE}`,
      "access-control-allow-origin": "*",
      "access-control-allow-methods": "GET",
    },
  });
};

function jsonResponse(body: unknown, status: number) {
  return new Response(JSON.stringify(body), {
    status,
    headers: {
      "content-type": "application/json; charset=utf-8",
      "cache-control": "public, max-age=30",
      "access-control-allow-origin": "*",
    },
  });
}
