'use strict';
// Collections: your own groups of games ("Couch co-op", "Finish these", "Kids"), shown as tabs in the Library.
// Kept in library.json, so each profile has its own. A game can be in any number of them.

const MAX = 30;

function list(data) { return Array.isArray(data.collections) ? data.collections : []; }

function create(store, name) {
  const clean = String(name || '').trim().slice(0, 40);
  if (!clean) throw new Error('Give the collection a name');
  if (list(store.data).length >= MAX) throw new Error(`Hearth has room for ${MAX} collections`);
  if (list(store.data).some(c => c.name.toLowerCase() === clean.toLowerCase())) throw new Error(`There's already a collection called ${clean}`);
  const c = { id: 'c' + Date.now().toString(36) + Math.random().toString(36).slice(2, 5), name: clean, games: [] };
  store.update(d => { d.collections = [...list(d), c]; });
  return c;
}
function rename(store, id, name) {
  const clean = String(name || '').trim().slice(0, 40);
  if (!clean) throw new Error('Give the collection a name');
  store.update(d => { const c = list(d).find(x => x.id === id); if (!c) throw new Error('No such collection'); c.name = clean; });
}
function remove(store, id) { store.update(d => { d.collections = list(d).filter(c => c.id !== id); }); }
function setGame(store, id, gameId, on) {
  store.update(d => {
    const c = list(d).find(x => x.id === id); if (!c) throw new Error('No such collection');
    const s = new Set(c.games); if (on) s.add(gameId); else s.delete(gameId); c.games = [...s];
  });
}
/** Which collections a game is in (ids). */
function of(data, gameId) { return list(data).filter(c => c.games.includes(gameId)).map(c => c.id); }
/** For the UI: [{id, name, count}]. */
function summary(data) { return list(data).map(c => ({ id: c.id, name: c.name, count: c.games.length })); }

module.exports = { create, rename, remove, setGame, of, summary, list };
