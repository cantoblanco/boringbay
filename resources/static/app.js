import { renderActivity } from './activity.js';

(() => {
  'use strict';
  const root = document.documentElement;
  const storageKey = 'boringbay.theme.v1';
  const preferredDark = window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches;
  let savedTheme = null;
  try { savedTheme = localStorage.getItem(storageKey); } catch (_) {}
  root.dataset.theme = savedTheme || (preferredDark ? 'dark' : 'light');
  document.querySelector('[data-theme-toggle]')?.addEventListener('click', () => {
    root.dataset.theme = root.dataset.theme === 'dark' ? 'light' : 'dark';
    try { localStorage.setItem(storageKey, root.dataset.theme); } catch (_) {}
  });

  document.querySelectorAll('img[data-fallback]').forEach(image => {
    image.addEventListener('error', () => {
      const fallback = image.dataset.fallback;
      if (fallback && image.src !== new URL(fallback, window.location.href).href) image.src = fallback;
    }, { once: true });
  });

  const list = document.querySelector('[data-activity-list]');
  const toasts = document.querySelector('[data-activity-toasts]');
  const pause = document.querySelector('[data-activity-pause]');
  if (!list || !toasts || !pause || !('WebSocket' in window)) return;
  let paused = false;
  let retries = 0;
  let socket = null;
  const retryDelays = [3000, 6000, 12000, 30000];
  pause.addEventListener('click', () => {
    paused = !paused;
    pause.setAttribute('aria-pressed', String(paused));
    pause.textContent = paused ? '继续' : '暂停';
  });
  function addActivity(data) {
    if (paused || !data || !data.member) return;
    renderActivity({
      document,
      list,
      toasts,
      viewportWidth: window.innerWidth,
      schedule: window.setTimeout.bind(window)
    }, data);
  }
  function connect() {
    const protocol = location.protocol === 'https:' ? 'wss:' : 'ws:';
    socket = new WebSocket(`${protocol}//${location.host}/api/ws`);
    socket.addEventListener('open', () => { retries = 0; });
    socket.addEventListener('message', event => {
      try { addActivity(JSON.parse(event.data)); } catch (_) {}
    });
    socket.addEventListener('close', () => {
      const delay = retryDelays[Math.min(retries++, retryDelays.length - 1)];
      window.setTimeout(connect, delay);
    });
    socket.addEventListener('error', () => socket?.close());
  }
  connect();
})();
