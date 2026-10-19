// ỌKỌ computer — file explorer and text editor panels.
(function () {
  'use strict';
  const hostCall = window.Oko.hostCall;
  const el = (id) => document.getElementById(id);
  const say = window.Oko.say;

  let currentFile = null;

  function load() {
    const list = el('file-list');
    list.innerHTML = '';
    hostCall('host.fs.list', { path: '.' })
      .then((res) => {
        (res.entries || []).forEach((e) => {
          const li = document.createElement('li');
          li.textContent = (e.dir ? '[d] ' : '    ') + e.name;
          li.addEventListener('click', () => open(e.name, e.dir));
          list.appendChild(li);
        });
      })
      .catch(() => {
        const li = document.createElement('li');
        li.textContent = 'unreachable';
        list.appendChild(li);
      });
  }

  function open(name, dir) {
    if (dir) {
      el('f-cwd').textContent = name;
      return;
    }
    hostCall('host.fs.read', { path: name })
      .then((res) => {
        el('ed-name').textContent = name;
        el('ed-text').value = res.text || '';
        currentFile = name;
      })
      .catch((e) => say('err', 'cannot read ' + name + ': ' + e.message));
  }

  function save() {
    if (!currentFile) {
      say('err', 'no file open');
      return;
    }
    hostCall('host.fs.write', { path: currentFile, text: el('ed-text').value })
      .then(() => say('it', 'saved ' + currentFile))
      .catch((e) => say('err', 'save failed: ' + e.message));
  }

  window.OkoFiles = { load: load, save: save };
})();
