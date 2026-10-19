// ---------------------------------------------------------------------------
// ZylCode public site worker — the Level-0 subscriber surface.
//
// Serves the landing page through env.ASSETS and owns exactly two API routes:
//
//   POST /api/waitlist    { email, note? }  → joins the Developer Preview
//                                             waitlist (D1, PK on email)
//   GET  /api/waitlist/count                → public social-proof count
//
// Governance notes (docs/governance/ZYLCODE_COMMERCIAL_MODEL.md §4):
// this is Level 0 — Public Development: a pre-release waitlist with
// status-labelled capabilities, NO prices and no commercial offer. Emails
// never leave D1; nothing here claims acceptance of any capability.
//
// Honesty rules mirrored from the product: invalid input gets a real
// error message (400), a duplicate join says so instead of pretending to be
// a first join, and an infrastructure failure returns an error — the client
// never renders "joined" unless this handler actually inserted a row.
// ---------------------------------------------------------------------------

const EMAIL_RE = /^[^\s@]{1,64}@[^\s@]+\.[^\s@]{2,}$/;
const MAX_NOTE = 200;

const DDL = `CREATE TABLE IF NOT EXISTS waitlist (
  email TEXT PRIMARY KEY,
  note TEXT,
  created_at TEXT NOT NULL,
  source TEXT NOT NULL
)`;

// Best-effort, isolate-local rate limit (5 joins / 10 minutes / IP). A Map
// in one isolate is not a global guarantee — it raises the cost of naive
// abuse and is documented as exactly that, not more.
const HITS = new Map();
const WINDOW_MS = 10 * 60 * 1000;
const MAX_HITS = 5;

function rateLimited(ip) {
  const now = Date.now();
  const list = (HITS.get(ip) || []).filter((t) => now - t < WINDOW_MS);
  if (list.length >= MAX_HITS) {
    HITS.set(ip, list);
    return true;
  }
  list.push(now);
  HITS.set(ip, list);
  return false;
}

function json(body, status = 200) {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json", "cache-control": "no-store" },
  });
}

async function handleWaitlist(request, env) {
  const ip =
    request.headers.get("cf-connecting-ip") ||
    request.headers.get("x-forwarded-for") ||
    "unknown";
  if (rateLimited(ip.split(",")[0].trim())) {
    return json(
      { error: "too many attempts from this address — try again in a few minutes" },
      429,
    );
  }

  let body;
  try {
    body = await request.json();
  } catch {
    return json({ error: "send a JSON body: { email, note? }" }, 400);
  }

  const email = String(body?.email ?? "").trim().toLowerCase();
  if (!EMAIL_RE.test(email)) {
    return json({ error: "that does not look like an email address" }, 400);
  }
  const note =
    typeof body?.note === "string" ? body.note.trim().slice(0, MAX_NOTE) : "";
  if (typeof body?.note === "string" && body.note.trim().length > MAX_NOTE) {
    return json({ error: `keep the note under ${MAX_NOTE} characters` }, 400);
  }

  const db = env.DB;
  await db.prepare(DDL).run();
  const inserted = await db
    .prepare(
      "INSERT OR IGNORE INTO waitlist (email, note, created_at, source) VALUES (?1, ?2, ?3, ?4)",
    )
    .bind(email, note || null, new Date().toISOString(), "landing")
    .run();

  const already = !(inserted.meta?.changes > 0);
  return json({ ok: true, already });
}

export default {
  async fetch(request, env) {
    const url = new URL(request.url);

    try {
      if (url.pathname === "/api/waitlist" && request.method === "POST") {
        return await handleWaitlist(request, env);
      }

      if (url.pathname === "/api/waitlist/purge" && request.method === "POST") {
        // Ops only: clears test rows and honours deletion requests.
        // Gated on a server-side Sites secret — rejected before the table is
        // touched when the secret is unset or the header does not match.
        const token = env.WAITLIST_ADMIN_TOKEN;
        const given = request.headers.get("x-admin-token") || "";
        if (!token || given.length !== token.length || given !== token) {
          return json({ error: "forbidden" }, 403);
        }
        await env.DB.prepare(DDL).run();
        await env.DB.prepare("DELETE FROM waitlist").run();
        return json({ ok: true });
      }

      if (url.pathname === "/api/waitlist/count") {
        await env.DB.prepare(DDL).run();
        const row = await env.DB.prepare(
          "SELECT COUNT(*) AS n FROM waitlist",
        ).first();
        return json({ count: Number(row?.n ?? 0) });
      }

      if (url.pathname.startsWith("/api/")) {
        return json({ error: "not found" }, 404);
      }

      return env.ASSETS.fetch(request);
    } catch (e) {
      // A failure here is a failure: the client shows the message, never a
      // fabricated success.
      return json({ error: "the waitlist service failed — try again shortly" }, 500);
    }
  },
};
