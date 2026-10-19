// ỌKỌ computer — the console that talks back (SRD §7.3).
//
// Every request goes to the host as one typed, versioned message; the host
// decides what the words mean (domain::computer) and answers. This file owns
// no grammar: it shows the conversation and the readouts.
//
// Speech is offered only if the engine has it. Servo has no speech synthesis,
// so the control is hidden rather than shown doing nothing.

(function () {
  'use strict';

  const hostCall = window.Oko.hostCall;
  const el = (id) => document.getElementById(id);

  let sessionId = null;
  let speaking = false;

  function say(kind, text) {
    const line = document.createElement('div');
    line.className = 'line ' + kind;
    line.textContent = text;
    el('transcript').appendChild(line);
    el('transcript').scrollTop = el('transcript').scrollHeight;
    if (kind === 'it' && speaking && window.speechSynthesis) {
      window.speechSynthesis.speak(new SpeechSynthesisUtterance(text));
    }
  }

  async function ask(text) {
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

  async function readouts() {
    try {
      const status = await hostCall('host.status');
      el('r-host').textContent = status.core + ' · ' + status.ui;
      const engine = await hostCall('host.browser.engine');
      el('r-engine').textContent = engine.engine + ' ' + engine.version + ' · ' + (engine.state || '?');
    } catch (error) {
      el('r-host').textContent = 'unreachable';
      el('r-engine').textContent = 'unreachable';
    }
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

    if (window.speechSynthesis) {
      const button = el('speak');
      button.hidden = false;
      button.addEventListener('click', () => {
        speaking = !speaking;
        button.textContent = speaking ? 'speaking on' : 'speak replies';
      });
    }

    setInterval(() => { el('clock').textContent = new Date().toLocaleTimeString(); }, 1000);

    // A computer boots and reports. The first thing on the console is the host
    // answering for itself, which also proves the token and the API work.
    readouts();
    ask('status');
  }

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', wire);
  } else {
    wire();
  }
})();
