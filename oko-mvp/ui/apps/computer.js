// ỌKỌ computer — multi-panel console.
(function () {
  'use strict';
  const hostCall = window.Oko.hostCall;
  window.Oko.say = say;
  const el = (id) => document.getElementById(id);

  let sessionId = null;

  function say(kind, text) {
    const line = document.createElement('div');
    line.className = 'line ' + kind;
    line.textContent = text;
    el('transcript').appendChild(line);
    el('transcript').scrollTop = el('transcript').scrollHeight;
  }

  async function ask(text) {
    if (!text) return;
    say('you', text);
    try {
      const answer = await hostCall('computer.ask', { text: text, session_id: sessionId });
      if (answer.session_id) {
        sessionId = answer.session_id;
        el('r-session').textContent = sessionId;
      }
      say('it', answer.reply);
    } catch (error) {
      say('err', 'the host did not answer: ' + error.message);
    }
  }

  async function loadStatus() {
    try {
      const status = await hostCall('host.status');
      el('s-core').textContent = status.core || 'ready';
      el('s-ui').textContent = status.ui || '?';
      el('r-host').textContent = status.core + ' · ' + status.ui;
      const sessions = await hostCall('host.terminal.read', {}).catch(() => null);
      el('s-sessions').textContent = sessions ? 'active' : 'none';
    } catch (e) { el('s-core').textContent = 'unreachable'; }
  }

  async function loadEngine() {
    try {
      const engine = await hostCall('host.browser.engine');
      el('e-name').textContent = engine.engine;
      el('e-version').textContent = engine.version;
      el('e-state').textContent = engine.state || '?';
      el('e-exp').textContent = engine.experimental ? 'yes' : 'no';
      el('r-engine').textContent = engine.engine + ' ' + engine.version + ' · ' + (engine.state || '?');
    } catch (e) { el('e-name').textContent = 'unreachable'; }
  }

  async function loadCapabilities() {
    try {
      const caps = await hostCall('host.browser.capabilities');
      el('cap-list').textContent = (caps.capabilities || []).join(', ') || 'none';
    } catch (e) { el('cap-list').textContent = 'unreachable'; }
  }

  async function loadApps() {
    const list = el('app-list');
    list.innerHTML = '';
    try {
      const res = await hostCall('host.app.list');
      (res.apps || []).forEach((a) => {
        const li = document.createElement('li');
        const span = document.createElement('span');
        span.textContent = a.name || a.id || '?';
        const btn = document.createElement('button');
        btn.textContent = 'LAUNCH';
        btn.addEventListener('click', () => hostCall('host.app.launch', { id: a.id })
          .then((res) => { if (res.url) openApp(a.name || a.id, res.url); else say('it', 'launched ' + (a.name || a.id)); })
          .catch((e) => say('err', e.message)));
        li.appendChild(span);
        li.appendChild(btn);
        list.appendChild(li);
      });
      if (!list.children.length) {
        const li = document.createElement('li');
        li.textContent = 'no applications';
        list.appendChild(li);
      }
    } catch (e) {
      const li = document.createElement('li');
      li.textContent = 'unreachable';
      list.appendChild(li);
    }
  }

  async function doCapture() {
    try {
      const res = await hostCall('render.capture', { path: '/tmp/oko-capture.ppm' });
      el('r-capture').textContent = res.path || 'captured';
      el('r-frames').textContent = (res.frame || 0).toString();
    } catch (e) { el('r-capture').textContent = 'failed'; }
  }

  function wave() {
    const c = el('wave');
    if (!c) return;
    const ctx = c.getContext('2d');
    const w = c.width, h = c.height;
    let t = 0;
    setInterval(() => {
      ctx.fillStyle = '#000';
      ctx.fillRect(0, 0, w, h);
      ctx.strokeStyle = '#ff9c00';
      ctx.beginPath();
      for (let x = 0; x < w; x++) {
        const y = h / 2 + Math.sin((x + t) * 0.05) * (h / 3) + Math.sin((x + t) * 0.13) * (h / 8);
        if (x === 0) ctx.moveTo(x, y); else ctx.lineTo(x, y);
      }
      ctx.stroke();
      t += 4;
    }, 60);
  }


  function openApp(name, url) {
    el('app-title').textContent = name;
    el('app-frame').src = url;
    el('app-overlay').hidden = false;
  }

  function closeApp() {
    el('app-frame').src = 'about:blank';
    el('app-overlay').hidden = true;
  }

  function wire() {
    el('prompt').addEventListener('submit', (event) => {
      event.preventDefault();
      const input = el('ask');
      const text = input.value.trim();
      input.value = '';
      if (text) ask(text);
      input.focus();
    });

    el('btn-engine').addEventListener('click', loadEngine);
    el('btn-apps').addEventListener('click', loadApps);
    el('btn-capture').addEventListener('click', doCapture);
    el('btn-status').addEventListener('click', () => ask('status'));
    el('app-close').addEventListener('click', closeApp);
    el('btn-files').addEventListener('click', () => window.OkoFiles.load());
    el('btn-save').addEventListener('click', () => window.OkoFiles.save());

    setInterval(() => { el('clock').textContent = new Date().toLocaleTimeString(); el('s-time').textContent = new Date().toLocaleTimeString(); }, 1000);

    loadStatus(); loadEngine(); loadCapabilities(); loadApps();
    window.OkoFiles.load();
    wave();
    ask('status');
  }

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', wire);
  } else {
    wire();
  }
})();
