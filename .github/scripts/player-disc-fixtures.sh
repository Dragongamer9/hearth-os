#!/bin/bash
# Fixture discs for Hearth Player's disc tests (player.yml; docs/PLAYER-DISCS.md), made with ffmpeg, dvdauthor,
# genisoimage and vcdimager:
#   hearth-test.iso   a small DVD-Video: its own menu (a still picture with two buttons: Play the film, Title 2), title 1
#                     (24 s, 4 chapters, English and French sound, English subtitles) and title 2 (10 s, 2 chapters,
#                     interlaced); dvd/ is the same disc as a VIDEO_TS folder
#   hearth-locked.iso the same disc with its film sectors marked scrambled, the way a CSS-locked DVD reads without
#                     libdvdcss (the player must say it's locked and offer DVD decoding)
#   vcd.cue/vcd.bin   a Video CD (one 20 s part)
#   cd.cue/cd.bin     a music CD stand-in: three songs with CD-Text names (a CUE sheet with TITLE and PERFORMER)
#   bd/               a Blu-ray disc's folder (BDMV): the player must say it doesn't play those
#   files/            a data disc's folder with a video file in it
# Usage: player-disc-fixtures.sh OUTDIR. Each part is made on its own; one that can't be made is said and skipped.
set -u
out="$1"; mkdir -p "$out"; cd "$out" || exit 1
export VIDEO_FORMAT=NTSC
ff() { timeout 300 ffmpeg -hide_banner -loglevel error -y "$@"; }
made=()

# ---- DVD-Video
if command -v dvdauthor >/dev/null && command -v genisoimage >/dev/null; then
  ff -f lavfi -i "testsrc2=size=720x480:rate=30000/1001:duration=24" -f lavfi -i "sine=f=440:duration=24" -f lavfi -i "sine=f=660:duration=24" \
    -map 0:v -map 1:a -map 2:a -target ntsc-dvd -aspect 16:9 -c:a ac3 -b:a 192k -ac 2 t1.mpg
  ff -f lavfi -i "testsrc=size=720x480:rate=30000/1001:duration=10" -f lavfi -i "sine=f=330:duration=10" -target ntsc-dvd -aspect 16:9 -flags +ilme+ildct -top 1 -c:a ac3 -ac 2 t2.mpg
  # The disc's own menu: a still picture (it waits for a press), its two buttons as a subpicture (spumux).
  menu=""
  if command -v spumux >/dev/null && command -v convert >/dev/null; then
    convert -size 720x480 xc:'#1b2440' +antialias -fill white -pointsize 46 -annotate +170+200 'Hearth Test' -pointsize 32 \
      -annotate +200+336 'Play the film' -annotate +200+406 'Title 2' bg.png 2>/dev/null
    convert -size 720x480 xc:transparent +antialias -fill none -stroke '#ffd166' -strokewidth 4 -draw 'rectangle 160,300 560,350' -draw 'rectangle 160,370 560,420' -colors 3 PNG8:mhl.png 2>/dev/null
    convert -size 720x480 xc:transparent +antialias -fill none -stroke '#ff5d5d' -strokewidth 4 -draw 'rectangle 160,300 560,350' -draw 'rectangle 160,370 560,420' -colors 3 PNG8:msel.png 2>/dev/null
    ff -loop 1 -i bg.png -f lavfi -i "anullsrc=r=48000:cl=stereo" -t 2 -target ntsc-dvd -aspect 16:9 -c:a ac3 -ac 2 menu0.mpg
    cat > menu.xml <<'EOF'
<subpictures format="NTSC"><stream><spu start="00:00:00.00" end="00:00:00.00" highlight="mhl.png" select="msel.png" force="yes">
<button name="b1" x0="160" y0="300" x1="560" y1="350" down="b2" />
<button name="b2" x0="160" y0="370" x1="560" y1="420" up="b1" />
</spu></stream></subpictures>
EOF
    if spumux -m dvd -s0 menu.xml < menu0.mpg > menu.mpg 2>spumux-menu.log && [ -s menu.mpg ]; then menu=yes; echo "menu: made"; else echo "menu: spumux couldn't ($(tail -1 spumux-menu.log))"; fi
  fi
  vmgm="<vmgm />"
  [ -n "$menu" ] && vmgm='<vmgm><fpc>jump vmgm menu 1;</fpc><menus><video format="ntsc" aspect="16:9" widescreen="nopanscan" /><subpicture lang="en" /><pgc entry="title"><vob file="menu.mpg" pause="inf" /><button name="b1">jump title 1;</button><button name="b2">jump title 2;</button></pgc></menus></vmgm>'
  # Subtitles the way DVDs carry them: a picture (spumux), white text with a black edge, no smoothing (4 colours at most).
  if command -v spumux >/dev/null && command -v convert >/dev/null; then
    convert -size 720x480 xc:transparent +antialias -fill white -stroke black -strokewidth 2 -pointsize 40 -gravity south \
      -annotate +0+50 'Hearth Player subtitles' -colors 4 PNG8:sub.png 2>/dev/null
    printf '<subpictures format="NTSC"><stream><spu start="00:00:00.50" end="00:00:23.00" image="sub.png" force="no"/></stream></subpictures>\n' > subs.xml
    if spumux -m dvd -s0 subs.xml < t1.mpg > t1s.mpg 2>spumux.log && [ -s t1s.mpg ]; then mv t1s.mpg t1.mpg; touch subs.made; echo "subtitles: made"; else echo "subtitles: spumux couldn't ($(tail -1 spumux.log))"; rm -f t1s.mpg; fi
  fi
  cat > dvd.xml <<EOF
<dvdauthor dest="dvd" jumppad="0">
  $vmgm
  <titleset>
    <titles>
      <video format="ntsc" aspect="16:9" widescreen="nopanscan" />
      <audio lang="en" />
      <audio lang="fr" />
      <subpicture lang="en" />
      <pgc><vob file="t1.mpg" chapters="0,0:06,0:12,0:18" /></pgc>
      <pgc><vob file="t2.mpg" chapters="0,0:05" /></pgc>
    </titles>
  </titleset>
</dvdauthor>
EOF
  rm -rf dvd
  if [ -n "$menu" ] && ! dvdauthor -x dvd.xml > dvdauthor.log 2>&1; then
    # The menu didn't go in: the disc without it, so the rest is still tested.
    echo "menu: dvdauthor refused it ($(grep -i err dvdauthor.log | tail -1))"; rm -rf dvd; sed -i 's|<vmgm>.*</vmgm>|<vmgm />|' dvd.xml; menu=""
  fi
  [ -n "$menu" ] && touch menu.made
  if { [ -d dvd/VIDEO_TS ] || dvdauthor -x dvd.xml > dvdauthor.log 2>&1; } && genisoimage -quiet -dvd-video -V HEARTH_TEST -o hearth-test.iso dvd; then
    made+=(hearth-test.iso)
    # Locked: every video sector's PES scrambling bits set (libdvdcss would clear them after unscrambling).
    python3 - hearth-test.iso hearth-locked.iso <<'PY'
import sys
src, dst = sys.argv[1], sys.argv[2]
b = bytearray(open(src, 'rb').read())
n = 0
for off in range(0, len(b) - 2048 + 1, 2048):
    s = b[off:off + 14]
    if s[:4] != b'\x00\x00\x01\xba' or s[4] >> 6 != 1:
        continue
    p = off + 14 + (b[off + 13] & 7)
    if b[p:p + 4] == b'\x00\x00\x01\xe0' and b[p + 6] >> 6 == 2:
        b[p + 6] |= 0x10
        n += 1
open(dst, 'wb').write(b)
print('locked sectors:', n)
PY
    made+=(hearth-locked.iso)
  else
    echo "DVD: couldn't be made: $(tail -3 dvdauthor.log | tr '\n' ' ')"
  fi
  rm -f t1.mpg t2.mpg menu0.mpg menu.mpg
else
  echo "DVD: dvdauthor or genisoimage isn't installed"
fi

# ---- Video CD
if command -v vcdimager >/dev/null; then
  ff -f lavfi -i "testsrc=size=352x240:rate=30000/1001:duration=20" -f lavfi -i "sine=f=520:duration=20" -target ntsc-vcd vcd.mpg
  if timeout 300 vcdimager -t vcd2 -l HEARTH_VCD -c vcd.cue -b vcd.bin vcd.mpg > vcdimager.log 2>&1; then made+=(vcd.cue); else echo "VCD: $(tail -2 vcdimager.log | tr '\n' ' ')"; fi
  rm -f vcd.mpg
else
  echo "VCD: vcdimager isn't installed"
fi

# ---- Music CD (raw CD audio in a .bin, CD-Text in its .cue)
for i in 1 2 3; do ff -f lavfi -i "sine=f=$((300 + i * 120)):duration=$((8 + i * 2)):sample_rate=44100" -ac 2 -f s16le -c:a pcm_s16le "t$i.raw"; done
python3 - <<'PY'
import os
names = ['Ember song 1', 'Kindling', 'The last log']
bin_, cue, lba = open('cd.bin', 'wb'), ['PERFORMER "Hearth Band"', 'TITLE "Kiln"', 'FILE "cd.bin" BINARY'], 0
for i, n in enumerate(names, 1):
    d = open(f't{i}.raw', 'rb').read()
    d += b'\0' * (-len(d) % 2352)
    m, s, f = lba // 4500, lba // 75 % 60, lba % 75
    cue += [f'  TRACK {i:02d} AUDIO', f'    TITLE "{n}"', '    PERFORMER "Hearth Band"' if i != 3 else '    PERFORMER "Guest Player"', f'    INDEX 01 {m:02d}:{s:02d}:{f:02d}']
    bin_.write(d)
    lba += len(d) // 2352
    os.remove(f't{i}.raw')
open('cd.cue', 'w').write('\n'.join(cue) + '\n')
PY
[ -s cd.bin ] && made+=(cd.cue)

# ---- Blu-ray folder, data disc folder
mkdir -p bd/BDMV/STREAM && printf 'INDX0200' > bd/BDMV/index.bdmv && made+=(bd)
mkdir -p "files/Holiday 2019" && ff -f lavfi -i "testsrc2=size=640x360:rate=25:duration=6" -c:v libx264 -pix_fmt yuv420p "files/Holiday 2019/Beach.mp4" && made+=(files)

echo "made: ${made[*]}"
ls -la
