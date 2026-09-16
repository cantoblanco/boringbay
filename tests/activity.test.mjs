import test from 'node:test';
import assert from 'node:assert/strict';
import { renderActivity } from '../resources/static/activity.js';

class FakeNode {
  constructor(tagName = '') {
    this.tagName = tagName;
    this.children = [];
    this.attributes = new Map();
    this.removed = false;
    this.textContent = '';
  }

  append(...nodes) { this.children.push(...nodes); }
  prepend(node) { this.children.unshift(node); }
  remove() { this.removed = true; }
  setAttribute(name, value) { this.attributes.set(name, value); }
  querySelector() { return null; }
  get lastElementChild() { return this.children.at(-1); }
}

const document = {
  createElement: tagName => new FakeNode(tagName),
  createTextNode: text => ({ textContent: text })
};

function flattenText(node) {
  return [node.textContent || '', ...(node.children || []).map(flattenText)].join('');
}

test('a websocket visit updates the rail and creates an expiring privacy-safe popup', () => {
  const list = new FakeNode('ol');
  const toasts = new FakeNode('div');
  const timers = [];
  const rendered = renderActivity({
    document,
    list,
    toasts,
    viewportWidth: 1440,
    schedule: (callback, delay) => timers.push({ callback, delay })
  }, {
    country: 'ES',
    member: { name: '测试博客', domain: 'example.com' },
    vt: 2,
    ip: '203.****.9'
  });

  assert.equal(rendered, true);
  assert.equal(list.children.length, 1);
  assert.equal(toasts.children.length, 1);
  assert.match(flattenText(toasts.children[0]), /无聊的沙雕 \+1/);
  assert.match(flattenText(toasts.children[0]), /ES/);
  assert.match(flattenText(toasts.children[0]), /203\.\*\*\*\*\.9/);
  assert.match(flattenText(toasts.children[0]), /测试博客/);
  assert.doesNotMatch(flattenText(toasts.children[0]), /203\.0\.113\.9/);
  assert.equal(timers[0].delay, 10_000);
  timers[0].callback();
  assert.equal(toasts.children[0].removed, true);
});
