#!/usr/bin/env python3
"""Compare two golden PNG captures with a tolerance policy.

Runs inside Blender background mode (bpy.data.images is the PNG codec, no
third-party deps):

    blender --background --factory-startup \
        --python tools/blender-reference-export/compare_golden.py -- \
        <actual.png> <expected.png> [--json <report.json>] \
        [--max-delta 0.007843] [--mean-delta 0.001961] [--psnr-floor 48.0]

Tolerance defaults implement the "same-toolchain reproduction" tier of
docs/reference/08-golden-tolerance-and-cross-layer-criteria.md:
per-pixel max abs delta <= 2/255, mean abs delta <= 0.5/255, PSNR >= 48 dB.

Exit 0 = within tolerance; 1 = outside tolerance or comparison failed.
"""

import argparse
import json
import math
import sys

import bpy

DEFAULT_MAX_DELTA = 2.0 / 255.0
DEFAULT_MEAN_DELTA = 0.5 / 255.0
DEFAULT_PSNR_FLOOR = 48.0


def parse_args():
    argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    p = argparse.ArgumentParser(description="Golden image comparison")
    p.add_argument("actual")
    p.add_argument("expected")
    p.add_argument("--json", default=None)
    p.add_argument("--max-delta", type=float, default=DEFAULT_MAX_DELTA)
    p.add_argument("--mean-delta", type=float, default=DEFAULT_MEAN_DELTA)
    p.add_argument("--psnr-floor", type=float, default=DEFAULT_PSNR_FLOOR)
    return p.parse_args(argv)


def main():
    args = parse_args()
    actual = bpy.data.images.load(args.actual)
    expected = bpy.data.images.load(args.expected)
    if actual.size[:] != expected.size[:]:
        print(f"[compare-golden] FAIL size mismatch {tuple(actual.size)} != {tuple(expected.size)}")
        return 1
    w, h = actual.size
    a = actual.pixels[:]
    b = expected.pixels[:]

    n = w * h
    max_delta = 0.0
    sum_delta = 0.0
    sum_sq = 0.0
    channels = 4
    count = n * channels
    for i in range(count):
        d = abs(a[i] - b[i])
        if d > max_delta:
            max_delta = d
        sum_delta += d
        sum_sq += d * d
    mean_delta = sum_delta / count
    mse = sum_sq / count
    psnr = float("inf") if mse == 0.0 else 10.0 * math.log10(1.0 / mse)

    ok = (max_delta <= args.max_delta
          and mean_delta <= args.mean_delta
          and psnr >= args.psnr_floor)
    report = {
        "actual": args.actual,
        "expected": args.expected,
        "size": [w, h],
        "max_delta": max_delta,
        "mean_delta": mean_delta,
        "psnr_db": psnr if math.isfinite(psnr) else None,
        "identical": mse == 0.0,
        "tolerance": {
            "max_delta": args.max_delta,
            "mean_delta": args.mean_delta,
            "psnr_floor_db": args.psnr_floor,
        },
        "status": "ok" if ok else "mismatch",
    }
    if args.json:
        with open(args.json, "w", encoding="utf-8") as f:
            json.dump(report, f, indent=1)
            f.write("\n")
    print(f"[compare-golden] {'OK' if ok else 'MISMATCH'} "
          f"max={max_delta:.6f} mean={mean_delta:.6f} "
          f"psnr={'inf' if not math.isfinite(psnr) else f'{psnr:.2f}dB'} "
          f"identical={report['identical']}")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
