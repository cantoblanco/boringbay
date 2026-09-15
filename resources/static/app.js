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

  const list = document.querySelector('[data-activity-list]');
  const pause = document.querySelector('[data-activity-pause]');
  if (!list || !pause || !('WebSocket' in window)) return;
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
    list.querySelector('.activity-empty')?.remove();
    const item = document.createElement('li');
    item.append(document.createTextNode(`来自「${data.country || '未知地区'}」的访客`));
    const link = document.createElement('a');
    link.href = `https://${data.member.domain}`;
    link.target = '_blank';
    link.rel = 'noopener';
    link.textContent = data.member.name;
    item.append(document.createTextNode(data.vt === 2 ? '访问了 ' : '从 '), link,
      document.createTextNode(data.vt === 2 ? '。' : ' 来到了无聊湾。'));
    list.prepend(item);
    const limit = window.innerWidth < 620 ? 3 : 10;
    while (list.children.length > limit) list.lastElementChild.remove();
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
