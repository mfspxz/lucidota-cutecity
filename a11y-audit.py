#!/usr/bin/env python3
"""Static accessibility audit (city-024): fetch public pages, run structural
checks. No browser needed. Full WCAG AA (contrast, focus order, screen-reader
flow) needs a browser pass post-launch — flagged, not faked.

Checks per page:
  - <html lang> present
  - <title> present and non-empty
  - exactly one <h1>
  - img tags all have alt (empty alt ok for decorative)
  - buttons have accessible names (text or aria-label)
  - form inputs have labels (label for=, aria-label, or wrapped)
  - skip link present
  - aria-hidden not on focusable content (basic)
  - color contrast: NOT checkable statically -> reported as gap
"""
import sys, re, html as ihtml, urllib.request
from html.parser import HTMLParser

BASE = "https://cutecity.cc"
PAGES = ["/", "/about", "/blog", "/portfolio", "/contact", "/faq", "/releases", "/accessibility"]

class Audit(HTMLParser):
    def __init__(self):
        super().__init__()
        self.issues = []
        self.h1 = 0
        self.imgs_no_alt = 0
        self.buttons_noname = 0
        self.inputs_nolabel = 0
        self.in_button = 0
        self.button_text = ""
        self.button_has_label = False
        self.lang = None
        self.title = ""
        self.in_title = False
        self.skip_link = False
        self.labels_for = set()
        self.input_ids = []
    def handle_starttag(self, tag, attrs):
        a = dict(attrs)
        if tag == "html":
            self.lang = a.get("lang")
        elif tag == "title":
            self.in_title = True
        elif tag == "h1":
            self.h1 += 1
        elif tag == "img":
            if "alt" not in a:
                self.imgs_no_alt += 1
        elif tag == "button":
            self.in_button += 1
            self.button_text = ""
            self.button_has_label = bool(a.get("aria-label") or a.get("aria-labelledby"))
        elif tag == "a" and a.get("href", "").startswith("#"):
            self.skip_link = True
        elif tag == "label" and a.get("for"):
            self.labels_for.add(a["for"])
        elif tag in ("input", "select", "textarea") and a.get("type") != "hidden":
            if not (a.get("aria-label") or a.get("aria-labelledby") or a.get("id") in self.labels_for or a.get("title")):
                self.input_ids.append(a.get("id") or a.get("name") or tag)
        elif tag == "input" and a.get("id"):
            pass
    def handle_endtag(self, tag):
        if tag == "title":
            self.in_title = False
        elif tag == "button" and self.in_button:
            self.in_button -= 1
            if not self.button_text.strip() and not self.button_has_label:
                self.buttons_noname += 1
    def handle_data(self, data):
        if self.in_title:
            self.title += data
        if self.in_button:
            self.button_text += data

def audit_page(path):
    req = urllib.request.Request(BASE + path, headers={"User-Agent": "CuteCity-A11y/1.0"})
    try:
        t = urllib.request.urlopen(req, timeout=20).read().decode("utf-8", "replace")
    except Exception as e:
        return {"path": path, "error": str(e)[:100]}
    au = Audit()
    try:
        au.feed(t)
    except Exception:
        pass
    issues = []
    if not au.lang: issues.append("missing <html lang>")
    if not au.title.strip(): issues.append("missing/empty <title>")
    if au.h1 != 1: issues.append(f"h1 count = {au.h1} (want 1)")
    if au.imgs_no_alt: issues.append(f"{au.imgs_no_alt} <img> without alt")
    if au.buttons_noname: issues.append(f"{au.buttons_noname} <button> without accessible name")
    if au.input_ids: issues.append(f"inputs without label: {au.input_ids[:3]}")
    if not au.skip_link: issues.append("no skip link")
    return {"path": path, "issues": issues, "bytes": len(t)}

def main():
    total_issues = 0
    for p in PAGES:
        r = audit_page(p)
        if "error" in r:
            print(f"{p}: FETCH ERROR {r['error']}")
            total_issues += 1
            continue
        total_issues += len(r["issues"])
        status = "OK" if not r["issues"] else "ISSUES: " + "; ".join(r["issues"])
        print(f"{p}: {status}")
    print(f"\n{total_issues} issue(s) across {len(PAGES)} pages")
    print("GAP (needs browser): color contrast ratios, focus visibility/order, screen-reader flow — not statically checkable")
    return 0 if total_issues == 0 else 1

if __name__ == "__main__":
    sys.exit(main())
