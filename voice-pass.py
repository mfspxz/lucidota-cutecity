#!/usr/bin/env python3
"""Voice-pass pipeline CLI (city-013). Fail-closed staging -> prod gate.

Commands:
  stage <area>                  copy staging/<area> (or named files) -> voice-pass/<area>/ + manifest
  approve <area> --by NAME [--note N]   record Zackary's approval
  reject <area> --note N        send back to draft with notes
  publish <area>                verify manifest + hashes; print deploy clearance (or REFUSE)
  status [<area>]               show pipeline state
"""
import sys, os, json, hashlib, shutil, datetime

ROOT = os.path.expanduser("~/workspace/cutecity")
STAGING = os.path.join(ROOT, "staging")
VP = os.path.join(ROOT, "voice-pass")

def sha256_file(p):
    h = hashlib.sha256()
    with open(p, "rb") as f:
        for chunk in iter(lambda: f.read(65536), b""):
            h.update(chunk)
    return "sha256:" + h.hexdigest()

def manifest_path(area):
    return os.path.join(VP, area, "manifest.json")

def load_manifest(area):
    p = manifest_path(area)
    if not os.path.exists(p):
        return None
    return json.load(open(p))

def ledger(summary, detail=""):
    os.system(
        "python3 ~/ledger/ledger.py %s >/dev/null 2>&1" % json.dumps(json.dumps({
            "actor": "subagent:cutecity-builder", "device": "vm", "chat": "direct",
            "type": "action", "summary": "voice-pass: " + summary, "detail": detail,
            "verdict": None, "refs": []}))
    )

def cmd_stage(area):
    src = os.path.join(STAGING, area)
    if not os.path.isdir(src):
        # allow staging single named groups: feed artifacts live at staging root
        print(f"no staging dir for area '{area}' (expected {src})", file=sys.stderr)
        return 2
    dst = os.path.join(VP, area)
    if os.path.exists(dst):
        shutil.rmtree(dst)
    shutil.copytree(src, dst)
    files = {}
    for dp, _, fns in os.walk(dst):
        for fn in fns:
            if fn == "manifest.json":
                continue
            fp = os.path.join(dp, fn)
            files[os.path.relpath(fp, dst)] = sha256_file(fp)
    man = {"area": area, "staged_at": datetime.datetime.now(datetime.timezone.utc).isoformat(),
           "files": files, "voice_passed": False, "passed_by": None,
           "passed_at": None, "note": None}
    json.dump(man, open(manifest_path(area), "w"), indent=1)
    ledger(f"staged area '{area}'", f"{len(files)} files, voice_passed=false")
    print(f"staged '{area}': {len(files)} files, awaiting voice pass")
    return 0

def cmd_approve(area, by, note):
    man = load_manifest(area)
    if not man:
        print(f"area '{area}' not staged", file=sys.stderr); return 2
    if not by or not by.strip():
        print("approver name required (--by)", file=sys.stderr); return 2
    man["voice_passed"] = True
    man["passed_by"] = by.strip()
    man["passed_at"] = datetime.datetime.now(datetime.timezone.utc).isoformat()
    man["note"] = note
    json.dump(man, open(manifest_path(area), "w"), indent=1)
    ledger(f"APPROVED area '{area}' by {by.strip()}", note or "")
    print(f"approved '{area}' by {by.strip()}")
    return 0

def cmd_reject(area, note):
    man = load_manifest(area)
    if not man:
        print(f"area '{area}' not staged", file=sys.stderr); return 2
    man["voice_passed"] = False
    man["passed_by"] = None
    man["note"] = "REJECTED: " + (note or "")
    json.dump(man, open(manifest_path(area), "w"), indent=1)
    ledger(f"REJECTED area '{area}'", note or "")
    print(f"rejected '{area}' -> back to draft")
    return 0

def cmd_publish(area):
    man = load_manifest(area)
    if not man:
        print(f"REFUSED: area '{area}' not staged", file=sys.stderr); return 3
    if not man.get("voice_passed") or not (man.get("passed_by") or "").strip():
        print(f"REFUSED: area '{area}' has no voice pass (voice_passed={man.get('voice_passed')}, by={man.get('passed_by')})",
              file=sys.stderr)
        ledger(f"publish REFUSED for '{area}' (no voice pass)", "")
        return 3
    dst = os.path.join(VP, area)
    for rel, want in man["files"].items():
        fp = os.path.join(dst, rel)
        if not os.path.exists(fp) or sha256_file(fp) != want:
            print(f"REFUSED: '{area}/{rel}' changed since staging (re-stage + re-approve)", file=sys.stderr)
            ledger(f"publish REFUSED for '{area}' (hash mismatch {rel})", "")
            return 3
    ledger(f"publish CLEARED for '{area}'", f"approved by {man['passed_by']} at {man['passed_at']}")
    print(f"CLEARED: '{area}' may publish (approved by {man['passed_by']})")
    print("deploy step: <parent's call — voice pass is the gate, not the deploy>")
    return 0

def cmd_status(area=None):
    areas = [area] if area else sorted(os.listdir(VP)) if os.path.isdir(VP) else []
    for a in areas:
        man = load_manifest(a)
        if not man:
            print(f"{a}: not staged"); continue
        st = "VOICE-PASSED" if man["voice_passed"] else "STAGED (awaiting pass)"
        print(f"{a}: {st} by={man['passed_by']} files={len(man['files'])}")
    return 0

def main():
    if len(sys.argv) < 2:
        print(__doc__); return 2
    cmd, area = sys.argv[1], (sys.argv[2] if len(sys.argv) > 2 and not sys.argv[2].startswith("--") else None)
    def flag(n):
        return next((sys.argv[i+1] for i in range(len(sys.argv)-1) if sys.argv[i] == n), None)
    if cmd == "stage" and area: return cmd_stage(area)
    if cmd == "approve" and area: return cmd_approve(area, flag("--by"), flag("--note"))
    if cmd == "reject" and area: return cmd_reject(area, flag("--note"))
    if cmd == "publish" and area: return cmd_publish(area)
    if cmd == "status": return cmd_status(area)
    print(__doc__); return 2

if __name__ == "__main__":
    sys.exit(main())
