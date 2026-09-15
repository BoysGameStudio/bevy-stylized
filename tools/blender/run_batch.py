#!/usr/bin/env python3
"""Batch runner for the Blender Reference Export Bridge.

Plain python3 (no bpy). Invokes Blender background mode once per input
.blend, collects per-file exit codes and manifests, and writes a batch
summary JSON. Exits non-zero if any input failed.

Usage:

    python3 run_batch.py --out-root <dir> [--blender blender] \
        [--contract-version blender-gltf-npr-v1] input1.blend [input2.blend ...]
"""

import argparse
import json
import os
import re
import subprocess
import sys
import time

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
EXPORT_SCRIPT = os.path.join(SCRIPT_DIR, "export_reference.py")


def slugify(path):
    base = os.path.splitext(os.path.basename(path))[0]
    return re.sub(r"[^A-Za-z0-9_-]", "_", base)


def parse_args():
    p = argparse.ArgumentParser(description="Batch Blender reference export")
    p.add_argument("inputs", nargs="+", help="Source .blend files")
    p.add_argument("--out-root", required=True, help="Root directory for per-file outputs")
    p.add_argument("--blender", default="blender", help="Blender executable")
    p.add_argument("--contract-version", default="blender-gltf-npr-v1")
    p.add_argument("--role", default="auto",
                   help="Artifact role; validated by the bridge against the signature contract")
    p.add_argument("--signature", default="expected-signature.genshin-v3.6.2.json",
                   help="Signature contract filename under contracts/")
    return p.parse_args()


def main():
    args = parse_args()
    os.makedirs(args.out_root, exist_ok=True)

    results = []
    failures = 0
    for src in args.inputs:
        src = os.path.abspath(src)
        slug = slugify(src)
        out_dir = os.path.join(args.out_root, slug)
        os.makedirs(out_dir, exist_ok=True)
        cmd = [
            args.blender, src,
            "--background", "--python-exit-code", "1",
            "--python", EXPORT_SCRIPT,
            "--", "--out", out_dir,
            "--contract-version", args.contract_version,
            "--role", args.role,
            "--signature", args.signature,
        ]
        print(f"[run-batch] exporting {src}", flush=True)
        started = time.time()
        proc = subprocess.run(cmd, capture_output=True, text=True)
        elapsed = time.time() - started

        manifest_path = os.path.join(out_dir, "export-manifest.json")
        manifest = None
        if os.path.exists(manifest_path):
            with open(manifest_path, "r", encoding="utf-8") as f:
                manifest = json.load(f)

        ok = proc.returncode == 0 and manifest is not None and manifest.get("status") == "ok"
        if not ok:
            failures += 1
        tail = (proc.stdout or "") + (proc.stderr or "")
        results.append({
            "input": src,
            "out_dir": out_dir,
            "exit_code": proc.returncode,
            "ok": ok,
            "elapsed_seconds": round(elapsed, 2),
            "manifest": manifest,
            "log_tail": tail.strip().splitlines()[-20:],
        })
        print(f"[run-batch] {'OK' if ok else 'FAILED'} ({proc.returncode}) {src}", flush=True)

    summary = {
        "batch_contract_version": args.contract_version,
        "blender": args.blender,
        "total": len(results),
        "failed": failures,
        "status": "ok" if failures == 0 else "failed",
        "results": results,
    }
    summary_path = os.path.join(args.out_root, "batch-summary.json")
    with open(summary_path, "w", encoding="utf-8") as f:
        json.dump(summary, f, indent=1)
        f.write("\n")
    print(f"[run-batch] summary: {summary_path} status={summary['status']}", flush=True)
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
