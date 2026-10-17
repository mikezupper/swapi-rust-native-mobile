// DOM interaction smoke checks, not a browser, native UI, or visual test.
// Requires jsdom 26.1.0; see README.md for an isolated install command.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const { JSDOM } = require('jsdom');

(async () => {
  const html = fs.readFileSync(path.join(__dirname, 'index.html'), 'utf8');
  const fixture = JSON.parse(fs.readFileSync(path.join(__dirname, 'people.json'), 'utf8'));
  const dom = new JSDOM(html, { runScripts: 'outside-only', url: 'http://localhost:8765/' });
  const { window } = dom;
  const $ = selector => window.document.querySelector(selector);
  const $$ = selector => window.document.querySelectorAll(selector);
  window.fetch = async url => {
    assert.equal(url, './people.json', 'Only local fixture data is requested');
    return { ok: true, json: async () => fixture };
  };
  window.matchMedia = () => ({ matches: true });
  window.HTMLElement.prototype.scrollIntoView = () => {};
  const input = value => {
    $('#search-input').value = value;
    $('#search-input').dispatchEvent(new window.Event('input', { bubbles: true }));
  };
  const mode = value => {
    $('#state-select').value = value;
    $('#state-select').dispatchEvent(new window.Event('change', { bubbles: true }));
  };
  await window.eval(fs.readFileSync(path.join(__dirname, 'app.js'), 'utf8'));
  assert.equal(window.document.documentElement.dataset.previewReady, 'true');
  assert.equal($$('#search-results .person-row').length, 6);
  assert.equal($('#character-title').textContent, 'Luke Skywalker');
  input('  VaDeR  ');
  assert.equal($$('#search-results .person-row').length, 1);
  $('#search-results .person-row').click();
  assert.equal($('#character-title').textContent, 'Darth Vader');
  assert.match($('#detail-content .facts').textContent, /202/);
  $('#detail-content [data-action="back"]').click();
  assert.equal(window.document.activeElement, $('#search-input'));
  input('<img src=x onerror=alert(1)>');
  assert.equal($$('#search-results img').length, 0);
  assert.match($('#search-results').textContent, /No signal/);
  $('#search-results [data-action="clear"]').click();
  assert.equal($('#search-input').value, '');
  assert.equal($$('#search-results .person-row').length, 6);
  mode('empty');
  assert.equal($$('#search-results .person-row').length, 0);
  $('#search-results [data-action="clear"]').click();
  assert.equal($('#state-select').value, 'content');
  mode('error');
  assert.equal($$('.state-panel [data-action="retry"]').length, 3);
  $('[data-action="retry"]').click();
  assert.equal($$('.state-panel [data-action="retry"]').length, 0);
  assert.equal($$('#search-results .person-row').length, 6);
  mode('loading');
  assert.equal($$('.skeleton[role="status"]').length, 3);
  input('Leia');
  assert.equal($('#state-select').value, 'content');
  $('#search-results .person-row').click();
  assert.equal($('#character-title').textContent, 'Leia Organa');
  assert.match($('#detail-content .facts').textContent, /150/);
  $('[data-platform="android"][aria-pressed]').click();
  assert.equal($('#concepts').dataset.platform, 'android');
  assert.equal($('#detail-content [data-action="back"] path').getAttribute('d'), 'M20 12H4m7-7-7 7 7 7');
  $('[data-theme="light"][aria-pressed]').click();
  assert.equal($('#concepts').dataset.theme, 'light');
  $('#reset').click();
  assert.equal($('#concepts').dataset.platform, 'ios');
  assert.equal($('#concepts').dataset.theme, 'dark');
  assert.equal($('#character-title').textContent, 'Luke Skywalker');
  assert.equal($('#search-input').value, '');
  assert.equal($$('#search-results .person-row').length, 6);
  dom.window.close();
  console.log('OK: fixture loading, search, escaping, selection/back focus, empty/error/loading recovery, platform/theme controls, and reset.');
  console.log('DOM simulation only. Layout, scrolling, native materials, and device behavior are not verified.');
})().catch(error => { console.error(error); process.exitCode = 1; });
