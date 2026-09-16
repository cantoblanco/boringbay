const STORAGE_KEY = 'boringbay.passport.v1';

function blankState() {
  return {
    version: 1,
    visitedMemberIds: [],
    favorites: [],
    routeCompletions: {},
    explorationDays: [],
    unlockedAchievements: []
  };
}

function shanghaiDateKey(date) {
  const parts = new Intl.DateTimeFormat('en-CA', {
    timeZone: 'Asia/Shanghai', year: 'numeric', month: '2-digit', day: '2-digit'
  }).formatToParts(date);
  const value = Object.fromEntries(parts.map(part => [part.type, part.value]));
  return `${value.year}-${value.month}-${value.day}`;
}

function shanghaiHour(date) {
  const hour = new Intl.DateTimeFormat('en-GB', {
    timeZone: 'Asia/Shanghai', hour: '2-digit', hourCycle: 'h23'
  }).formatToParts(date).find(part => part.type === 'hour')?.value;
  return Number(hour);
}

function hasThreeDayStreak(days) {
  const values = [...new Set(days)].sort();
  for (let index = 2; index < values.length; index += 1) {
    const current = new Date(`${values[index]}T00:00:00Z`);
    const previous = new Date(`${values[index - 1]}T00:00:00Z`);
    const first = new Date(`${values[index - 2]}T00:00:00Z`);
    if ((current - previous) / 86400000 === 1 && (previous - first) / 86400000 === 1) return true;
  }
  return false;
}

export function createPassport(storage = window.localStorage, clock = () => new Date()) {
  let state = blankState();
  function persist() { storage.setItem(STORAGE_KEY, JSON.stringify(state)); }
  function load() {
    try {
      const raw = storage.getItem(STORAGE_KEY);
      if (!raw) return state;
      const parsed = JSON.parse(raw);
      if (parsed?.version !== 1 || !Array.isArray(parsed.visitedMemberIds)) throw new Error('invalid');
      state = { ...blankState(), ...parsed };
    } catch (_) {
      state = blankState();
      try { persist(); } catch (_) {}
    }
    return state;
  }
  function unlock(id) {
    if (!state.unlockedAchievements.includes(id)) state.unlockedAchievements.push(id);
  }
  function visit(memberId, tags = [], options = {}) {
    const id = Number(memberId);
    if (!state.visitedMemberIds.includes(id)) state.visitedMemberIds.push(id);
    const now = clock();
    const day = shanghaiDateKey(now);
    if (!state.explorationDays.includes(day)) state.explorationDays.push(day);
    if (state.visitedMemberIds.length >= 1) unlock('first-voyage');
    if (state.visitedMemberIds.length >= 5) unlock('five-islands');
    if (shanghaiHour(now) <= 4) unlock('night-owl');
    if (options.lowExposure || tags.includes('hidden-gem')) unlock('hidden-gem');
    if (hasThreeDayStreak(state.explorationDays)) unlock('three-day-streak');
    persist();
    return state;
  }
  function toggleFavorite(memberId) {
    const id = Number(memberId);
    state.favorites = state.favorites.includes(id)
      ? state.favorites.filter(value => value !== id)
      : [...state.favorites, id];
    persist();
    return state.favorites.includes(id);
  }
  function completeRoute(date) {
    state.routeCompletions[String(date)] = true;
    persist();
  }
  function reset() { state = blankState(); persist(); return state; }
  function snapshot() { return JSON.parse(JSON.stringify(state)); }
  load();
  return { load, visit, toggleFavorite, completeRoute, achievements: () => [...state.unlockedAchievements], reset, state: snapshot };
}

export { STORAGE_KEY };
