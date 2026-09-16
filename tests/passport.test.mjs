import test from 'node:test';
import assert from 'node:assert/strict';
import { createPassport } from '../resources/static/passport.js';

function memoryStorage() {
  const data = new Map();
  return {
    getItem: key => data.get(key) ?? null,
    setItem: (key, value) => data.set(key, value),
    removeItem: key => data.delete(key)
  };
}

test('visit is idempotent and unlocks first voyage', () => {
  const passport = createPassport(memoryStorage(), () => new Date('2026-09-16T12:00:00+08:00'));
  passport.visit(12, ['tech']);
  passport.visit(12, ['tech']);
  assert.deepEqual(passport.state().visitedMemberIds, [12]);
  assert.ok(passport.achievements().includes('first-voyage'));
});

test('invalid storage resets safely', () => {
  const store = memoryStorage();
  store.setItem('boringbay.passport.v1', '{broken');
  const passport = createPassport(store, () => new Date('2026-09-16T12:00:00+08:00'));
  assert.deepEqual(passport.state().visitedMemberIds, []);
});

test('five islands, hidden gem and night owl unlock exactly once', () => {
  const passport = createPassport(memoryStorage(), () => new Date('2026-09-16T02:00:00+08:00'));
  for (let id = 1; id <= 5; id += 1) passport.visit(id, [], { lowExposure: id === 5 });
  assert.deepEqual(new Set(passport.achievements()), new Set(['first-voyage', 'five-islands', 'night-owl', 'hidden-gem']));
});
