// Browser showcase driver: run each catalog scene in the disposable browser
// (one navigation per scene; failure isolation) and collect the host's
// published results. Never attaches to a user profile.
import {spawn} from 'node:child_process';
import * as fs from 'node:fs/promises';
import path from 'node:path';
import os from 'node:os';
const [chrome, urlBase, output, flagsJson, scenesJson] = process.argv.slice(2);
if (!chrome || !urlBase || !output || !scenesJson) throw Error('expected chrome url-base output flags-json scenes-json');
const flags = JSON.parse(flagsJson || '[]');
const scenes = JSON.parse(scenesJson);
const file = name => path.join(output, name);
const profile = await fs.mkdtemp(path.join(os.tmpdir(), 'stylized-browser-showcase-'));
const log = await fs.open(file('browser-runtime.log'), 'w');
const browser = spawn(chrome, [...flags, '--disable-background-networking', '--enable-logging=stderr', '--no-first-run', '--no-default-browser-check', `--user-data-dir=${profile}`, '--remote-debugging-port=0', 'about:blank'], {stdio: ['ignore', log.fd, log.fd]});
await fs.writeFile(file('browser-process.json'), JSON.stringify({pid: browser.pid, profile}));
const sleep = ms => new Promise(r => setTimeout(r, ms));
const exited = new Promise(r => browser.once('exit', r));
try {
  let port;
  for (let i = 0; i < 200; i++) { try { port = (await fs.readFile(profile + '/DevToolsActivePort', 'utf8')).split('\n')[0]; break; } catch { await sleep(100); } }
  if (!port) throw Error('browser endpoint absent');
  const version = await (await fetch(`http://127.0.0.1:${port}/json/version`)).json();
  const tabs = await (await fetch(`http://127.0.0.1:${port}/json/list`)).json();
  const ws = new WebSocket(tabs.find(x => x.type === 'page').webSocketDebuggerUrl);
  await new Promise((r, j) => { ws.onopen = r; ws.onerror = j; });
  let id = 0; const pending = new Map();
  ws.onmessage = e => { const m = JSON.parse(e.data); if (m.id) { const p = pending.get(m.id); if (p) { pending.delete(m.id); clearTimeout(p.timer); m.error ? p.reject(m.error) : p.resolve(m.result); } } };
  const call = (method, params = {}) => new Promise((resolve, reject) => { const n = ++id; const timer = setTimeout(() => { pending.delete(n); reject(Error(method + ' timed out')); }, 60000); pending.set(n, {resolve, reject, timer}); ws.send(JSON.stringify({id: n, method, params})); });
  await call('Runtime.enable'); await call('Page.enable');
  const results = {};
  for (const scene of scenes) {
    const entry = {scene};
    try {
      await call('Page.navigate', {url: `${urlBase}?scene=${scene}`});
      let result;
      for (let i = 0; i < 240; i++) {
        const response = await call('Runtime.evaluate', {expression: '(()=>{const r=globalThis.stylizedResult;if(r){const {rgba,...meta}=r;return meta;}return null;})()', returnByValue: true});
        if (response.exceptionDetails) throw Error(JSON.stringify(response.exceptionDetails));
        result = response.result.value;
        if (result) break;
        await sleep(500);
      }
      if (!result) { entry.status = 'FAIL'; entry.reason = 'scene result timeout'; results[scene] = entry; console.log(JSON.stringify(entry)); continue; }
      Object.assign(entry, result);
      const errors = (await call('Runtime.evaluate', {expression: 'globalThis.stylizedErrors || []', returnByValue: true})).result.value || [];
      entry.errors = errors;
      if (entry.status === 'PASS') {
        entry.rgba = [];
        const length = entry.width * entry.height * 4;
        if (length > 16777216) throw Error('unexpected capture size');
        for (let offset = 0; offset < length; offset += 16384) {
          const part = await call('Runtime.evaluate', {expression: `globalThis.stylizedResult.rgba.slice(${offset},${offset + 16384})`, returnByValue: true});
          entry.rgba.push(...part.result.value);
        }
        entry.rgba_fnv1a64 = undefined; // lane computes artifact hashes from bytes
      }
      results[scene] = entry;
      console.log(JSON.stringify({scene, status: entry.status, errors: errors.length}));
    } catch (error) {
      results[scene] = {scene, status: 'FAIL', reason: String(error)};
      console.log(JSON.stringify({scene, status: 'FAIL', reason: String(error)}));
    }
  }
  const adapter = await call('Runtime.evaluate', {expression: 'navigator.gpu.requestAdapter().then(a=>a?{vendor:a.info.vendor,architecture:a.info.architecture,device:a.info.device,description:a.info.description}:null)', awaitPromise: true, returnByValue: true});
  await fs.writeFile(file('browser-showcase-capture.json'), JSON.stringify({browser: version.Browser, adapter: adapter.result.value, scenes: results}));
  ws.close(); for (const p of pending.values()) clearTimeout(p.timer);
} finally { browser.kill('SIGTERM'); await Promise.race([exited, sleep(3000)]); if (browser.exitCode === null && browser.signalCode === null) { browser.kill('SIGKILL'); await exited; } await fs.rm(profile, {recursive: true, force: true, maxRetries: 10, retryDelay: 100}); await log.close(); }
