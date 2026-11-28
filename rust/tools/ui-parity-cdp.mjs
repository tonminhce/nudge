import { spawn } from 'node:child_process';
import { mkdtemp, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import {
  captureSurface,
  diffSurfaces,
  formatDiff,
  saveCapture,
  saveDiff,
} from './ui-parity-harness.mjs';

const CHROME = '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome';

async function waitForDebugger(port, timeoutMs = 40000) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    try {
      const response = await fetch(`http://127.0.0.1:${port}/json/version`);
      if (response.ok) return response.json();
    } catch {}
    await new Promise((resolve) => setTimeout(resolve, 200));
  }
  throw new Error('CDP_UNAVAILABLE');
}

export async function openSession(options = {}) {
  const port = options.port ?? 9333;
  const width = options.width ?? 1440;
  const height = options.height ?? 900;
  const profile = await mkdtemp(join(tmpdir(), 'nudge-ui-parity-'));
  const proc = spawn(CHROME, [
    '--headless=new',
    `--remote-debugging-port=${port}`,
    `--user-data-dir=${profile}`,
    '--no-first-run',
    '--no-default-browser-check',
    '--disable-gpu',
    `--window-size=${width},${height}`,
    'about:blank',
  ], { stdio: 'ignore' });

  const version = await waitForDebugger(port);
  const socket = new WebSocket(version.webSocketDebuggerUrl);
  await new Promise((resolve, reject) => {
    socket.addEventListener('open', resolve, { once: true });
    socket.addEventListener('error', reject, { once: true });
  });

  let nextId = 1;
  const pending = new Map();
  const listeners = new Set();
  socket.addEventListener('message', (event) => {
    const message = JSON.parse(event.data);
    if (message.id && pending.has(message.id)) {
      const { resolve, reject } = pending.get(message.id);
      pending.delete(message.id);
      message.error ? reject(new Error(message.error.message)) : resolve(message.result);
      return;
    }
    for (const listener of listeners) listener(message);
  });
  const browserSend = (method, params = {}, session) => new Promise((resolve, reject) => {
    const id = nextId++;
    pending.set(id, { resolve, reject });
    socket.send(JSON.stringify({ id, method, params, ...(session ? { sessionId: session } : {}) }));
  });

  const { targetId } = await browserSend('Target.createTarget', { url: 'about:blank' });
  const { sessionId } = await browserSend('Target.attachToTarget', { targetId, flatten: true });
  const send = (method, params = {}) => browserSend(method, params, sessionId);
  await send('Page.enable');
  await send('Runtime.enable');
  await send('Network.enable');
  await send('Emulation.setDeviceMetricsOverride', {
    width,
    height,
    deviceScaleFactor: 2,
    mobile: false,
  });

  const evaluate = async (expression) => {
    const result = await send('Runtime.evaluate', {
      expression,
      awaitPromise: true,
      returnByValue: true,
    });
    if (result.exceptionDetails) {
      throw new Error(
        result.exceptionDetails.exception?.description ?? result.exceptionDetails.text,
      );
    }
    return result.result.value;
  };

  const tab = {
    automation: { evaluateScript: (source) => evaluate(`(() => { ${source}; return extractSurface(); })();`) },
    screenshot: async () => {
      const { data } = await send('Page.captureScreenshot', { format: 'png' });
      return Buffer.from(data, 'base64');
    },
  };

  return {
    tab,
    width,
    height,
    onMessage: (listener) => listeners.add(listener),
    goto: (url) => send('Page.navigate', { url }),
    evaluate,
    setCookie: (name, value, url) => send('Network.setCookie', { name, value, url }),
    storage: async (entries) => {
      for (const [key, value] of Object.entries(entries)) {
        await evaluate(`localStorage.setItem(${JSON.stringify(key)}, ${JSON.stringify(value)});`);
      }
    },
    fill: (selector, value) => evaluate(`(() => {
      const node = document.querySelector(${JSON.stringify(selector)});
      if (!node) throw new Error('MISSING_INPUT ' + ${JSON.stringify(selector)});
      const proto = node.tagName === 'TEXTAREA' ? HTMLTextAreaElement : HTMLInputElement;
      Object.getOwnPropertyDescriptor(proto.prototype, 'value').set.call(node, ${JSON.stringify(value)});
      node.dispatchEvent(new Event('input', { bubbles: true }));
      node.dispatchEvent(new Event('change', { bubbles: true }));
    })();`),
    click: (selector) => evaluate(`(() => {
      const node = document.querySelector(${JSON.stringify(selector)});
      if (!node) throw new Error('MISSING_BUTTON ' + ${JSON.stringify(selector)});
      node.click();
    })();`),
    wait: async (predicate, timeoutMs = 20000) => {
      const deadline = Date.now() + timeoutMs;
      while (Date.now() < deadline) {
        if (await evaluate(`Boolean(${predicate})`)) return true;
        await new Promise((resolve) => setTimeout(resolve, 250));
      }
      return false;
    },
    settle: (ms) => new Promise((resolve) => setTimeout(resolve, ms)),
    focus: (selector) => evaluate(`(() => {
      const node = document.querySelector(${JSON.stringify(selector)});
      if (!node) throw new Error('MISSING_INPUT ' + ${JSON.stringify(selector)});
      node.focus();
    })();`),
    insertText: (value) => send('Input.insertText', { text: value }),
    pressEnter: () => {
      const key = { key: 'Enter', code: 'Enter', windowsVirtualKeyCode: 13, nativeVirtualKeyCode: 13 };
      return Promise.all([
        send('Input.dispatchKeyEvent', { type: 'keyDown', text: '\r', ...key }),
        send('Input.dispatchKeyEvent', { type: 'keyUp', ...key }).then(() => true),
      ]);
    },
    clickText: (matcher) => evaluate(`(() => {
      const pattern = new RegExp(${JSON.stringify(matcher)}, 'i');
      const node = [...document.querySelectorAll('button')].find(b => pattern.test(b.textContent || ''));
      if (!node) throw new Error('MISSING_BUTTON ' + ${JSON.stringify(matcher)});
      const rect = node.getBoundingClientRect();
      return JSON.stringify({ x: rect.x + rect.width / 2, y: rect.y + rect.height / 2 });
    })();`).then(async (point) => {
      const { x, y } = JSON.parse(point);
      const buttons = 1;
      await send('Input.dispatchMouseEvent', { type: 'mousePressed', x, y, button: 'left', buttons, clickCount: 1 });
      await send('Input.dispatchMouseEvent', { type: 'mouseReleased', x, y, button: 'left', buttons: 0, clickCount: 1 });
    }),
    close: async () => {
      proc.kill('SIGKILL');
      socket.close();
      await rm(profile, { recursive: true, force: true, maxRetries: 5, retryDelay: 200 })
        .catch(() => {});
    },
  };
}

export { captureSurface, diffSurfaces, formatDiff, saveCapture, saveDiff, writeFile };
