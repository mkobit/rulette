// Research fixture runner, not a Rulette feature or an upstream test suite.
// Run only in the disposable rulette-l8r-skills Docker Sandbox.
import { execFileSync, spawn } from 'node:child_process';
import { createHash } from 'node:crypto';
import { createServer } from 'node:http';
import { hostname, homedir, tmpdir } from 'node:os';
import { dirname, join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';
import { stripVTControlCharacters } from 'node:util';
import * as fs from 'node:fs/promises';

if (hostname() !== 'rulette-l8r-skills') {
  throw new Error('Run in the disposable rulette-l8r-skills sandbox; global tests write to its home.');
}
const root = await fs.mkdtemp(join(tmpdir(), 'rulette-skills-'));
const project = join(root, 'project');
const source = join(root, 'source');
const other = join(root, 'other');
const nested = join(root, 'nested');
const partial = join(root, 'partial');
for (const path of [project, source, other, nested, partial]) await fs.mkdir(path);
const records = [];
const evidence = {
  date: new Date().toISOString(),
  version: '1.7.2',
  commit: '671e8c320810d36fed80fac5f1a2c1bf7e82d812',
  node: process.version,
  npm: execFileSync('npm', ['--version'], { encoding: 'utf8' }).trim(),
  kernel: execFileSync('uname', ['-sr'], { encoding: 'utf8' }).trim(),
  platform: process.platform,
  arch: process.arch,
  hostname: hostname(),
  telemetry: 'DO_NOT_TRACK=1',
  records,
};
const output = join(dirname(fileURLToPath(import.meta.url)), 'skills-1.7.2-results.json');
function clean(value) {
  return stripVTControlCharacters(value).replaceAll(root, '$WORKSPACE').replaceAll(homedir(), '$SANDBOX_HOME');
}
async function save() {
  await fs.writeFile(output, JSON.stringify(evidence, null, 2) + '\n');
}
async function file(path, value) {
  await fs.mkdir(dirname(path), { recursive: true });
  await fs.writeFile(path, value);
}
async function skill(base, name, body) {
  const dir = join(base, name);
  await file(join(dir, 'SKILL.md'), `---\nname: ${name}\ndescription: Controlled benchmark fixture.\n---\n\n${body}\n`);
  await file(join(dir, 'references', 'note.txt'), 'opaque companion\n');
  await file(join(dir, 'scripts', 'sample.sh'), '#!/bin/sh\nexit 0\n');
  await fs.chmod(join(dir, 'scripts', 'sample.sh'), 0o755);
}
await skill(source, 'benchmark-alpha', 'First source, version one.');
await skill(source, 'benchmark-beta', 'Second skill.');
await skill(other, 'benchmark-alpha', 'Colliding source, version two.');
await skill(nested, 'benchmark-root', 'Root skill.');
await skill(join(nested, 'benchmark-root', 'children'), 'benchmark-child', 'Nested skill.');
await file(join(partial, '.claude'), 'This regular file deliberately blocks an agent directory.\n');

async function snapshot(base) {
  const result = {};
  async function walk(path) {
    let stat;
    try { stat = await fs.lstat(path); } catch (error) {
      if (error.code === 'ENOENT') return;
      throw error;
    }
    const name = relative(base, path) || '.';
    if (stat.isSymbolicLink()) result[name] = { link: clean(await fs.readlink(path)) };
    else if (stat.isDirectory()) {
      for (const child of (await fs.readdir(path)).sort()) await walk(join(path, child));
    } else if (stat.isFile()) {
      const bytes = await fs.readFile(path);
      result[name] = {
        sha256: createHash('sha256').update(bytes).digest('hex'),
        mode: (stat.mode & 0o777).toString(8),
        text: clean(bytes.toString()),
      };
    }
  }
  await walk(base);
  return result;
}
async function run(id, args, cwd = project, extraEnv = {}) {
  const started = performance.now();
  const child = spawn('npx', ['--yes', 'skills@1.7.2', ...args], {
    cwd,
    env: { ...process.env, DO_NOT_TRACK: '1', npm_config_cache: join(root, 'npm-cache'), ...extraEnv },
    stdio: ['ignore', 'pipe', 'pipe'],
    detached: true,
  });
  let stdout = '', stderr = '', timedOut = false;
  child.stdout.on('data', chunk => { stdout += chunk; });
  child.stderr.on('data', chunk => { stderr += chunk; });
  const timer = setTimeout(() => {
    timedOut = true;
    process.kill(-child.pid, 'SIGKILL');
  }, 60000);
  const status = await new Promise((resolve, reject) => {
    child.on('error', reject);
    child.on('close', (code, signal) => resolve({ code, signal }));
  });
  clearTimeout(timer);
  const record = {
    id, command: ['npx', '--yes', 'skills@1.7.2', ...args].map(clean),
    cwd: clean(cwd), ...status, timedOut,
    elapsedMs: Math.round(performance.now() - started),
    stdout: clean(stdout), stderr: clean(stderr),
    files: await snapshot(cwd),
  };
  records.push(record);
  if (id === 'use') {
    const temporaryPath = stdout.match(/\/tmp\/skills-use-[^\s]+\/benchmark-alpha/)?.[0];
    record.temporaryResources = temporaryPath ? await snapshot(temporaryPath) : null;
  }
  await save();
  console.log(`${id}: exit=${status.code} timeout=${timedOut} ${record.elapsedMs}ms`);
}

await run('version', ['--version']);
if (records[0].code !== 0 || records[0].stdout.trim() !== evidence.version) {
  throw new Error('Pinned CLI could not be verified; stop before lifecycle tests.');
}
await run('help', ['--help']);
await run('find', ['find', 'typescript']);
try {
  const response = await fetch('https://skills.sh/api/search?q=typescript&limit=20', { signal: AbortSignal.timeout(15000) });
  evidence.searchApiProbe = { status: response.status, url: response.url, body: (await response.text()).slice(0, 500) };
} catch (error) { evidence.searchApiProbe = { error: error.message }; }
await run('inspect-missing-command', ['inspect', source]);
await run('json-without-confirmation', ['add', source, '--list', '--json']);
await run('json-preview-rejected', ['add', source, '--list', '--json', '-y']);
await run('source-preview', ['add', source, '--list']);
await run('empty-list', ['list', '--json']);
await run('agent-detection', ['add', source, '-s', 'benchmark-beta', '-y', '--json']);
await run('selective-symlink', ['add', source, '-s', 'benchmark-alpha', '-a', 'codex', 'claude-code', 'cursor', '-y', '--json']);
await run('installed-list', ['list', '--json']);
await run('use', ['use', source, '-s', 'benchmark-alpha']);
await run('invalid-agent', ['add', source, '-s', 'benchmark-beta', '-a', 'not-an-agent', '-y', '--json']);
await run('missing-skill', ['add', source, '-s', 'missing', '-a', 'codex', '-y', '--json']);
await run('invalid-path', ['add', join(root, 'absent'), '-a', 'codex', '-y', '--json']);
await run('nested-default', ['add', join(nested, 'benchmark-root'), '--list']);
await run('nested-full-depth', ['add', join(nested, 'benchmark-root'), '--list', '--full-depth']);
await run('local-update', ['update', '-p', '-y']);
await file(join(source, 'benchmark-alpha', 'SKILL.md'), '---\nname: benchmark-alpha\ndescription: Changed fixture.\n---\nChanged source.\n');
await run('local-update-after-change', ['update', '-p', '-y']);
await run('local-reinstall', ['add', source, '-s', 'benchmark-alpha', '-a', 'codex', 'claude-code', 'cursor', '-y', '--json']);
await file(join(root, 'sentinel.txt'), 'Controlled file outside the skill directory.\n');
await fs.symlink(join(root, 'sentinel.txt'), join(source, 'benchmark-beta', 'outside-link.txt'));
await run('copy', ['add', source, '-s', 'benchmark-beta', '-a', 'claude-code', 'cursor', '-y', '--copy', '--json']);
await file(join(project, '.agents', 'skills', 'benchmark-alpha', 'local-edit.txt'), 'Locally edited installed tree.\n');
await run('collision', ['add', other, '-s', 'benchmark-alpha', '-a', 'codex', 'claude-code', 'cursor', '-y', '--json']);
await run('partial-target-failure', ['add', source, '-s', 'benchmark-alpha', '-a', 'codex', 'claude-code', '-y', '--json'], partial);
await fs.unlink(join(partial, '.claude'));
await run('partial-recovery', ['add', source, '-s', 'benchmark-alpha', '-a', 'codex', 'claude-code', '-y', '--json'], partial);
await run('restore-local', ['experimental_install', '-a', 'claude-code', '-y']);
await run('global-add', ['add', source, '-s', 'benchmark-alpha', '-a', 'claude-code', '-g', '-y', '--copy', '--json']);
evidence.globalAfterAdd = await snapshot(join(homedir(), '.agents'));
evidence.globalClaudeAfterAdd = await snapshot(join(homedir(), '.claude', 'skills', 'benchmark-alpha'));
await run('global-list', ['list', '-g', '--json']);
await run('global-update', ['update', '-g', '-y']);
await run('global-remove', ['remove', 'benchmark-alpha', '-g', '-a', 'claude-code', '-y']);
await run('remove-one-agent', ['remove', 'benchmark-alpha', '-a', 'claude-code', '-y']);
await run('remove-project', ['remove', 'benchmark-alpha', 'benchmark-beta', '-y']);
await run('list-after-remove', ['list', '--json']);
await file(join(project, 'node_modules', 'benchmark-package', 'package.json'), JSON.stringify({ name: 'benchmark-package', version: '1.0.0' }));
await skill(join(project, 'node_modules', 'benchmark-package', 'skills'), 'benchmark-sync', 'Dependency skill.');
await file(join(project, 'package.json'), JSON.stringify({ name: 'benchmark-project', private: true, dependencies: { 'benchmark-package': '1.0.0' } }));
await run('sync-preview', ['experimental_sync', '-a', 'claude-code', '-y', '--dry-run', '--no-remote']);
await run('sync', ['experimental_sync', '-a', 'claude-code', '-y', '--no-remote']);

// Controlled HTTP source exercises downloading without repository credentials or git operations.
const archivePath = join(root, 'fixture.tgz');
execFileSync('tar', ['-czf', archivePath, '-C', source, 'benchmark-beta']);
const server = createServer(async (request, response) => {
  if (request.url === '/SKILL.md') {
    response.writeHead(200, { 'Content-Type': 'text/plain' });
    response.end('---\nname: benchmark-download\ndescription: Direct HTTP fixture.\n---\nDownloaded instruction.\n');
  } else if (request.url === '/fixture.tgz') {
    response.writeHead(200, { 'Content-Type': 'application/gzip' });
    response.end(await fs.readFile(archivePath));
  } else if (request.url?.startsWith('/api/search')) {
    response.writeHead(503); response.end('Controlled search failure.');
  } else { response.writeHead(404); response.end(); }
});
await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
const url = `http://127.0.0.1:${server.address().port}/SKILL.md`;
try {
  await run('download-json', ['add', url, '-a', 'claude-code', '-y', '--json']);
  await run('direct-download', ['add', url, '-a', 'claude-code', '-y']);
  await run('archive-download', ['add', url.replace('SKILL.md', 'fixture.tgz'), '-s', 'benchmark-beta', '-a', 'claude-code', '-y']);
  await run('download-update', ['update', '-p', '-y']);
  await run('find-http-failure', ['find', 'typescript'], project, { SKILLS_API_URL: url.replace('/SKILL.md', '') });
  await run('unknown-command', ['not-a-command']);
} finally {
  await new Promise(resolve => server.close(resolve));
  evidence.globalAfterRemove = await snapshot(join(homedir(), '.agents'));
  await save();
}
console.log(`Evidence saved to ${output}; disposable workspace retained at ${root}.`);
