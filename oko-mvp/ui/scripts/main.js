// ỌKỌ desktop shell (SRD UI-002, UI-003, §7.3).
//
// The desktop chrome: the top bar, the splitter, the clock and the boot
// sequence. What a pane renders lives in panes.js; how a message reaches the
// host lives in host-api.js. This file is only the workspace around them.

(function () {
  'use strict';

  const panes = window.Oko.panes;
  const el = (id) => document.getElementById(id);

  function wireTerminalInput() {
    const input = el('terminal-input');
    input.addEventListener('keydown', (event) => {
      if (event.key !== 'Enter') return;
      const text = input.value;
      input.value = '';
      panes.write('❯ ' + text + '\n');
      panes.runLine(text).catch((error) => panes.write('error: ' + error.message + '\n'));
    });
  }

  // The two panes are peers: either can be widened, and the splitter remembers
  // nothing, because a layout that outlives a reload is a separate decision.
  function wireSplitter() {
    const splitter = el('splitter');
    let dragging = false;
    splitter.addEventListener('mousedown', () => {
      dragging = true;
      splitter.classList.add('dragging');
    });
    document.addEventListener('mouseup', () => {
      dragging = false;
      splitter.classList.remove('dragging');
    });
    document.addEventListener('mousemove', (event) => {
      if (!dragging) return;
      const width = el('workspace').getBoundingClientRect().width;
      const ratio = Math.min(0.8, Math.max(0.2, event.clientX / width));
      el('panel-terminal').style.width = (ratio * 100) + '%';
      splitter.style.left = (ratio * 100) + '%';
    });
  }

  function wireButtons() {
    el('btn-home').addEventListener('click', panes.showEngineStatus);
    el('btn-open').addEventListener('click', () => {
      const url = window.prompt('open url', '/apps/hello.html');
      if (url) {
        panes.runLine('browser open ' + url).catch((e) => panes.write('error: ' + e.message + '\n'));
      }
    });
    el('btn-launcher').addEventListener('click', () => {
      panes.runLine('apps').catch((error) => panes.write('error: ' + error.message + '\n'));
    });
    el('btn-settings').addEventListener('click', () => {
      panes.refreshStatus().catch(() => {});
    });
  }

  // A launcher can ask for a pane to be open at startup:
  // `?open=/apps/hello.html` or `?open=https://example.com`.
  function openRequestedPane() {
    const requested = new URLSearchParams(window.location.search).get('open');
    if (requested) panes.openWebPane(requested);
  }

  function wire() {
    wireTerminalInput();
    wireSplitter();
    wireButtons();
    setInterval(() => { el('clock').textContent = new Date().toLocaleTimeString(); }, 1000);
    openRequestedPane();
    panes.openTerminal()
      .then(panes.refreshStatus)
      .catch((error) => {
        el('status').textContent = 'core: unreachable';
        panes.write('could not reach the host: ' + error.message + '\n');
      });
  }

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', wire);
  } else {
    wire();
  }
})();
