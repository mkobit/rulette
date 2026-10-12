"""Replay six current CLI journeys in sandbox temporary directories."""
import hashlib
import json
import pathlib
import re
import subprocess
import tempfile
import time

binary = pathlib.Path('target/debug/rulette').resolve()
root = pathlib.Path(tempfile.mkdtemp(prefix='rulette-ka2-', dir='/tmp'))
source = root / 'source'
source.mkdir()
(source / 'AGENTS.md').write_text('# Guidance\n\nUse explicit inputs.\n')
live = root / 'live'
live.mkdir()
(live / 'unrelated.txt').write_text('caller owned\n')
records = []


def snapshot():
    return {str(p.relative_to(live)): {'sha256': hashlib.sha256(p.read_bytes()).hexdigest(),
                                      'bytes': p.read_text(), 'mode': oct(p.stat().st_mode & 0o777)}
            for p in sorted(live.rglob('*')) if p.is_file()}


def run(journey, label, *args):
    before = snapshot()
    start = time.monotonic()
    result = subprocess.run([str(binary), *map(str, args)], text=True, capture_output=True)
    record = dict(journey=journey, label=label, argv=[str(binary), *map(str, args)],
                  exit=result.returncode, stdout=result.stdout, stderr=result.stderr,
                  elapsed_seconds=time.monotonic() - start, before=before, after=snapshot())
    records.append(record)
    print(label, result.returncode, flush=True)
    return record


def stage(journey, label, name, targets):
    args = ['transform', source, '--from', 'codex', '--project-root', live, '--stage', root / name]
    for target in targets:
        args += ['--target', target]
    result = run(journey, label, *args)
    match = re.search(r'sha256_[0-9a-f]{64}', result['stderr'])
    if not match:
        raise RuntimeError('Staging failed; inspect recorded output')
    return root / name / 'rulette.plan.json', match.group()


def apply(journey, label, plan, digest, *extra):
    return run(journey, label, 'transform', '--apply', plan, '--expect-plan-sha256', digest,
               '--allow-project-root', live, *extra)


try:
    run('preview/publication', 'inspect-help', 'inspect', '--help')
    run('preview/publication', 'transform-help', 'transform', '--help')
    run('preview/publication', 'preview', 'inspect', source, '--from', 'codex', '--to', 'codex')
    plan, digest = stage('preview/publication', 'first-stage', 'first-stage', ['codex@project'])
    apply('preview/publication', 'first-apply', plan, digest)
    plan, digest = stage('multi-target', 'multi-stage', 'multi-stage',
                         ['codex@project', 'claude@project', 'codex@project'])
    apply('multi-target', 'multi-apply', plan, digest)
    run('CI drift/coverage', 'unchanged-check', 'transform', source, '--from', 'codex',
        '--target', 'codex@project', '--target', 'claude@project', '--project-root', live, '--check')
    run('CI drift/coverage', 'coverage', 'inspect', source, '--from', 'codex', '--coverage', '--json', '--strict')
    (source / 'AGENTS.md').write_text('# Guidance\n\nUse revised explicit inputs.\n')
    run('CI drift/coverage', 'changed-check', 'transform', source, '--from', 'codex',
        '--target', 'codex@project', '--project-root', live, '--check')
    plan, digest = stage('update/migration', 'update-stage', 'update-stage', ['codex@project'])
    apply('update/migration', 'update-conflict', plan, digest)
    apply('update/migration', 'update-replace', plan, digest, '--replace')
    apply('failure recovery', 'wrong-digest', plan, 'sha256_' + '0' * 64, '--replace')
    run('failure recovery', 'missing-input', 'inspect', root / 'missing', '--from', 'codex')
    other = root / 'other-source'
    other.mkdir()
    (other / 'AGENTS.md').write_text('# Other guidance\n\nConflicting source identity.\n')
    run('failure recovery', 'source-collision', 'transform', source, other, '--from', 'codex',
        '--target', 'codex@project', '--project-root', live, '--stage', root / 'collision-stage')
    run('failure recovery', 'unsupported-scope', 'transform', source, '--from', 'codex',
        '--target', 'cursor@user', '--user-root', 'cursor=' + str(root / 'user'), '--stage', root / 'unsupported')
    run('failure recovery', 'corrected-check', 'transform', '--apply', plan,
        '--expect-plan-sha256', digest, '--allow-project-root', live, '--check')
    plan, digest = stage('retirement/shared destinations', 'omit-claude-stage', 'omit-stage', ['codex@project'])
    apply('retirement/shared destinations', 'omit-claude-apply', plan, digest)
finally:
    output = pathlib.Path('docs/benchmarks/rulette-workflow-assessment.json')
    output.write_text(json.dumps({'binary': str(binary), 'temporary_root': str(root),
                                 'binary_sha256': hashlib.sha256(binary.read_bytes()).hexdigest(),
                                 'binary_version': subprocess.run([str(binary), '--version'], text=True,
                                                                  capture_output=True).stdout.strip(),
                                 'timing': 'Agent execution proxy; excludes source editing, review and tool overhead.',
                                 'records': records}, indent=2) + '\n')
