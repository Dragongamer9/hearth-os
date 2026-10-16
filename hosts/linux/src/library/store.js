'use strict';
// Nimbus's own per-user data: favorites, hidden games, manual games, play stats, enabled sources, settings.
// Saved as ~/.local/share/nimbus/library.json.

const path = require('node:path');
const { DATA_DIR, readJson, writeJsonAtomic } = require('../util');

const DEFAULTS = () => ({
  favorites: [], hidden: [], manual: [], stats: {},
  steamBackground: true,
  autoUpdateCheck: true,
  sources: { steam: true, epic: true, gog: true, xbox: true },
});

class Store {
  constructor(file = path.join(DATA_DIR, 'library.json')) {
    this.file = file;
    const saved = readJson(file); // a corrupt file starts fresh rather than crash the shell
    const d = DEFAULTS();
    this.data = saved && typeof saved === 'object' ? { ...d, ...saved, sources: { ...d.sources, ...(saved.sources || {}) } } : d;
  }
  update(change) { change(this.data); writeJsonAtomic(this.file, this.data); }
  sourceOn(k) { return this.data.sources[k] !== false; }
  setIn(listName, id, on) {
    this.update(d => {
      const s = new Set(d[listName]);
      if (on) s.add(id); else s.delete(id);
      d[listName] = [...s];
    });
  }
}

module.exports = { Store };
