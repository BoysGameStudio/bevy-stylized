"""Required-test execution and explicit task failures (stdlib only)."""
import json
import re
import subprocess


class CheckFailed(RuntimeError):
    """A contract, process, or validation failure; exit 1."""


class ResourceBlocked(RuntimeError):
    """An identified required external resource is unavailable; exit 2."""


RESOURCE_MESSAGES = (
    'required GPU unavailable',
    'accepted baseline missing',
    'accepted regression baseline missing',
)
SUMMARY = re.compile(r'test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;')


def invoke(command, **kwargs):
    try:
        return subprocess.run(command, **kwargs)
    except FileNotFoundError as error:
        raise ResourceBlocked(f'required executable missing: {command[0]}') from error


def aggregate(cases):
    counts = {key: sum(c['status'] == state for c in cases)
              for key, state in [('passed', 'PASS'), ('failed', 'FAIL'), ('blocked', 'BLOCKED')]}
    status = 'FAIL' if counts['failed'] else 'BLOCKED' if counts['blocked'] else 'PASS'
    return dict(counts, status=status)


def classify_case(output, code, identity):
    """One exact test per process: resource diagnostics cannot mask another case."""
    summaries = SUMMARY.findall(output)
    starts = re.findall(r'^test ([\w:]+) \.\.\. ', output, re.M)
    valid = (len(summaries) == 1 and starts == [identity]
             and tuple(map(int, summaries[0][1:])) in {(1, 0, 0), (0, 1, 0)})
    if not valid:
        return 'FAIL', 'test-output-invalid'
    if code == 0 and summaries[0] == ('ok', '1', '0', '0'):
        return 'PASS', 'test-passed'
    if code != 0 and summaries[0] == ('FAILED', '0', '1', '0'):
        # Only a resource panic from this exact test establishes BLOCKED. A warning
        # or resource phrase printed before an unrelated assertion is insufficient.
        panics = re.findall(r"thread '[^']*'(?: \(\d+\))? panicked at [^\n]*\n([^\n]*)", output)
        if panics and all(any(message.startswith(marker) for marker in RESOURCE_MESSAGES)
                          for message in panics):
            return 'BLOCKED', 'required-resource-missing'
    return 'FAIL', 'test-failed'


def run_tests(project, name, command, expected, env):
    """Check the filtered ignored-test inventory, then execute each exact identity.

    command is cargo test with one --test binary and an optional Cargo name filter,
    without the libtest separator. expected is the exact list, never a count.
    """
    if (not isinstance(expected, list) or not expected
            or any(not isinstance(n, str) or not re.fullmatch(r'[\w:]+', n) for n in expected)
            or len(expected) != len(set(expected))):
        raise CheckFailed('required suite needs unique, nonempty case identities')
    env = dict(env)
    env.pop('GOLDEN_REBASELINE', None)
    env['CARGO_TERM_COLOR'] = 'never'
    logs = project.run_output / 'logs'
    logs.mkdir(parents=True, exist_ok=True)
    listing = invoke(command + ['--', '--ignored', '--list'], cwd=project.root,
                             env=env, capture_output=True, text=True)
    (logs / (name + '-inventory.log')).write_text(listing.stdout + listing.stderr)
    identities = re.findall(r'^([\w:]+): test$', listing.stdout, re.M)
    errors = []
    if listing.returncode or sorted(identities) != sorted(expected):
        errors.append('required test inventory mismatch or inventory process failed')
    cases = []
    if not errors:
        for identity in expected:
            print(f'CASE {name}:{identity}', flush=True)
            result = invoke(command + ['--', '--ignored', '--exact', identity,
                                               '--nocapture', '--test-threads=1'],
                                    cwd=project.root, env=env, stdout=subprocess.PIPE,
                                    stderr=subprocess.STDOUT, text=True)
            log = logs / (name + '-' + identity.replace('::', '-') + '.log')
            log.write_text(result.stdout)
            status, reason = classify_case(result.stdout, result.returncode, identity)
            cases.append({'case': identity, 'status': status, 'reason_code': reason,
                          'exit_code': result.returncode, 'log': str(log)})
    state = dict(aggregate(cases), suite=name, expected=len(expected), identities=expected,
                 actual_inventory=identities, executed=len(cases), cases=cases,
                 not_executed=len(expected) - len(cases), errors=errors,
                 inventory_exit_code=listing.returncode)
    if errors:
        state['status'] = 'FAIL'
    project.report(name, state)
    print(json.dumps(state, indent=2))
    return state


def require_suites(states):
    if not states:
        raise CheckFailed('no required suites executed')
    status = aggregate(states)['status']
    if status == 'FAIL':
        raise CheckFailed('required suite failed; inspect this run reports')
    if status == 'BLOCKED':
        raise ResourceBlocked('required suite resource missing; inspect this run reports')
