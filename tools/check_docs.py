#!/usr/bin/env python3
from pathlib import Path
import re, sys

ROOT = Path(__file__).resolve().parents[1]
ERRORS = []
WARNINGS = []

def err(msg): ERRORS.append(msg)
def warn(msg): WARNINGS.append(msg)

def text(path): return path.read_text(encoding="utf-8")

# 1. Every non-archive Markdown file should have an H1.
for f in ROOT.rglob("*.md"):
    if "archive" in f.parts:
        continue
    lines = text(f).splitlines()
    if not any(line.startswith("# ") for line in lines):
        err(f"{f.relative_to(ROOT)}: missing H1")

# 2. Code fences must be balanced.
for f in ROOT.rglob("*.md"):
    if text(f).count("```") % 2:
        err(f"{f.relative_to(ROOT)}: unbalanced code fences")

# 3. ADR integrity.
adr_dir = ROOT / "docs/adr"
adr_files = sorted(adr_dir.glob("[0-9][0-9][0-9][0-9]-*.md"))
ids = []
valid_status = {"Proposed","V1 Blocker","Accepted","Rejected","Superseded"}
for f in adr_files:
    t = text(f)
    fid = f.name[:4]
    ids.append(fid)
    h = re.search(r"^# ADR-(\d{4}):", t, re.M)
    if not h or h.group(1) != fid:
        err(f"{f.relative_to(ROOT)}: ADR H1 id does not match filename")
    sm = re.search(r"- \*\*Status:\*\* (.+)", t)
    status = sm.group(1).strip() if sm else None
    if status not in valid_status:
        err(f"{f.relative_to(ROOT)}: invalid/missing ADR status {status!r}")
    if status == "Accepted":
        for section in ("Decision","Evidence","Consequences"):
            m = re.search(rf"^## {section}\s*$([\s\S]*?)(?=^## |\Z)", t, re.M)
            if not m or not m.group(1).strip() or re.search(r"\bTBD\b", m.group(1), re.I):
                err(f"{f.relative_to(ROOT)}: Accepted ADR has incomplete {section}")
if len(ids) != len(set(ids)):
    err("duplicate ADR ids")

# 4. Traceability must reference every ADR, especially every V1 Blocker.
trace = text(ROOT/"TRACEABILITY.md")
trace_ids = set(re.findall(r"ADR-(\d{4})", trace))
all_ids = set(ids)
for missing in sorted(all_ids - trace_ids):
    err(f"TRACEABILITY.md: missing ADR-{missing}")

# 5. ADR index must match real files.
index = text(adr_dir/"ADR_INDEX.md")
index_ids = set(re.findall(r"ADR-(\d{4})", index))
if index_ids != all_ids:
    err(f"ADR_INDEX mismatch: missing={sorted(all_ids-index_ids)} extra={sorted(index_ids-all_ids)}")

# 6. Local Markdown links to .md files must resolve.
link_re = re.compile(r"\[[^\]]*\]\(([^)]+\.md(?:#[^)]+)?)\)")
for f in ROOT.rglob("*.md"):
    if "archive" in f.parts:
        continue
    for target in link_re.findall(text(f)):
        target_path = target.split("#",1)[0]
        if target_path.startswith(("http://","https://")):
            continue
        resolved = (f.parent / target_path).resolve()
        try:
            resolved.relative_to(ROOT.resolve())
        except ValueError:
            err(f"{f.relative_to(ROOT)}: link escapes documentation root: {target}")
            continue
        if not resolved.exists():
            err(f"{f.relative_to(ROOT)}: broken Markdown link: {target}")

# 7. Root-style textual paths like docs/foo.md should exist.
path_re = re.compile(r"(?<![\w/])((?:docs|tools)/[A-Za-z0-9_./-]+\.md)")
for f in ROOT.rglob("*.md"):
    if "archive" in f.parts:
        continue
    for p in path_re.findall(text(f)):
        if not (ROOT/p).exists():
            err(f"{f.relative_to(ROOT)}: stale documentation path: {p}")

# 8. Archive must be visibly non-normative.
archive = ROOT/"docs/archive/bevy_npr_framework_master_plan.md"
if archive.exists() and "ARCHIVED / NON-NORMATIVE" not in text(archive)[:1000]:
    err("archived Master Plan missing NON-NORMATIVE banner")

# 9. Forbidden legacy terminology outside archive.
forbidden = [
    "Stable Profile ABI",
    "bevy-stylized-profile-gacha",
    "NprQualityTier",
    "StylizedMaterialClass",
]
for f in ROOT.rglob("*.md"):
    if "archive" in f.parts:
        continue
    t = text(f)
    for term in forbidden:
        if term in t:
            err(f"{f.relative_to(ROOT)}: forbidden legacy term: {term}")

# 10. Required governance files.
required = [
    "ARCHITECTURE.md","GLOSSARY.md","TRACEABILITY.md","DOCS_METADATA.toml",
    "THIRD_PARTY_NOTICES.md","docs/adr/ADR_INDEX.md",
    "docs/quality/semantic-contract.md","docs/performance/targets.md",
    "docs/platforms/SUPPORT_MATRIX.md","docs/release/V1_RELEASE_CHECKLIST.md",
]
for p in required:
    if not (ROOT/p).exists():
        err(f"missing required file: {p}")

for w in WARNINGS:
    print("WARN:", w)
for e in ERRORS:
    print("ERROR:", e)
print(f"Checked {len(list(ROOT.rglob('*.md')))} Markdown files and {len(adr_files)} ADRs.")
if ERRORS:
    print(f"FAILED with {len(ERRORS)} error(s).")
    sys.exit(1)
print("PASS")
