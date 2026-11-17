# The new Hearth Photos: a photo library, not just a viewer (approved Nov 15, 2026)

Mockups: next-update/photos-1-library.png, next-update/photos-2-photo.png, next-update/photos-3-albums-people-places.png, next-update/photos-4-controller.png, next-update/photos-5-edit.png
Release: 1.14

The owner's choice: the library look (1, 2, 3, 5) while you use a keyboard and mouse, the controller look (4) by itself
when a controller is used (the same idea as the new Settings). Everything (search, faces, what's in a photo) is worked
out on this PC; nothing leaves it. Opening a single picture from Files still opens it straight away.

## Library (photos-1)
- [x] left list: Photos with its icon; All photos, Memories, Favorites, Videos, Game captures, People, Places, From your phone, Folders, Trash, each with its count; the picked one marked — hosts/hearth-photos/ui/library.js: "['all', 'All photos'], ['memories', 'Memories'], ['favorites', 'Favorites']"
- [x] Your albums: each with its small picture — hosts/hearth-photos/ui/library.js: "class=\"li al"
- [x] the phone box: Chris's iPhone with its dot, "New photos come in by themselves · last 4 min ago", and photos · size · "all on this PC, nothing leaves it" — hosts/hearth-photos/ui/library.js: "New photos come in by themselves · last"
- [x] search: "Search: sunset at the lake …" by what's in a photo, "Searched on this PC" — hosts/hearth-photos/ui/library.js: "Searched on this PC"
- [x] Import, Select, Slideshow and the Size slider — hosts/hearth-photos/ui/library.js: "data-lact=\"slideshow\">Slideshow"
- [x] Memories: four cards (a year ago today, a trip, game captures, a season), "made on this PC from your own photos" — hosts/hearth-photos/ui/library.js: "made on this PC from your own photos"
- [x] the photos by day (Today, Yesterday, Earlier this week …) with the day, count and where they came from; rows of pictures at their own shapes — hosts/hearth-photos/ui/library.js: "Earlier this week"
- [x] on the pictures: ▶ and the length for videos, ♥ for favorites, a label for screenshots and game captures (which game, Steam) — hosts/hearth-photos/ui/library.js: "'<span class=\"lab\">Screenshot</span>'"
- [x] the controller's / keyboard's ember ring on the picked picture — hosts/hearth-photos/ui/library.js: "function firstTile()"
- [x] the date strip on the right: months, then years, to jump to — hosts/hearth-photos/ui/library.js: "The date strip: this year"

## A photo (photos-2)
- [x] back, its name, when and "4 of 9" — hosts/hearth-photos/ui/app.js: "bits.push(`${S.i + 1} of ${S.pics.length}`)"
- [x] Favorite, Edit, Share, Add to album, Set as wallpaper, Info, Trash — hosts/hearth-photos/ui/index.html: "data-act=\"wallpaper\" title=\"Set as wallpaper\">Set as wallpaper"
- [x] the picture large, ‹ › to the others, Fit / 100 % / Turn / Full screen — hosts/hearth-photos/ui/index.html: "data-act=\"zoomFit\" id=\"zoomFit\">Fit"
- [x] the strip of the day's other pictures, the shown one marked — hosts/hearth-photos/ui/app.js: "function renderStrip("
- [x] Details: date and time, camera and its settings, size, file kind — hosts/hearth-photos/ui/app.js: "<h3>Details</h3>"
- [x] Place: a small map (OpenStreetMap) with the pin and the place's name — hosts/hearth-photos/ui/place.js: "map from OpenStreetMap"
- [x] People: the faces in it, "+ Name a face" — hosts/hearth-photos/ui/library.js: "+ Name a face"
- [x] Found in it: what's in the picture as tags, "worked out on this PC (your graphics card)" — hosts/hearth-photos/ui/library.js: "Worked out on this PC${a.by === 'gpu' ? ' (your graphics card)' : ''}"
- [x] In albums, "+ Add to an album" — hosts/hearth-photos/ui/app.js: "+ Add to an album"
- [x] where the file is, Show in Files — hosts/hearth-photos/ui/app.js: "Show in Files</button>"

## Albums, people and places (photos-3)
- [x] Collections with Albums / People / Places / Folders and + New album — hosts/hearth-photos/ui/library.js: "['collections', 'Albums'], ['people', 'People'], ['places', 'Places'], ['folders', 'Folders']"
- [x] albums kept up to date by themselves: Game captures (Steam, Game Mode and Desktop), From your phone, Screenshots (this PC and your Ally), Videos, Favorites, Made by your AI (ComfyUI pictures), each with its count — hosts/hearth-photos/ui/library.js: "'Made by your AI'"
- [x] Your albums with their counts, and + New album; Wallpapers used by Hearth's wallpaper picker — hosts/hearth-photos/ui/library.js: "used by Hearth's wallpaper picker"
- [x] People: faces grouped on this PC, names only you see, Add a name, See all; turning faces off forgets every group — hosts/hearth-photos/ui/library.js: "Turn faces off in Settings › Photos and every group is forgotten."
- [x] Places: a map with groups of photos and their counts, Open the map — hosts/hearth-photos/ui/library.js: "Open the map</button>"

## Controller look (photos-4)
- [x] LB / RB tabs: For you, All photos, Albums, People, Places, Videos; the photo count — hosts/hearth-photos/ui/library.js: "['foryou', 'For you'], ['all', 'All photos'], ['albums', 'Albums'], ['people', 'People'], ['places', 'Places'], ['videos', 'Videos']"
- [x] a big Memory with A Play as a slideshow and X Browse them — hosts/hearth-photos/ui/library.js: "Play as a slideshow"
- [x] rows of big tiles (Today and yesterday, Albums) with names — hosts/hearth-photos/ui/library.js: "<h2>Today and yesterday</h2>"
- [x] button hints: A Open, X Slideshow, Y Search, ☰ Options, B Back; "shows like this when a controller is in use" — hosts/hearth-photos/ui/library.js: "Shows like this when a controller is in use · keyboard and mouse get the library"

## Edit (photos-5)
- [x] Cancel, the photo's name, Undo, Redo, Hold to see the original, Replace the original… (asks), Save as a copy (the main button) — hosts/hearth-photos/ui/edit.js: "Hold to see the original</button>"
- [x] crop with the frame, its corners and thirds; Straighten with its angle — hosts/hearth-photos/ui/edit.js: "<label for=\"es-angle\">Straighten</label>"
- [x] Light / Color / Looks / Fix up tabs, Auto fix, Crop shape — hosts/hearth-photos/ui/edit.js: "['fix', 'Fix up']"
- [x] Brightness, Contrast, Highlights, Shadows, Warmth, Saturation, Vignette sliders with their values — hosts/hearth-photos/ui/edit.js: "['vg', 'Vignette']"
- [x] "The original stays as it is …": a copy next to it; replacing asks first and keeps the old one in Trash for 30 days — hosts/hearth-photos/ui/edit.js: "keeps the old one in Trash for 30 days"
