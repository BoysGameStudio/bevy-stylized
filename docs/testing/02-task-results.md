# Developer task result contract

The developer task entrypoint is `python3 tools/dev.py`. Its result contract is
`bevy_stylized.task/v1`; it does not change rendering, authoring or baseline schemas.

## Invocation and report ownership

A task allocates a unique run directory beneath the configured project output.
Automation may pass `--run-id ID` before the subcommand; IDs are 1–80 ASCII letters,
digits, underscores or hyphens and cannot reuse an existing directory. The parent
retains this ID before starting the child. Interactive invocations print `RUN ID`.

Each run writes `runs/ID/LANE.json`, child reports and logs. A fixed `LANE.json` in
the output root is only a convenience copy. Never infer a current success solely
from this copy or from an old log. `Project.read_report` checks the expected run ID,
terminal state and optional expected source hash; the isolation caller uses it.
Callers must also verify the intended lane, inputs and expected outcome.

Before executing work, the entrypoint writes RUNNING. It then writes PASS, FAIL or
BLOCKED, including on handled failures. A killed process can leave RUNNING; this
is incomplete evidence, not success. If the initial current-state write fails,
work does not start. A failed write or missing report must be handled using the
child exit status and the parent-owned ID; an old PASS is never a substitute.

Reports retain historical runs. Only the convenience copies are replaced. A lane's
`reports` array lists child reports created by this invocation, not files discovered
from earlier runs. Per-suite case identity and raw process exit codes remain visible.

## Identity scope

Every report records run ID, lane, timestamps and a bounded source SHA-256. Source
identity covers root TOML/lock files, product/app/lab/test/tool sources, schemas,
first-party fixtures, source/recipe manifests and CI/Cargo configuration. It works
without Git and excludes caches and generated output. Documentation edits do not
change this execution identity. R0's full tracked-file manifest is a separate audit
snapshot; its digest uses a different scope and must not be compared as identical.

Explicit asset, case, manifest, candidate and baseline arguments record their paths
and file hashes (or null when missing). These are entry-input identities, not a
claim that a GLB hash covers its external textures. Existing asset/case validators
continue checking their dependency manifests and provenance. Freeze those inputs
for a comparison and retain their domain reports. A task-level PASS does not create
an upstream parity claim or accept a visual baseline.

## Failures and exits

| Condition | Report | CLI exit |
|---|---|---|
| Task completed | PASS | 0 |
| Contract/validation/process/internal failure | FAIL with reason code | 1 |
| Explicitly identified missing required resource | BLOCKED | 2 |
| Interrupted execution | FAIL if handled; otherwise RUNNING remains | nonzero |
| Argument parsing failure | usage message; no task report | 2 |

Reason codes distinguish validation, input contract, child process, I/O and internal
errors. Generic ValueError/RuntimeError does not mean an external resource is missing.
Child exit codes are preserved in reports rather than passed through as task exits.
Resources explicitly classified include required test GPU/baselines, declared
missing asset/case inputs, required executables, prepared packs and the qualified
isolation tools. Corrupt inputs, stale outputs and build failures remain FAIL.

The same exit code 2 can occur before task execution for invalid usage. Consumers
must validate the report and its reason instead of interpreting the exit code alone.
A diagnostic reference task can complete while its domain parity result is FAIL;
use `--required` to enforce the domain parity gate, and retain that domain report.

## Required tests

`tests/suites.json` owns required case names. GPU binary keys also select the binaries;
there is no second hard-coded GPU list. Empty registries or invalid/empty binary
case lists are rejected before any test process starts. The runner lists the filtered ignored tests
of each binary and compares the exact set, including duplicates. It then runs each
case using stable libtest `--exact`, `--ignored`, `--nocapture`, one thread, and checks
both the case identity and test summary. No nightly JSON harness is required.

Separate processes allow case-local resource classification. Only resource panic
messages from that process establish BLOCKED; a resource phrase printed before an
unrelated assertion is not sufficient. Malformed output, zero execution, unexpected
identity, multiple tests, inventory errors and process failure cannot pass. Logs
are retained to investigate changes to the locked libtest output format.

A suite reports expected/executed/not-executed counts and per-case status, reason,
exit code and log. Mixed failures retain both failed and blocked counts; FAIL takes
precedence in aggregation. The GPU command also checks synthetic import. Required
resource absence cannot silently skip a test. `GOLDEN_REBASELINE` is removed from
the required-test environment; baseline acceptance stays a separate explicit task.

## CPU and continuous checks

`check --lane cpu` retains ordinary workspace CPU tests and no external character
requirement. `check --lane inspector-cpu` runs the optional inspector's library tests
and all-target clippy (including binary compilation). The Rust CI installs its
X11/Wayland development prerequisites and runs this same entrypoint.

`check --lane consumers --fetch` explicitly allows dependency preparation for each
independent consumer, followed by offline validation. Without `--fetch`, a missing
Cargo cache is BLOCKED and a compile error is FAIL. This command never prepares art.
CPU CI now invokes this lane. Actual remote CI execution is separate evidence from
workflow definition or local success; no new GPU runner is implied.

Serialize Cargo and GPU/performance work on one workspace/device. Each GPU case now
starts its own test process; these gate timings are not comparable performance
benchmarks. Use the dedicated measurement workflows for performance claims.

## Isolation report dimensions

The isolation result retains engineering_status, image_regression_status and
parity_status separately, including on partial failure. Failed child steps retain
their observed exit/status rather than disappearing from the step list.
performance_status distinguishes a smoke measurement from accepted performance
qualification. The full Git-free copy hash is snapshot_sha256; source_sha256 stays
the task's bounded execution identity. These different digest scopes must not be
compared as though they were identical.
