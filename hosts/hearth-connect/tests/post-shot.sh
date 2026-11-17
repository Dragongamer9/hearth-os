#!/bin/sh
# Posts a screenshot as SHOT notices (title "SHOT <name> <n>", a JPEG in base64 cut in pieces), the way the other
# workflows do: taken at 1920x1080, made small enough for ten notices (the lead puts them back together).
#   sh post-shot.sh <name> <png>
name="$1"; f="$2"
[ -f "$f" ] || { echo "::warning title=SHOT $name::no picture ($f)"; exit 0; }
tmp=$(mktemp --suffix .jpg)
for w in 1280 1120 960 800 640; do
  for q in 48 38 30; do
    convert "$f" -resize "${w}x" -quality "$q" "jpg:$tmp" 2>/dev/null || magick "$f" -resize "${w}x" -quality "$q" "jpg:$tmp"
    [ "$(base64 -w0 "$tmp" | wc -c)" -le 37900 ] && break 2
  done
done
echo "SHOT $name: $(identify -format '%wx%h' "$f" 2>/dev/null) taken, posted at ${w}px q$q"
base64 -w0 "$tmp" | fold -w 3790 | nl -ba -w2 -s' ' | head -10 | while read -r i part; do echo "::notice title=SHOT $name $i::$part"; done
rm -f "$tmp"
