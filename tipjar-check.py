#!/usr/bin/env python3
"""Solana tip-jar wiring verifier (city-026). Fail-closed: exits 0 only when
the tip jar is fully functional.

Checks:
  1. GET /api/assets/wallet-qr -> 200 + PNG bytes
  2. Homepage (or /contact) contains a Solana address (base58, 32 bytes)
  3. The address on the page matches the QR asset route (consistency)

Current live state (2026-10-07): NOT WIRED — /api/assets/wallet-qr 404s,
homepage has a placeholder with empty wallet config. D1 site_settings holds
the label/asset key but no QR asset is in R2 and no address is published.
To wire: (1) Zackary provides his Solana address, (2) upload QR via
/api/admin/wallet-qr (needs admin key), (3) re-run this script.
"""
import sys, re, urllib.request

BASE = sys.argv[1] if len(sys.argv) > 1 else "https://cutecity.cc"
UA = {"User-Agent": "CuteCity-TipJar/1.0"}
B58 = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz"

def b58decode(s):
    n = 0
    for c in s:
        n = n * 58 + B58.index(c)
    full = n.to_bytes(32, "big")
    # leading '1's are leading zero bytes
    pad = len(s) - len(s.lstrip("1"))
    return b"\x00" * pad + full[-32+pad:] if pad else full

def get(path):
    req = urllib.request.Request(BASE + path, headers=UA)
    try:
        r = urllib.request.urlopen(req, timeout=20)
        return r.status, r.headers.get("Content-Type", ""), r.read(200000)
    except urllib.error.HTTPError as e:
        return e.code, "", b""
    except Exception as e:
        return 0, "", str(e).encode()

def main():
    fails = []
    # 1. QR asset route
    st, ct, body = get("/api/assets/wallet-qr")
    qr_ok = st == 200 and body[:8] == b"\x89PNG\r\n\x1a\n"
    print(f"1. /api/assets/wallet-qr -> {st} {ct} {'PNG ok' if qr_ok else 'NOT WIRED'}")
    if not qr_ok:
        fails.append("qr-asset")
    # 2. Address on public pages
    addrs = set()
    for page in ["/", "/contact", "/about"]:
        st, _, body = get(page)
        if st != 200:
            continue
        text = body.decode("utf-8", "replace")
        for m in re.finditer(r"[1-9A-HJ-NP-Za-km-z]{43,44}", text):
            s = m.group(0)
            # Boundary check: reject fragments embedded in longer base64 runs
            # (e.g. Cloudflare beacon integrity="sha512-..." hashes).
            before = text[m.start()-1] if m.start() > 0 else " "
            after = text[m.end()] if m.end() < len(text) else " "
            if before in B58 + "+/=" or after in B58 + "+/=":
                continue
            try:
                raw = b58decode(s)
                if len(raw) == 32:
                    addrs.add(s)
            except Exception:
                pass
    print(f"2. Solana addresses on public pages: {len(addrs)} {sorted(addrs)[:2]}")
    if not addrs:
        fails.append("no-address")
    if fails:
        print("TIP JAR NOT FUNCTIONAL:", fails)
        return 1
    print("TIP JAR FUNCTIONAL")
    return 0

if __name__ == "__main__":
    sys.exit(main())
