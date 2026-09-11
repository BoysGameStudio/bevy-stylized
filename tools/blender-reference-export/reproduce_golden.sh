#!/usr/bin/env bash
# Reproduce the It-1b upstream golden baseline and diff against the stored
# captures. Per docs/roadmap/02-iteration-plan.md §3 It-1b deliverable 4,
# "clean environment" is concretized as: same machine, freshly spawned
# process of the locked Blender version, scene rebuilt from the committed
# generator script.
#
# Usage: tools/blender-reference-export/reproduce_golden.sh
# Exit: 0 = all cases within tolerance (see
# docs/reference/08-golden-tolerance-and-cross-layer-criteria.md);
# 1 = mismatch or hard failure; 2 = a case was blocked at render time.
set -u

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
BLENDER="${BLENDER:-/snap/bin/blender}"
TOOLS="$REPO_ROOT/tools/blender-reference-export"
GOLDEN_DIR="$REPO_ROOT/fixtures/golden-local/upstream"
WORK="$REPO_ROOT/target/tmp/golden-repro"
CASES="pbr_control gacha_upstream"

rm -rf "$WORK"
mkdir -p "$WORK/rendered"

echo "[reproduce] step 1/3: rebuild fixture scene (fresh blender process)"
"$BLENDER" --background --factory-startup \
    --python "$TOOLS/make_reference_scene.py" -- \
    --out "$WORK/reference_scene.blend" || { echo "[reproduce] scene build FAILED"; exit 1; }

echo "[reproduce] step 2/3: re-render golden cases"
"$BLENDER" --background --factory-startup \
    --python "$TOOLS/render_golden.py" -- \
    --blend "$WORK/reference_scene.blend" \
    --out-dir "$WORK/rendered"
render_rc=$?
if [ "$render_rc" -eq 1 ]; then
    echo "[reproduce] render FAILED"
    exit 1
fi

echo "[reproduce] step 3/3: compare against stored goldens in $GOLDEN_DIR"
status=0
for case in $CASES; do
    if [ ! -f "$GOLDEN_DIR/$case.png" ]; then
        echo "[reproduce] MISSING stored golden: $GOLDEN_DIR/$case.png"
        status=1
        continue
    fi
    if [ ! -f "$WORK/rendered/$case.png" ]; then
        echo "[reproduce] case $case not re-rendered (blocked?); see $WORK/rendered"
        [ "$render_rc" -eq 2 ] && status=2 || status=1
        continue
    fi
    "$BLENDER" --background --factory-startup \
        --python "$TOOLS/compare_golden.py" -- \
        "$WORK/rendered/$case.png" "$GOLDEN_DIR/$case.png" \
        --json "$WORK/compare-$case.json" || status=1
done

echo "[reproduce] done, reports in $WORK (exit $status)"
exit "$status"
