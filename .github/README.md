# Local validation only

Remote workflows and their archived YAML definitions have been removed.
Do not recreate hosted or self-hosted remote jobs, dispatch workflows or make
remote checks a prerequisite. Use `python3 tools/dev.py` and the contributing
guide for local CPU, consumer, document, asset and GPU lanes.

Missing GPU, source assets or accepted baselines block a required task.
Retain engineering, image regression, upstream parity and performance results
as distinct outcomes; local checks do not claim coverage on untested platforms.
