//! ỌKỌ desktop UI — JavaScript IPC client (SRD UI-008).
//!
//! Connects to the host over typed, versioned, validated IPC messages.
//! All sensitive requests are capability-checked by the native host
//! (SRD SEC-002). Remote web pages cannot reach the host API directly
//! (SRD UI-009).

(function () {
  'use strict';

  // --- IPC over stdio (MVP transport) -------------------------------------

  const lines = [];
  let resolveLine = null;

  function sendMessage(msg) {
    const line = JSON.stringify(msg);
    if (typeof process !== 'undefined' && process.stdout) {
      // Node.js / host-side JS runtime
      process.stdout.write(line + '\n');
    } else {
      // Browser engine context: post through the engine's host bridge.
      // The engine adapter implements this behind BrowserEnginePort.
      window.__okoHostBridge__ && window.__okoHostBridge__.send(line);
    }
  }

  function readLine() {
    return new Promise((resolve) => {
      resolveLine = resolve;
      if (typeof process !== 'undefined' && process.stdin) {
        const readline = require('readline');
        const rl = readline.createInterface({ input: process.stdin });
        rl.on('line', (line) => {
          if (resolveLine) { resolveLine(line); resolveLine = null; }
        });
      }
    });
  }

  async function readMessages() {
    while (true) {
      const line = await readLine();
      if (!line) break;
      try {
        const msg = JSON.parse(line);
        handleIncoming(msg);
      } catch (e) {
        console.error('oko-ui: bad IPC message:', e);
      }
    }
  }

  // --- Host API calls ------------------------------------------------------

  let msgId = 0;

  async function hostCall(route, payload = {}) {
    const id = String(++msgId);
    const msg = { v: 1, id, direction: 'request', route, payload };
    sendMessage(msg);
    // In a real browser engine, the response comes async via readMessages().
    // Here we resolve via a pending map.
    return new Promise((resolve) => {
      window.__okoPending__ = window.__okoPending__ || {};
      window.__okoPending__[id] = resolve;
    });
  }

  async function hostNotify(route, payload = {}) {
    const msg = { v: 1, id: 'notify', direction: 'notification', route, payload };
    sendMessage(msg);
  }

  // --- Incoming message handler -------------------------------------------

  async function handleIncoming(msg) {
    if (msg.direction === 'response') {
      const pending = window.__okoPending__ && window.__okoPending__[msg.id];
      if (pending) {
        pending(msg.payload);
        delete window.__okoPending__[msg.id];
      }
      return;
    }
    if (msg.direction === 'notification') {
      if (msg.route === 'ui.reload') {
        // TERM-007: terminal sessions survive UI reload — just re-render.
        renderDesktop();
      }
    }
  }

  // --- Desktop UI ----------------------------------------------------------

  const els = {};

  function $ (id) { return document.getElementById(id); }

  function renderDesktop() {
    els.status = $('status');
    els.clock = $('clock');
    els.terminalOutput = $('terminal-output');
    els.terminalInput = $('terminal-input');
    els.browserEngine = $('browser-engine');
    els.browserStatus = $('browser-status');
    els.capStatus = $('cap-status');
    els.engineStatus = $('engine-status');

    // Clock (UI-003: keyboard shortcuts, live UI)
    setInterval(() => {
      const now = new Date();
      if (els.clock) els.clock.textContent = now.toLocaleTimeString();
    }, 1000);

    // Terminal input → host.terminal.write (via host API)
    if (els.terminalInput) {
      els.terminalInput.addEventListener('keydown', async (e) => {
        if (e.key === 'Enter') {
          const text = els.terminalInput.value;
          els.terminalInput.value = '';
          appendTerminal(text + '\n');
          // Send to host via IPC host API (capability-checked)
          try {
            await hostCall('host.terminal.write', { session_id: window.__okoSessionId__, text: text + '\n' });
          } catch (err) {
            appendTerminal('error: ' + err + '\n');
          }
        }
      });
    }

    // Buttons
    if ($('btn-launcher')) {
      $('btn-launcher').addEventListener('click', () => {
        appendTerminal('app launcher: opening (desktop shell UI)\n');
      });
    }
    if ($('btn-settings')) {
      $('btn-settings').addEventListener('click', () => {
        appendTerminal('settings: opening (desktop shell UI)\n');
      });
    }

    // Query host status on load
    refreshStatus();
  }

  async function refreshStatus() {
    try {
      const payload = await hostCall('host.status', {});
      if (els.status) els.status.textContent = 'core: ' + (payload.core || 'unknown');
    } catch (e) {
      if (els.status) els.status.textContent = 'core: disconnected';
    }
    try {
      const eng = await hostCall('host.browser.engine', {});
      if (els.browserEngine) els.browserEngine.textContent = eng.engine || 'unknown';
      if (els.engineStatus) els.engineStatus.textContent = 'engine: ' + (eng.engine || 'unknown') + (eng.experimental ? ' (experimental)' : '');
    } catch (e) {
      if (els.browserEngine) els.browserEngine.textContent = 'unavailable';
    }
  }

  function appendTerminal(text) {
    if (els.terminalOutput) {
      els.terminalOutput.textContent += text;
      els.terminalOutput.scrollTop = els.terminalOutput.scrollHeight;
    }
  }

  function createTerminal() {
    return hostCall('host.terminal.create', {})
      .then((payload) => {
        window.__okoSessionId__ = payload.session_id;
        appendTerminal('terminal session ' + payload.session_id + ' started\n');
      })
      .catch((err) => appendTerminal('error: ' + err + '\n'));
  }

  // --- Boot ----------------------------------------------------------------

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', () => {
      renderDesktop();
      createTerminal();
      readMessages(); // background IPC reader
    });
  } else {
    renderDesktop();
    createTerminal();
    readMessages();
  }

})();
