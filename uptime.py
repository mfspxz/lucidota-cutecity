#!/usr/bin/env python3
"""Uptime monitor (city-022): cron-ready health probe -> ledger + status file.

Probes /api/health + / on cutecity.cc, records to the ledger on state CHANGE
(up/down) and always to the local status log. Alerts = ledger entries
(the ledger is the alert bus; Zackary reads it).

Install: */5 * * * * python3 ~/workspace/cutecity/uptime.py
"""
import sys, os, json, time, urllib.request, datetime

BASE = "https://cutecity.cc"
STATE = os.path.expanduser("~/workspace/cutecity/hidden/uptime-state.json")
LOG = os.path.expanduser("~/workspace/cutecity/hidden/uptime.log")
os.makedirs(os.path.dirname(STATE), exist_ok=True)

def probe(path, timeout=20):
    t0 = time.time()
    try:
        req = urllib.request.Request(BASE + path, headers={"User-Agent": "CuteCity-Uptime/1.0"})
        r = urllib.request.urlopen(req, timeout=timeout)
        body = r.read(2000).decode("utf-8", "replace")
        return {"ok": True, "status": r.status, "ms": int((time.time()-t0)*1000), "body": body[:200]}
    except Exception as e:
        return {"ok": False, "status": 0, "ms": int((time.time()-t0)*1000), "error": str(e)[:200]}

def ledger(summary, detail, verdict):
    os.system("python3 ~/ledger/ledger.py %s >/dev/null 2>&1" % json.dumps(json.dumps({
        "actor": "subagent:cutecity-builder", "device": "vm", "chat": "cron:uptime",
        "type": "observation", "summary": "uptime: " + summary, "detail": detail,
        "verdict": verdict, "refs": []})))

def main():
    now = datetime.datetime.now(datetime.timezone.utc).isoformat()
    health = probe("/api/health")
    root = probe("/")
    up = health["ok"] and health.get("status") == 200 and root["ok"] and root.get("status") == 200
    prev = {}
    if os.path.exists(STATE):
        try: prev = json.load(open(STATE))
        except Exception: pass
    prev_up = prev.get("up")
    json.dump({"ts": now, "up": up, "health": health, "root": root}, open(STATE, "w"))
    with open(LOG, "a") as f:
        f.write(json.dumps({"ts": now, "up": up, "health_ms": health["ms"], "root_ms": root["ms"]}) + "\n")
    if prev_up is not None and prev_up != up:
        if up:
            ledger("RECOVERED cutecity.cc", f"health={health} root={root}", "proven")
        else:
            ledger("DOWN cutecity.cc", f"health={health} root={root}", "proven")
        print("STATE CHANGE ->", "UP" if up else "DOWN")
    else:
        print("up" if up else "DOWN", f"(health {health['ms']}ms, root {root['ms']}ms)")
    return 0 if up else 1

if __name__ == "__main__":
    sys.exit(main())
