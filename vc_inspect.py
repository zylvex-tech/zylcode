import sqlite3, os, hashlib

cands = [
    os.path.join(os.path.expanduser("~"), ".zylcode", "vector_cache.db"),
    r"C:\Projects\zylcode\vector_cache.db",
    r"C:\Projects\zylcode\.zylcode\vector_cache.db",
]
for c in cands:
    exists = os.path.exists(c)
    print(f"{'FOUND' if exists else 'absent'}: {c}")
    if exists:
        con = sqlite3.connect(c)
        cur = con.cursor()
        rows = cur.execute(
            "SELECT prompt_hash, prompt_text, response_text, created_at FROM vector_cache ORDER BY created_at DESC"
        ).fetchall()
        print(f"  rows={len(rows)}")
        for h, p, r, ts in rows:
            sp = hashlib.sha256((p or "").encode()).hexdigest()
            print(f"  --- {ts}")
            print(f"    stored_hash={h[:16]}  sha256(prompt)={sp[:16]}")
            print(f"    prompt_len={len(p or '')}")
            print(f"    prompt: {(p or '')[:300]!r}")
            print(f"    response_len={len(r or '')}")
            print(f"    response: {(r or '')[:600]!r}")
        con.close()