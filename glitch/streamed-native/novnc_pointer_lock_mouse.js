/* GLITCH_NOVNC_POINTER_LOCK_MOUSE_V1 */
/* GLITCH_NOVNC_POINTER_LOCK_MOUSE_V1_PROMISE_COOLDOWN */
/* GLITCH_X11_MOUSE_BRIDGE_V1 */
import UI from './app/ui.js';

(function () {
  if (window.__glitchNoVNCPointerLockMouseV1 && window.__glitchNoVNCPointerLockMouseV1.version) return;

  const settings = Object.assign({
    xScale: 1.0,
    yScale: 1.0,
    maxDelta: 48.0,
  }, window.__glitchNoVNCPointerLockSettingsV1 || {});
  let activeRfb = null;
  let activeCanvas = null;
  let pointerLockRequestPending = false;
  let pointerLockCooldownUntil = 0;
  let x11MouseSocket = null;
  let x11MouseSocketReady = false;
  let x11MouseReconnectAt = 0;

  function log() {
    try {
      console.log.apply(console, ['[glitch-mouse]'].concat(Array.from(arguments)));
    } catch (e) {}
  }

  function clamp(value, maxAbs) {
    if (!Number.isFinite(value)) return 0;
    const limit = Math.max(0, Number(maxAbs) || 0);
    if (limit <= 0) return 0;
    if (value > limit) return limit;
    if (value < -limit) return -limit;
    return value;
  }

  function canvasCenter(canvas) {
    const rect = canvas.getBoundingClientRect();
    return {
      x: Math.max(0, rect.width / 2),
      y: Math.max(0, rect.height / 2),
    };
  }

  function buttonMask(button) {
    if (button < 0 || button > 7) return 0;
    return 1 << button;
  }

  function isLocked(canvas) {
    return document.pointerLockElement === canvas;
  }

  function stop(ev) {
    ev.preventDefault();
    ev.stopPropagation();
  }

  function requestLock(canvas) {
    if (!canvas || typeof canvas.requestPointerLock !== 'function') return false;
    if (isLocked(canvas)) return true;

    const now = Date.now();
    if (pointerLockRequestPending || now < pointerLockCooldownUntil) return false;

    pointerLockRequestPending = true;

    try {
      const result = canvas.requestPointerLock();

      if (result && typeof result.then === 'function') {
        result.then(function () {
          pointerLockRequestPending = false;
        }).catch(function (err) {
          pointerLockRequestPending = false;
          pointerLockCooldownUntil = Date.now() + 1500;
          log('pointer lock request rejected', err && err.message ? err.message : err);
          emit('glitch:novnc-pointer-lock-error', {
            settings: settings,
            error: err && err.message ? err.message : String(err || 'unknown'),
          });
        });
      } else {
        window.setTimeout(function () {
          pointerLockRequestPending = false;
        }, 250);
      }

      return true;
    } catch (err) {
      pointerLockRequestPending = false;
      pointerLockCooldownUntil = Date.now() + 1500;
      log('pointer lock request failed', err && err.message ? err.message : err);
      emit('glitch:novnc-pointer-lock-error', {
        settings: settings,
        error: err && err.message ? err.message : String(err || 'unknown'),
      });
      return false;
    }
  }

  function ensureX11MouseSocket() {
    const now = Date.now();
    if (x11MouseSocket && (x11MouseSocketReady || x11MouseSocket.readyState === WebSocket.CONNECTING)) return;
    if (now < x11MouseReconnectAt) return;

    x11MouseReconnectAt = now + 1500;

    try {
      const scheme = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
      x11MouseSocket = new WebSocket(scheme + '//' + window.location.host + '/glitch-x11-mouse-ws');
      x11MouseSocket.addEventListener('open', function () {
        x11MouseSocketReady = true;
        log('x11 mouse bridge connected');
      });
      x11MouseSocket.addEventListener('close', function () {
        x11MouseSocketReady = false;
        x11MouseReconnectAt = Date.now() + 1500;
      });
      x11MouseSocket.addEventListener('error', function () {
        x11MouseSocketReady = false;
        x11MouseReconnectAt = Date.now() + 1500;
      });
    } catch (err) {
      x11MouseSocketReady = false;
      x11MouseReconnectAt = Date.now() + 1500;
    }
  }

  function sendX11RelativeMouse(dx, dy) {
    ensureX11MouseSocket();
    if (!x11MouseSocketReady || !x11MouseSocket || x11MouseSocket.readyState !== WebSocket.OPEN) return false;

    try {
      x11MouseSocket.send(JSON.stringify({ dx: dx, dy: dy }));
      return true;
    } catch (err) {
      x11MouseSocketReady = false;
      return false;
    }
  }

  function emit(name, detail) {
    try {
      window.dispatchEvent(new CustomEvent(name, { detail: detail || {} }));
    } catch (e) {}
  }

  function sendRelativeMouse(dxRaw, dyRaw, maskOverride) {
    const rfb = activeRfb;
    const canvas = activeCanvas || (rfb && rfb._canvas);
    if (!rfb || !canvas || typeof rfb._sendMouse !== 'function') return false;

    const dx = clamp((Number(dxRaw) || 0) * settings.xScale, settings.maxDelta);
    const dy = clamp((Number(dyRaw) || 0) * settings.yScale, settings.maxDelta);
    if (dx === 0 && dy === 0) return false;

    const center = canvasCenter(canvas);
    const mask = Number.isFinite(Number(maskOverride)) ? Number(maskOverride) : (rfb._mouseButtonMask || 0);
    rfb._sendMouse(center.x + dx, center.y + dy, mask);
    sendX11RelativeMouse(dx, dy);
    return true;
  }

  function numberFrom(payload, names) {
    for (const name of names) {
      if (payload && Object.prototype.hasOwnProperty.call(payload, name)) {
        const value = Number(payload[name]);
        if (Number.isFinite(value)) return value;
      }
    }
    return 0;
  }

  function bridgeEventType(payload, fallbackType) {
    const raw = payload && (payload.type || payload.event || payload.name || payload.kind || fallbackType);
    return raw ? String(raw).toLowerCase() : '';
  }

  function handleExternalMouseDelta(payload, fallbackType) {
    if (typeof payload === 'string') {
      try {
        payload = JSON.parse(payload);
      } catch (e) {
        return false;
      }
    }

    const type = bridgeEventType(payload, fallbackType);
    const allowed = [
      'aegis:mouse-delta',
      'aegis:pointer-delta',
      'aegis-bridge:mouse-delta',
      'aegis-bridge:pointer-delta',
      'glitch:mouse-delta',
      'glitch:pointer-delta',
      'glitch-novnc-pointer-delta',
    ];

    if (!allowed.includes(type)) return false;

    const dx = numberFrom(payload, ['dx', 'movementX', 'deltaX', 'x']);
    const dy = numberFrom(payload, ['dy', 'movementY', 'deltaY', 'y']);
    const hasMask = !!payload && ['buttonMask', 'buttons', 'mask'].some(function (name) {
      return Object.prototype.hasOwnProperty.call(payload, name);
    });
    const mask = hasMask ? numberFrom(payload, ['buttonMask', 'buttons', 'mask']) : undefined;
    return sendRelativeMouse(dx, dy, mask);
  }

  function registerWithAegisBridge(api) {
    const candidates = [
      window.AegisBridge,
      window.aegisBridge,
      window.__aegisBridge,
      window.aegis_bridge,
    ].filter(Boolean);

    for (const bridge of candidates) {
      for (const method of ['registerMouseBridge', 'registerPointerBridge', 'registerInputBridge', 'setMouseBridge']) {
        if (typeof bridge[method] === 'function') {
          try {
            bridge[method](api);
            log('registered mouse bridge with Aegis via', method);
            return true;
          } catch (err) {
            log('Aegis bridge registration failed', err && err.message ? err.message : err);
          }
        }
      }
    }

    return false;
  }

  function patchRfb(rfb) {
    if (!rfb || rfb.__glitchPointerLockMousePatched) return false;

    const canvas = rfb._canvas;
    const originalHandler = rfb._eventHandlers && rfb._eventHandlers.handleMouse;

    if (!canvas || !originalHandler || typeof rfb._sendMouse !== 'function') return false;

    rfb.__glitchPointerLockMousePatched = true;
    activeRfb = rfb;
    activeCanvas = canvas;

    ['mousedown', 'mouseup', 'mousemove', 'click', 'contextmenu'].forEach(function (eventName) {
      canvas.removeEventListener(eventName, originalHandler);
    });

    function sendButton(ev, down) {
      const center = canvasCenter(canvas);
      const mask = buttonMask(ev.button);
      if (mask && typeof rfb._handleMouseButton === 'function') {
        rfb._handleMouseButton(center.x, center.y, down, mask);
      }
    }

    function handleMouse(ev) {
      if (ev.type === 'mousedown') {
        requestLock(canvas);
        if (!isLocked(canvas)) {
          originalHandler(ev);
          return;
        }
        stop(ev);
        sendButton(ev, true);
        return;
      }

      if (ev.type === 'mouseup') {
        if (!isLocked(canvas)) {
          originalHandler(ev);
          return;
        }
        stop(ev);
        sendButton(ev, false);
        return;
      }

      if (ev.type === 'mousemove') {
        if (!isLocked(canvas)) {
          stop(ev);
          return;
        }

        stop(ev);

        sendRelativeMouse(ev.movementX || 0, ev.movementY || 0, rfb._mouseButtonMask || 0);
        return;
      }

      if (ev.type === 'click' || ev.type === 'contextmenu') {
        stop(ev);
      }
    }

    rfb._eventHandlers.handleMouse = handleMouse;
    ['mousedown', 'mouseup', 'mousemove', 'click', 'contextmenu'].forEach(function (eventName) {
      canvas.addEventListener(eventName, handleMouse);
    });

    canvas.style.cursor = 'crosshair';
    canvas.title = 'Click to capture mouse';

    document.addEventListener('pointerlockchange', function () {
      if (isLocked(canvas)) {
        pointerLockRequestPending = false;
        log('pointer lock active');
        emit('glitch:novnc-pointer-lock-active', { settings: settings });
      } else {
        pointerLockRequestPending = false;
        pointerLockCooldownUntil = Date.now() + 1000;
        log('pointer lock released; click the game to capture mouse again');
        emit('glitch:novnc-pointer-lock-released', { settings: settings });
      }
    });

    document.addEventListener('pointerlockerror', function () {
      pointerLockRequestPending = false;
      pointerLockCooldownUntil = Date.now() + 1500;
      log('pointer lock denied; if this page is in an iframe, the iframe must allow pointer-lock');
      emit('glitch:novnc-pointer-lock-error', { settings: settings });
    });

    log('pointer lock mouse patch installed', settings);
    emit('glitch:novnc-pointer-lock-ready', { api: window.__glitchNoVNCPointerLockMouseV1, settings: settings });
    registerWithAegisBridge(window.__glitchNoVNCPointerLockMouseV1);
    return true;
  }

  function waitForRfb() {
    patchRfb(UI && UI.rfb);
    window.setTimeout(waitForRfb, 1000);
  }

  window.__glitchNoVNCPointerLockMouseV1 = {
    version: '1',
    settings: settings,
    patchRfb: patchRfb,
    requestPointerLock: function () {
      requestLock(activeCanvas || (activeRfb && activeRfb._canvas));
    },
    sendRelativeMouse: function (dx, dy, buttonMask) {
      return sendRelativeMouse(dx, dy, buttonMask);
    },
    isPointerLocked: function () {
      const canvas = activeCanvas || (activeRfb && activeRfb._canvas);
      return !!canvas && isLocked(canvas);
    },
    getRfb: function () {
      return activeRfb;
    },
  };

  window.addEventListener('message', function (ev) {
    handleExternalMouseDelta(ev.data);
  });

  [
    'aegis:mouse-delta',
    'aegis:pointer-delta',
    'aegis-bridge:mouse-delta',
    'aegis-bridge:pointer-delta',
    'glitch:mouse-delta',
    'glitch:pointer-delta',
    'glitch-novnc-pointer-delta',
  ].forEach(function (eventName) {
    window.addEventListener(eventName, function (ev) {
      handleExternalMouseDelta(ev && ev.detail ? ev.detail : {}, eventName);
    });
  });

  emit('glitch:novnc-pointer-lock-loading', { api: window.__glitchNoVNCPointerLockMouseV1, settings: settings });
  registerWithAegisBridge(window.__glitchNoVNCPointerLockMouseV1);
  waitForRfb();
})();
