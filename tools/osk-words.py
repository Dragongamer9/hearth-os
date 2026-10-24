#!/usr/bin/env python3
"""osk-words: makes the on-screen keyboard's word lists (ui/osk/words/<lang>.js): the most used words of each
language, from FrequencyWords by Hermit Dave (OpenSubtitles 2018, CC BY-SA 4.0), without single letters that
aren't words, words with digits or other scripts, and swear words.
  python3 tools/osk-words.py DIR ui/osk/words/en.js   DIR holds <lang>.txt from
  https://github.com/hermitdave/FrequencyWords/tree/master/content/2018/<lang>/<lang>_50k.txt (Norwegian: no.txt)
then python3 tools/osk-sync.py."""
import re, sys, os
src = sys.argv[1]; out = sys.argv[2]
N = 6000
LET = {
 'en': "a-z'", 'fr': "a-zàâæçéèêëîïôœùûüÿ'", 'de': "a-zäöüß", 'es': "a-záéíñóúü", 'sv': "a-zåäöé", 'fi': "a-zåäö",
 'da': "a-zæøåé", 'nb': "a-zæøåéèêóòô", 'pt': "a-záàâãçéêíóôõú", 'it': "a-zàèéìíòóùú'",
}
ONE = {'en': 'ai', 'fr': 'aày', 'de': '', 'es': 'aeoyu', 'sv': 'iå', 'fi': '', 'da': 'iå', 'nb': 'iå', 'pt': 'aeoà', 'it': 'aeèioà'}
BLOCK = set('''fuck fucking fucked fucker fuckin motherfucker motherfucking shit shitty bullshit bitch bitches asshole bastard dick dicks pussy cunt whore slut nigga nigger niggas fag faggot cock cocksucker dickhead goddamn
merde putain connard connasse salope enculé enculer bordel pute couilles nique niquer
scheiße scheisse arschloch ficken fick fickt hure fotze wichser verdammt
mierda puta puto joder coño cabrón cabron pendejo gilipollas jodido polla
cazzo merda stronzo puttana vaffanculo fottuto cazzi coglione
porra caralho foda foder cacete merda puta
jävla jävlar helvete fitta kuk hora skit fan
lort kraftedeme fanden luder kusse pik
faen jævla dritt hore fitte pikk
vittu perkele saatana paska helvetti huora kusipää mulkku'''.split())
lang = os.path.basename(out)[:-3]
srcl = {'nb': 'no'}.get(lang, lang)
allowed = re.compile('^[' + LET[lang] + ']+$')
words = []
seen = set()
for line in open(os.path.join(src, srcl + '.txt'), encoding='utf-8'):
    w = line.split(' ')[0].strip().lower()
    if not w or w in seen or w in BLOCK: continue
    if not allowed.match(w): continue
    if w.startswith("'") or w.endswith("'") or "''" in w: continue
    if len(w) == 1 and w not in ONE[lang]: continue
    seen.add(w)
    if lang == 'en' and (w == 'i' or w.startswith("i'")): w = 'I' + w[1:]
    words.append(w)
    if len(words) >= N: break
with open(out, 'w', encoding='utf-8') as f:
    f.write('// Hearth on-screen keyboard: the %d most used %s words, most used first (word suggestions).\n' % (len(words), lang))
    f.write('// From FrequencyWords by Hermit Dave (OpenSubtitles 2018), CC BY-SA 4.0: https://github.com/hermitdave/FrequencyWords\n')
    f.write('// Made by tools/osk-words.py; loaded by ui/osk/hearth-osk.js (a script tag on web pages, read as text in Mantel).\n')
    f.write('(typeof HearthOSK !== "undefined") && HearthOSK.words("%s", "%s");\n' % (lang, ' '.join(words)))
print(lang, len(words))
