#!/bin/sh
# Installs Hearth Office (Write, Sheets, Slides) into a Hearth OS image. Run by os-bases/*/build.sh from the
# folder CI packs into hearth-app.tar (base-anatase.yml, base-peppermint.yml):
#   office/hearth-office        the program (one for the three apps)
#   office/install.sh           this script
# The icons (hearth-write, hearth-sheets, hearth-slides) come with Hearth's icon set (branding/icons).
# Puts the program in /usr/bin with a name per app, their menu entries (with the file types they open), the
# .hslides file type. build.sh then makes them the default apps for office files (hearth-system defaults-system).
# DESTDIR installs somewhere else (for checking this script).
set -eu
SRC=$(cd "$(dirname "$0")" && pwd)
D=${DESTDIR:-}
[ -x "$SRC/hearth-office" ] || { echo "hearth-office isn't in $SRC; Hearth Office left out"; exit 0; }
install -Dm755 "$SRC/hearth-office" "$D/usr/bin/hearth-office"
for a in write sheets slides; do ln -sf hearth-office "$D/usr/bin/hearth-$a"; done
if [ -z "$D" ] && command -v ldd >/dev/null && ldd /usr/bin/hearth-office | grep 'not found'; then
  echo "warning: Hearth Office needs the libraries above, which this image doesn't have"
fi

W='application/vnd.openxmlformats-officedocument.wordprocessingml.document;application/vnd.oasis.opendocument.text;application/rtf;text/rtf;text/markdown;text/x-markdown;'
S='application/vnd.openxmlformats-officedocument.spreadsheetml.sheet;application/vnd.ms-excel.sheet.macroEnabled.12;application/vnd.ms-excel;application/vnd.oasis.opendocument.spreadsheet;text/csv;text/tab-separated-values;'
P='application/vnd.openxmlformats-officedocument.presentationml.presentation;application/x-hearth-slides;'
install -d "$D/usr/share/applications" "$D/usr/share/mime/packages"
cat > "$D/usr/share/applications/hearth-write.desktop" <<DESKTOP
[Desktop Entry]
Name=Hearth Write
GenericName=Word Processor
Comment=Write letters, reports and documents; opens and saves Word (.docx), OpenDocument (.odt), RTF, Markdown and text
Exec=/usr/bin/hearth-write %f
TryExec=/usr/bin/hearth-office
Icon=hearth-write
Type=Application
Categories=Office;WordProcessor;
MimeType=${W}text/plain;
Keywords=word;document;docx;odt;rtf;letter;report;writer;text;office;
StartupNotify=true
Actions=new-window;

[Desktop Action new-window]
Name=New document
Exec=/usr/bin/hearth-write
DESKTOP
cat > "$D/usr/share/applications/hearth-sheets.desktop" <<DESKTOP
[Desktop Entry]
Name=Hearth Sheets
GenericName=Spreadsheet
Comment=Budgets, lists and numbers with formulas; opens and saves Excel (.xlsx), OpenDocument (.ods) and CSV
Exec=/usr/bin/hearth-sheets %f
TryExec=/usr/bin/hearth-office
Icon=hearth-sheets
Type=Application
Categories=Office;Spreadsheet;
MimeType=${S}
Keywords=spreadsheet;excel;xlsx;ods;csv;calc;table;budget;formula;office;
StartupNotify=true
Actions=new-window;

[Desktop Action new-window]
Name=New spreadsheet
Exec=/usr/bin/hearth-sheets
DESKTOP
cat > "$D/usr/share/applications/hearth-slides.desktop" <<DESKTOP
[Desktop Entry]
Name=Hearth Slides
GenericName=Presentations
Comment=Make and present slides on a screen or TV, with a keyboard, clicker or controller; opens and saves PowerPoint (.pptx)
Exec=/usr/bin/hearth-slides %f
TryExec=/usr/bin/hearth-office
Icon=hearth-slides
Type=Application
Categories=Office;Presentation;
MimeType=${P}
Keywords=presentation;slides;powerpoint;pptx;impress;deck;present;office;
StartupNotify=true
Actions=new-window;

[Desktop Action new-window]
Name=New presentation
Exec=/usr/bin/hearth-slides
DESKTOP
# Hearth Slides' own files (.hslides).
cat > "$D/usr/share/mime/packages/hearth-office.xml" <<MIME
<?xml version="1.0" encoding="UTF-8"?>
<mime-info xmlns="http://www.freedesktop.org/standards/shared-mime-info">
  <mime-type type="application/x-hearth-slides">
    <comment>Hearth Slides presentation</comment>
    <sub-class-of type="application/json"/>
    <glob pattern="*.hslides"/>
    <icon name="hearth-slides"/>
  </mime-type>
</mime-info>
MIME

# Office files open in Hearth Office in Hearth's desktop (XDG_CURRENT_DESKTOP=Nimbus), unless you pick other apps:
# build.sh makes every Hearth app the default for the types in its MimeType once they're all in (hearth-system
# defaults-system).

if [ -z "$D" ]; then
  command -v update-mime-database >/dev/null && update-mime-database /usr/share/mime || true
  command -v update-desktop-database >/dev/null && update-desktop-database -q /usr/share/applications || true
  command -v gtk-update-icon-cache >/dev/null && gtk-update-icon-cache -q -t -f /usr/share/icons/hicolor || true
fi
echo "Hearth Office installed: /usr/bin/hearth-office (hearth-write, hearth-sheets, hearth-slides)"
