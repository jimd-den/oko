// ỌKỌ panes: what a pane renders and does (SRD UI-002, §7.3).
//
// A pane hosts a terminal or a web document. Both are ordinary DOM inside this
// one workspace, both are driven through the same host API, and the terminal is
// also the command surface for the desktop — so `browser open <url>` in the
// terminal opens a web pane beside it. That is what "interchangeable" means
// here: same workspace, same policy, either kind of pane.

(function (global) {
  'use strict';

  const hostCall = global.Oko.hostCall;
  const el = (id) => document.getElementById(id);

  let sessionId = null;

  function write(text) {
    const out = el('terminal-output');
    out.textContent += text;
    out.scrollTop = out.scrollHeight;
  }

  async function openTerminal() {
    const created = await hostCall('host.terminal.create');
    sessionId = created.session_id;
    write('ỌKỌ terminal · session ' + sessionId + '\n');
    write('try: ls · echo hello · engine · browser open /apps/hello.html\n\n');
  }

  async function runInShell(text) {
    await hostCall('host.terminal.write', { session_id: sessionId, text: text + '\n' });
    const read = await hostCall('host.terminal.read', { session_id: sessionId });
    write(read.output || '');
  }

  // A line the desktop understands is handled by the desktop; anything else is
  // a shell command. That is the command model of SRD §7.3, reached from the
  // terminal instead of from a second, separate interface.
  async function runLine(text) {
    const trimmed = text.trim();
    if (trimmed.startsWith('browser open ')) {
      const url = trimmed.slice('browser open '.length).trim();
      await hostCall('host.browser.open', { url: url });
      openWebPane(url);
      return;
    }
    if (trimmed === 'engine') {
      await refreshEngine();
      write('engine status refreshed\n');
      return;
    }
    if (trimmed === 'apps') {
      const apps = await hostCall('host.app.list');
      write('apps: ' + JSON.stringify(apps.sessions) + '\n');
      return;
    }
    await runInShell(trimmed);
  }

  function openWebPane(url) {
    const view = el('browser-view');
    view.textContent = '';
    const frame = document.createElement('iframe');
    frame.className = 'browser-frame';
    // A page this host serves ships with ỌKỌ and is trusted local UI (zone B).
    // Anything from the network is untrusted content: it gets a unique origin,
    // so it cannot read this shell's token or reach the host API
    // (zone C, SRD §10.2, UI-009).
    const local = url.startsWith('/');
    frame.setAttribute(
      'sandbox',
      local ? 'allow-scripts allow-same-origin' : 'allow-scripts allow-forms allow-popups'
    );
    frame.setAttribute('referrerpolicy', 'no-referrer');
    frame.src = url;
    view.appendChild(frame);
    el('browser-title').textContent = local ? url : url.replace(/^https?:\/\//, '');
  }

  function showEngineStatus() {
    el('browser-view').innerHTML =
      '<div class="browser-placeholder">' +
      '<p>engine: <span id="browser-engine"></span></p>' +
      '<p id="browser-status"></p>' +
      '<p class="hint">type <code>browser open &lt;url&gt;</code> in the terminal</p>' +
      '</div>';
    refreshEngine();
  }

  async function refreshEngine() {
    try {
      const engine = await hostCall('host.browser.engine');
      const state = engine.state || 'unknown';
      const name = el('browser-engine');
      if (name) name.textContent = engine.engine + ' ' + engine.version;
      const status = el('browser-status');
      if (status) {
        status.textContent = state === 'ready'
          ? 'ready' + (engine.experimental ? ' (experimental)' : '')
          : 'unavailable: ' + (engine.reason || 'no reason given');
      }
      el('engine-status').textContent = 'engine: ' + engine.engine + ' · ' + state;
    } catch (error) {
      el('engine-status').textContent = 'engine: unreachable';
    }
  }

  async function refreshStatus() {
    try {
      const status = await hostCall('host.status');
      el('status').textContent = 'core: ' + status.core;
      const caps = await hostCall('host.browser.capabilities');
      const granted = Object.keys(caps).filter((key) => caps[key] === true);
      el('cap-status').textContent = 'caps: ' + (granted.join(' ') || 'none');
    } catch (error) {
      el('status').textContent = 'core: unreachable';
    }
    await refreshEngine();
  }

  global.Oko.panes = {
    write: write,
    openTerminal: openTerminal,
    runLine: runLine,
    openWebPane: openWebPane,
    showEngineStatus: showEngineStatus,
    refreshStatus: refreshStatus,
    refreshEngine: refreshEngine,
  };
})(window);
