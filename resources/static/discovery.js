import { createPassport } from './passport.js';

function track(kind, memberId = null, channel = null) {
  fetch('/api/events', {
    method: 'POST', headers: { 'content-type': 'application/json' }, keepalive: true,
    body: JSON.stringify({ kind, member_id: memberId, channel })
  }).catch(() => {});
}

let passport;
try {
  passport = createPassport(window.localStorage, () => new Date());
  const probe = '__boringbay_storage_probe__';
  window.localStorage.setItem(probe, '1');
  window.localStorage.removeItem(probe);
} catch (_) {
  document.querySelectorAll('.js-passport').forEach(element => { element.hidden = true; });
}

if (passport) {
  document.querySelectorAll('.js-passport').forEach(element => { element.hidden = false; });
  const update = () => {
    const state = passport.state();
    document.querySelectorAll('[data-member-card], .route-stop').forEach(card => {
      const visited = state.visitedMemberIds.includes(Number(card.dataset.memberId));
      card.classList.toggle('is-visited', visited);
      const stamp = card.querySelector('.route-stamp');
      if (stamp) { stamp.textContent = visited ? '◉' : '○'; stamp.setAttribute('aria-label', visited ? '已经访问' : '尚未访问'); }
    });
    document.querySelectorAll('[data-favorite]').forEach(button => {
      const favorite = state.favorites.includes(Number(button.dataset.favorite));
      button.textContent = favorite ? '★' : '☆';
      button.setAttribute('aria-pressed', String(favorite));
    });
    const summary = document.querySelector('[data-passport-summary]');
    if (summary) summary.textContent = `本地护照：到访 ${state.visitedMemberIds.length} 站 · 收藏 ${state.favorites.length} 站 · 成就 ${state.unlockedAchievements.length} 枚`;
    const route = document.querySelector('[data-route-date]');
    if (route) {
      const ids = [...document.querySelectorAll('.route-stop')].map(card => Number(card.dataset.memberId));
      const stamps = ids.filter(id => state.visitedMemberIds.includes(id)).length;
      document.querySelector('[data-route-stamps]').textContent = `${stamps} / ${ids.length} 枚邮戳`;
      document.querySelector('[data-route-complete]').disabled = stamps !== ids.length;
    }
  };
  document.querySelectorAll('.member-outbound').forEach(link => link.addEventListener('click', () => {
    passport.visit(Number(link.dataset.memberId), (link.dataset.tags || '').split(',').filter(Boolean));
    track('member_outbound', Number(link.dataset.memberId), link.dataset.overrideChannel || link.dataset.channel || 'unknown');
    update();
  }));
  document.querySelectorAll('.feed-outbound').forEach(link => link.addEventListener('click', () => {
    track('feed_outbound', Number(link.dataset.memberId), 'feed');
  }));
  document.querySelectorAll('[data-favorite]').forEach(button => button.addEventListener('click', () => {
    passport.toggleFavorite(button.dataset.favorite);
    update();
  }));
  document.querySelector('[data-random-discovery]')?.addEventListener('click', () => {
    track('random_use');
    const state = passport.state();
    const candidates = [...document.querySelectorAll('[data-member-card] .member-outbound')];
    const target = candidates.find(link => !state.visitedMemberIds.includes(Number(link.dataset.memberId))) || candidates[0];
    if (target) {
      target.dataset.overrideChannel = 'random';
      target.click();
      delete target.dataset.overrideChannel;
    }
  });
  document.querySelector('[data-route-complete]')?.addEventListener('click', () => {
    const route = document.querySelector('[data-route-date]');
    if (route) passport.completeRoute(route.dataset.routeDate);
    track('route_complete');
    update();
  });
  document.querySelector('[data-share-route]')?.addEventListener('click', async event => {
    const button = event.currentTarget;
    const url = button.dataset.shareUrl || window.location.href;
    track('share_click');
    try {
      if (navigator.share) await navigator.share({ title: '无聊湾今日航线', text: '我走完了，你来试试？', url });
      else { await navigator.clipboard.writeText(url); button.textContent = '链接已复制'; }
    } catch (_) { /* user cancellation is not an application error */ }
  });
  document.querySelector('[data-passport-reset]')?.addEventListener('click', () => {
    if (window.confirm('确定清除这台浏览器中的护照、收藏和成就吗？')) { passport.reset(); update(); }
  });
  update();
  if (document.querySelector('[data-route-date]')) track('route_start');
}
