'use strict';
// Which app this Electron runs: Hearth's own window (main.js), or, with --hearth-theater, Hearth Theater's windows
// (hosts/hearth-theater/electron, carried in resources/theater; started by /usr/bin/hearth-theater). One Chromium
// for both, so Theater draws wherever Hearth's Chromium window does (on Nvidia graphics too). docs/THEATER.md.
const path = require('path');

if (process.argv.includes('--hearth-theater')) {
  const { app } = require('electron');
  require(app.isPackaged ? path.join(process.resourcesPath, 'theater', 'electron', 'main.js') : path.join(__dirname, '..', '..', 'hearth-theater', 'electron', 'main.js'));
} else {
  require('./main.js');
}
