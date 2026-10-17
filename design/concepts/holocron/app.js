/* Design-only interaction model. Product state and networking will live in Rust. */
(async () => {
  const $ = (selector) => document.querySelector(selector);
  const icons = {
    search: '<circle cx="10.5" cy="10.5" r="6.5"/><path d="m16 16 5 5"/>',
    compass: '<circle cx="12" cy="12" r="9"/><path d="m16 8-2.5 5.5L8 16l2.5-5.5Z"/>',
    people: '<circle cx="9" cy="7" r="3"/><path d="M3 20v-2a6 6 0 0 1 12 0v2M17 4a3 3 0 0 1 0 6m1 4a5 5 0 0 1 3 4v2"/>',
    arrow: '<path d="M5 12h14m-5-5 5 5-5 5"/>',
    chevron: '<path d="m9 5 7 7-7 7"/>',
    back: '<path d="m15 5-7 7 7 7"/>',
    androidBack: '<path d="M20 12H4m7-7-7 7 7 7"/>',
    signal: '<path d="m3 3 18 18M4 10a13 13 0 0 1 2-1m4-2a13 13 0 0 1 10 3M7 14a8 8 0 0 1 5-2m4 1 1 1m-7 4a3 3 0 0 1 4 0"/><circle cx="12" cy="21" r=".5"/>',
    archive: '<path d="M4 8h16v13H4zM3 3h18v5H3zM9 12h6"/>',
  };
  const icon = (name) => `<svg class="icon" viewBox="0 0 24 24" aria-hidden="true">${icons[name]}</svg>`;
  document.querySelectorAll('[data-icon]').forEach(el => { el.innerHTML = icon(el.dataset.icon); });
  const escape = (value) => String(value).replace(/[&<>"']/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
  let people;
  try {
    const response = await fetch('./people.json');
    if (!response.ok) throw new Error(`Fixture HTTP ${response.status}`);
    people = (await response.json()).people;
    if (!Array.isArray(people) || !people.length) throw new Error('Missing fixture records');
  } catch (error) {
    $('#preview-status').textContent = 'The sample data could not load. Serve this folder over HTTP using the README command, then reload.';
    console.error(error);
    return;
  }
  const state = { platform: 'ios', theme: 'dark', mode: 'content', selectedId: 1, origin: 'explore' };
  const monogram = name => name.includes('-') && !name.includes(' ') ? name : name.split(/\s+/).map(word => word[0]).join('').slice(0, 2);
  const humanValue = value => ({'n/a':'Not applicable','unknown':'Unknown','none':'None'}[value] || value);
  const row = person => `<button class="person-row" data-person="${person.id}" aria-label="View ${escape(person.name)}"><span class="avatar ${person.id % 3 === 2 ? 'warm' : person.id % 3 === 0 ? 'cool' : ''}" aria-hidden="true">${escape(monogram(person.name))}</span><span class="person-copy"><strong>${escape(person.name)}</strong><small>${person.filmUrls.length} films · Born ${escape(person.birthYear)}</small></span>${icon('chevron')}</button>`;
  const loading = (label, large = false) => `<div class="skeleton" role="status"><p class="skeleton-label">${label}</p><div class="skeleton-block ${large ? 'tall' : ''}" aria-hidden="true"></div><div class="skeleton-block" aria-hidden="true"></div><div class="skeleton-block" aria-hidden="true"></div></div>`;
  const errorPanel = () => `<div class="state-panel" role="status">${icon('signal')}<h4>A pause in the signal.</h4><p>We couldn’t reach the archive. Check your connection and try again.</p><button class="primary-button" data-action="retry">Try again ${icon('arrow')}</button></div>`;
  function renderExplore() {
    $('#explore-content').innerHTML = state.mode === 'loading' ? loading('Discovering the archive…', true) : state.mode === 'error' ? errorPanel() : `<button class="hero-card" data-person="1" aria-label="Explore Luke Skywalker"><img class="hero-art" src="assets/desert-orbit.png" alt="" width="1024" height="1536"><span class="hero-badge">THE CHARACTER EDIT / 001</span><span class="hero-copy"><span class="kicker">IN THE SPOTLIGHT</span><strong>Luke<br>Skywalker</strong><span class="hero-footer"><span>Meet the legend</span><span class="round-arrow">${icon('arrow')}</span></span></span></button><div class="section-label"><h4>Familiar faces</h4><button class="text-button" data-action="search">View all ${icon('arrow')}</button></div><div class="people-list">${[people.find(p => p.id === 5), people.find(p => p.id === 4)].map(row).join('')}</div>`;
  }
  function renderSearch() {
    const query = $('#search-input').value.trim().toLocaleLowerCase();
    $('#clear-search').hidden = !$('#search-input').value;
    if (state.mode === 'loading') { $('#search-results').innerHTML = loading('Searching the archive…'); return; }
    if (state.mode === 'error') { $('#search-results').innerHTML = errorPanel(); return; }
    const found = state.mode === 'empty' ? [] : people.filter(person => person.name.toLocaleLowerCase().includes(query));
    $('#search-results').innerHTML = found.length ? `<p class="result-label">${found.length} ${found.length === 1 ? 'character' : 'characters'}</p><div class="people-list">${found.map(row).join('')}</div><p class="search-footnote">Every name is a new connection.<br>Choose a character to explore their record.</p>` : `<div class="state-panel">${icon('search')}<h4>No signal for that name.</h4><p>No characters match${query ? ` “${escape($('#search-input').value.trim())}”` : ' this search'}. Try a different name.</p><button class="primary-button" data-action="clear">Clear search</button></div>`;
  }
  function renderDetail(animate = false) {
    const target = $('#detail-content');
    if (state.mode === 'loading') { target.innerHTML = loading('Opening character record…', true); return; }
    if (state.mode === 'error') { target.innerHTML = errorPanel(); return; }
    const person = people.find(p => p.id === state.selectedId);
    const backIcon = state.platform === 'ios' ? 'back' : 'androidBack';
    target.innerHTML = `<div class="${animate ? 'detail-enter' : ''}"><div class="detail-hero"><img class="hero-art" src="assets/desert-orbit.png" alt="" width="1024" height="1536"><div class="detail-top"><button class="icon-button" data-action="back" aria-label="Back to ${state.origin === 'search' ? 'Search' : 'Explore'}">${icon(backIcon)}</button><span>Character record</span></div><div class="detail-title"><p class="kicker">THE GALACTIC ARCHIVE / ${String(person.id).padStart(3, '0')}</p><h3 id="character-title" tabindex="-1">${escape(person.name)}</h3><p>One life. A galaxy of connections.</p></div></div><div class="detail-body"><dl class="facts"><div><dt>Height</dt><dd>${escape(person.height)} <small>cm</small></dd></div><div><dt>Mass</dt><dd>${escape(person.mass)} <small>kg</small></dd></div><div><dt>Born</dt><dd>${escape(person.birthYear.replace('BBY',''))}<small> BBY</small></dd></div></dl><h4>The details</h4><dl class="record"><div><dt>Eye color</dt><dd>${escape(humanValue(person.eyeColor))}</dd></div><div><dt>Hair color</dt><dd>${escape(humanValue(person.hairColor))}</dd></div><div><dt>Film appearances</dt><dd>${person.filmUrls.length} films</dd></div></dl><p class="archive-note">${icon('archive')}<span>Record from the Star Wars API.<br>BBY: Before the Battle of Yavin.</span></p></div></div>`;
  }
  function render() { renderExplore(); renderSearch(); renderDetail(); }
  function setMode(mode) { state.mode = mode; $('#state-select').value = mode; render(); }
  function moveTo(screen, focusSelector) {
    const reduceMotion = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
    $(`#${screen}-concept`).scrollIntoView({ behavior: reduceMotion ? 'instant' : 'smooth', block: 'nearest', inline: 'center' });
    if (focusSelector) $(focusSelector)?.focus({ preventScroll: true });
  }
  document.addEventListener('click', event => {
    const button = event.target.closest('button');
    if (!button) return;
    if (button.dataset.platform) {
      state.platform = button.dataset.platform;
      $('#concepts').dataset.platform = state.platform;
      document.querySelectorAll('[data-platform][aria-pressed]').forEach(el => el.setAttribute('aria-pressed', String(el === button)));
      $('#platform-note').textContent = state.platform === 'ios'
        ? 'iOS concept: floating translucent navigation, soft continuous shapes, and a chevron back control. SwiftUI will supply native materials, gestures, and accessibility.'
        : 'Android concept: anchored navigation with a pill selection indicator, generous rounded shapes, and an arrow back control. Compose will supply native gestures, ripples, and accessibility.';
      renderDetail();
    }
    if (button.dataset.theme) {
      state.theme = button.dataset.theme;
      $('#concepts').dataset.theme = state.theme;
      document.querySelectorAll('[data-theme][aria-pressed]').forEach(el => el.setAttribute('aria-pressed', String(el === button)));
    }
    if (button.dataset.person) {
      state.selectedId = Number(button.dataset.person);
      state.origin = button.closest('#search-concept') ? 'search' : 'explore';
      renderDetail(true);
      $('#detail-content').scrollTop = 0;
      moveTo('detail', '#character-title');
      $('#preview-status').textContent = `Detail updated: ${people.find(p => p.id === state.selectedId).name}. The back control returns to ${state.origin === 'search' ? 'Search' : 'Explore'}.`;
    }
    switch (button.dataset.action) {
      case 'search': moveTo('search', '#search-input'); break;
      case 'explore': moveTo('explore', '#explore-scroll'); break;
      case 'back': moveTo(state.origin, state.origin === 'search' ? '#search-input' : '#explore-scroll'); break;
      case 'retry': setMode('content'); $('#preview-status').textContent = 'Retry preview: sample records restored. No live network request was made.'; break;
      case 'clear': $('#search-input').value = ''; setMode('content'); $('#search-input').focus(); break;
    }
  });
  $('#search-input').addEventListener('input', () => {
    if (state.mode !== 'content') setMode('content');
    else renderSearch();
  });
  $('#search-form').addEventListener('submit', event => event.preventDefault());
  $('#clear-search').addEventListener('click', () => { $('#search-input').value = ''; setMode('content'); $('#search-input').focus(); });
  $('#state-select').addEventListener('change', event => setMode(event.target.value));
  $('#reset').addEventListener('click', () => {
    state.selectedId = 1; state.origin = 'explore';
    $('#search-input').value = '';
    document.querySelector('[data-platform="ios"][aria-pressed]').click();
    document.querySelector('[data-theme="dark"][aria-pressed]').click();
    setMode('content');
    document.querySelectorAll('.phone-scroll').forEach(el => { el.scrollTop = 0; });
    $('#preview-status').textContent = 'Preview reset. Tap a character to update Detail, or try a search.';
  });
  render();
  document.documentElement.dataset.previewReady = 'true';
})();
