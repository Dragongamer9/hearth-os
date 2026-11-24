# Hearth Files does everything Windows 11's File Explorer does (first 6 screens, approved by the owner Nov 21, 2026)

Mockups: files-explorer/files-explorer-1-main-window.png, files-explorer/files-explorer-2-view.png, files-explorer/files-explorer-3-right-click.png, files-explorer/files-explorer-4-copying.png, files-explorer/files-explorer-5-properties.png, files-explorer/files-explorer-6-search.png
Release: next

Sources: hearth-workspace hearth/mockups/files-explorer. More screens follow (Home, This PC, Network, Folder options and
the rest); each part is ticked only once it's built and pictured in CI (hearth-apps-rust.yml).

## 1 · Main window
- [ ] Tabs in the title bar with their folder's icon, close on each, + for a new tab (Ctrl+T, Ctrl+W)
- [ ] A tab dragged out becomes its own window; files dropped on a tab go to its folder
- [ ] The navigation tree: Home, Favourites, Quick access (pinned, with the pin mark), This PC with its drives expandable, Network, Trash
- [ ] The address bar's steps with › between them; Search <folder> box
- [ ] Command bar: New ▾, Cut, Copy, Paste, Rename, Share, Delete, Sort ▾, View ▾, Filter ▾, ⋯, Details
- [ ] Item check boxes, the header's select-all box; selected rows lit, the current one outlined
- [ ] Preview pane (Alt+P): the picture, name, type, size, dimensions, taken, modified, tags with + add, rating; Open, Open with ▾, Share
- [ ] Status bar: N items, N selected · size, the view buttons
## 2 · View
- [ ] Extra large, large, medium and small icons, list, details, tiles, content; the current one marked
- [ ] Compact view; Ctrl + mouse wheel changes the size; each folder remembers its view
- [ ] Show ›: navigation pane, details pane (Alt+Shift+P), preview pane (Alt+P), item check boxes, file name extensions, hidden items (Ctrl+H)
- [ ] Group by ›, Choose columns…, Folder options…
- [ ] Picture thumbnails in the icon views
## 3 · Right-click
- [ ] Opens at the pointer (below and right; above or left when there's no room)
- [ ] Windows 11's icon row: Cut, Copy, Rename, Share, Delete
- [ ] Open (Enter), Open with ›, Open in new tab, Open in new window
- [ ] Add to Favourites, Pin to Quick access, Compress to ›, Copy as path (Ctrl+Shift+C)
- [ ] Properties (Alt+Enter), Show more options (Shift+F10)
- [ ] Open with's list (the default marked), Choose another app…, Always use
## 4 · Copying
- [ ] The copy window: what, from where to where, % done and amounts, the speed graph with its average line, speed, name, time left, items left
- [ ] Pause, Cancel, Fewer / More details
- [ ] A second copy shown at the same time with its bar, pause and cancel
- [ ] Name clashes: Replace (to the Trash), Skip, Keep both (numbered), Let me decide for each file (compare dates, sizes, pictures); Do this for all
## 5 · Properties
- [ ] General: icon, name, type, opens with + Change…, location, size, size on disk, created / modified / accessed, read-only and hidden; Cancel, Apply, OK
- [ ] Security in plain words: owner, what you, others on this PC and everyone else can do; Who can open it…, Advanced…
- [ ] Details tab
- [ ] Previous versions from Hearth's snapshots: Open, Restore a copy beside it, Restore
- [ ] Several items at once
## 6 · Search
- [ ] Search results in <folder> in the address bar; the search box with ✕; Ctrl+E / Ctrl+F
- [ ] Filter chips: Kind, Date modified, Size, Search in, Look inside files; Close search
- [ ] Results as you type, the match highlighted, a Where column with open folder
- [ ] "N found as you typed · still looking in …"
- [ ] Recent searches and Clear search history
