// ỌKỌ host API client (SRD UI-008).
//
// One transport, one place to get it wrong. The shell is served by oko-host
// over loopback HTTP, so this works in any engine that renders HTML and runs
// JavaScript: Servo, Chromium, or whatever browser the machine has. That is
// what makes the engine interchangeable (SRD §5.2).
//
// Every call carries the session token the host injected into this page. The
// host refuses a call without it, so a page this host did not serve cannot
// drive it (SRD SEC-002, SEC-014, UI-009).

(function (global) {
  'use strict';

  const token = (document.querySelector('meta[name="oko-token"]') || {}).content || '';
  let sequence = 0;

  // Send one typed, versioned message and return its payload. Throws on a
  // refusal or a host error, so a caller cannot mistake failure for data.
  async function hostCall(route, payload) {
    const message = {
      v: 1,
      id: String(++sequence),
      direction: 'request',
      route: route,
      payload: payload || {},
    };
    const reply = await fetch('/api', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json', 'X-Oko-Token': token },
      body: JSON.stringify(message),
    });
    if (!reply.ok) {
      throw new Error('host refused ' + route + ': HTTP ' + reply.status);
    }
    const answer = await reply.json();
    if (answer.error) {
      throw new Error(answer.error);
    }
    return answer.payload;
  }

  global.Oko = { hostCall: hostCall };
})(window);
