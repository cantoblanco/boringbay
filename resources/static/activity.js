function activityCopy(data) {
  if (!data?.member?.name || !data?.member?.domain) return null;
  const country = data.country || '未知地区';
  return data.vt === 2
    ? { before: `来自「${country}」的访客访问了 `, after: '。' }
    : { before: `来自「${country}」的访客从 `, after: ' 来到了无聊湾。' };
}

function appendActivity(document, parent, data, copy) {
  parent.append(document.createTextNode(copy.before));
  const link = document.createElement('a');
  link.href = `https://${data.member.domain}`;
  link.target = '_blank';
  link.rel = 'noopener';
  link.textContent = data.member.name;
  parent.append(link, document.createTextNode(copy.after));
}

export function renderActivity(environment, data) {
  const { document, list, toasts, viewportWidth, schedule } = environment;
  const copy = activityCopy(data);
  if (!copy || !list || !toasts) return false;

  list.querySelector('.activity-empty')?.remove();
  const item = document.createElement('li');
  appendActivity(document, item, data, copy);
  list.prepend(item);
  const listLimit = viewportWidth < 620 ? 3 : 10;
  while (list.children.length > listLimit) list.lastElementChild.remove();

  const toast = document.createElement('article');
  toast.className = 'activity-toast';
  toast.setAttribute('role', 'status');
  const title = document.createElement('strong');
  title.textContent = '无聊的沙雕 +1';
  const message = document.createElement('p');
  appendActivity(document, message, data, copy);
  toast.append(title, message);
  toasts.prepend(toast);
  const toastLimit = viewportWidth < 1234 ? 2 : 10;
  while (toasts.children.length > toastLimit) toasts.lastElementChild.remove();
  schedule(() => toast.remove(), 10_000);
  return true;
}
