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

  // The command bar: the terminal made invisible. Every line runs through the
  // same panes.runLine command model as the terminal (SRD §7.3) — the user
  // never sees a terminal, but every keystroke is a command on the core.
  function wireCommandBar() {
    const bar = el('command-bar');
    bar.addEventListener('keydown', (event) => {
      if (event.key !== 'Enter') return;
      const text = bar.value;
      bar.value = '';
      if (!text.trim()) return;
      panes.write('⌁ ' + text + '\n');
      panes.runLine(text).catch((error) => panes.write('error: ' + error.message + '\n'));
    });
  }

  // Stardate: 1000 units per Earth year, epoch 2000-01-01. Positive for any
  // date the console will ever see; a linear extrapolation from the TNG
  // 2364 = 41000 convention would go negative before 2323.
  function stardate(now) {
    const startOfYear = new Date(now.getFullYear(), 0, 1);
    const dayOfYear = (now - startOfYear) / 86400000;
    const fractionalYear = now.getFullYear() + dayOfYear / 365.25;
    return ((fractionalYear - 2000) * 1000).toFixed(1);
  }

  function wireClock() {
    const tick = () => {
      const now = new Date();
      el('clock').textContent = now.toLocaleTimeString();
      el('stardate').textContent = 'SD ' + stardate(now);
    };
    tick();
    setInterval(tick, 1000);
  }

  // Red alert is console chrome only: the core has no alert route, so the
  // trace says so instead of pretending the core changed state.
  function wireAlert() {
    let armed = false;
    el('btn-alert').addEventListener('click', () => {
      armed = !armed;
      document.body.classList.toggle('red-alert', armed);
      el('btn-alert').textContent = armed ? 'STAND DOWN' : 'ALERT';
      panes.write(armed
        ? '// RED ALERT — console chrome only; the core has no alert route\n'
        : '// alert cancelled — all decks report normal\n');
    });
  }
  // `?open=/apps/hello.html` or `?open=https://example.com`.
  function openRequestedPane() {
    const requested = new URLSearchParams(window.location.search).get('open');
    if (requested) panes.openWebPane(requested);
  }

  function wire() {
    wireTerminalInput();
    wireCommandBar();
    wireClock();
    wireAlert();
    wireSplitter();
    wireButtons();
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
