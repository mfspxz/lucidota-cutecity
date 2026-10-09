#!/usr/bin/env python3
"""Launch checklist runner (city-014). Exits 0 only if ALL checks green.

Automated checks run now; content/voice-pass gates report their real state
(PENDING/BLOCKED are honest, not failures of the runner — but they block launch).
"""
import sys, os, json, subprocess, glob

ROOT = os.path.expanduser("~/workspace/cutecity")
RUST = os.path.join(ROOT, "rust")
results = []

def check(name, ok, detail=""):
    results.append({"name": name, "ok": bool(ok), "detail": detail})
    print(("PASS " if ok else "FAIL ") + name + (f" — {detail}" if detail else ""))

def run(cmd, **kw):
    try:
        r = subprocess.run(cmd, capture_output=True, text=True, timeout=kw.get("timeout", 120))
        return r.returncode, r.stdout.strip(), r.stderr.strip()
    except Exception as e:
        return 99, "", str(e)

# 1. Routes
rc, out, _ = run([f"{RUST}/target/debug/cute-routes", "verify", "--base", "https://cutecity.cc"], timeout=180)
try:
    n_fail = json.loads(out)["failures"] if out else -1
except Exception:
    n_fail = -1
check("routes: 29/29 verify", rc == 0 and n_fail == 0, f"exit={rc} failures={n_fail}")

# 2. Admin gates are inside verify (already counted); check migration file exists
check("admin: audit migration staged",
      os.path.exists(f"{RUST}/crates/cute-cms/src/schema.rs"), "MIGRATION_002_ADMIN_AUDIT")

# 3. Feed/API artifacts
fj = os.path.join(ROOT, "staging/feed.json")
ok = False
detail = "missing"
if os.path.exists(fj):
    try:
        d = json.load(open(fj))
        ok = d.get("version") == "https://jsonfeed.org/version/1.1" and isinstance(d.get("items"), list)
        detail = f"{len(d.get('items', []))} items"
    except Exception as e:
        detail = str(e)
check("feed: feed.json valid JSON Feed 1.1", ok, detail)
v1ok = True
for f in glob.glob(os.path.join(ROOT, "staging/v1/*.json")):
    try:
        assert json.load(open(f))["version"] == "v1"
    except Exception:
        v1ok = False
check("api: v1/*.json envelopes", v1ok and len(glob.glob(os.path.join(ROOT, "staging/v1/*.json"))) == 4,
      f"{len(glob.glob(os.path.join(ROOT, 'staging/v1/*.json')))}/4 files")

# 8. SEO artifacts
og = glob.glob(os.path.join(ROOT, "staging/og/*.html"))
check("seo: og snippets staged", len(og) >= 7, f"{len(og)} files")
sm = os.path.join(ROOT, "staging/sitemap.xml")
check("seo: sitemap.xml present", os.path.exists(sm))

# 12. Backup
rc, out, _ = run([f"{RUST}/target/debug/cute-backup", "dump-schema",
                  "--snapshot", f"{ROOT}/d1-snapshot-2026-10-07.json"])
check("backup: dump-schema", rc == 0, out.splitlines()[-1] if out else "")
rc, out, _ = run([f"{RUST}/target/debug/cute-backup", "restore-check",
                  "--snapshot", f"{ROOT}/d1-snapshot-2026-10-07.json",
                  "--db", "/tmp/cutecity-launch-restore.db"])
check("backup: restore-check", rc == 0, "0 failures" if rc == 0 else out[-200:])

# 15. Voice pass gate — report real state (PENDING is honest)
areas = ["feed", "og", "saltspringslasher", "tricycles", "districts", "odin-map", "copy"]
vp_root = os.path.join(ROOT, "voice-pass")
for a in areas:
    mp = os.path.join(vp_root, a, "manifest.json")
    if not os.path.exists(mp):
        check(f"voice-pass: {a}", False, "not staged")
        continue
    m = json.load(open(mp))
    ok = bool(m.get("voice_passed")) and bool((m.get("passed_by") or "").strip())
    check(f"voice-pass: {a}", ok, "CLEARED" if ok else "awaiting Zackary")

fails = [r for r in results if not r["ok"]]
# Voice-pass pendings are expected pre-launch; separate them in the summary.
gate_blocks = [r for r in fails if not r["name"].startswith("voice-pass:")]
print(f"\n{len(results)-len(fails)}/{len(results)} checks green; "
      f"{len(fails)-len(gate_blocks)} voice-pass pending; {len(gate_blocks)} hard failures")
sys.exit(0 if not gate_blocks and not [f for f in fails if f["name"].startswith("voice-pass:")] else 1)
