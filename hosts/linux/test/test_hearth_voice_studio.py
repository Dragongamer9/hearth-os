"""Tests for Voice Studio (hosts/linux/desktop/shell/hearth-voice-studio, docs/HOME-SERVER.md): Hearth's own voice
studio on the home server. Its settings go through hearth_settings_io (only what you change, never defaults over a
broken file); the page is only for this PC and Tailscale, behind a PIN (its own, or Server mode's) with SameSite=Strict
sign-ins and same-origin writes; apps use an API key; the OpenAI-compatible API speaks and transcribes; cloning needs
consent; the queue (audiobooks, dubbing, transcripts) lives on disk and carries on after a restart or a game; the home
server, its root helper, the user unit and the image treat it like AI pictures (off, installed only when asked, never
part of starting). The engines are stand-ins (MockEngines): no models, no graphics card, no network."""
import sys
sys.dont_write_bytecode = True
import importlib.machinery
import importlib.util
import io
import json
import os
import pathlib
import shutil
import stat
import tempfile
import threading
import time
import unittest
import urllib.error
import urllib.request
import wave
import zipfile

HERE = pathlib.Path(__file__).resolve().parent
ROOT = HERE.parent.parent.parent
SHELL = ROOT / 'hosts' / 'linux' / 'desktop' / 'shell'
UNITS = ROOT / 'os' / 'system_files' / 'usr' / 'lib' / 'systemd' / 'user'


def load(name, path):
    loader = importlib.machinery.SourceFileLoader(name, str(path))
    spec = importlib.util.spec_from_loader(name, loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


vs = load('hearth_voice_studio_t', SHELL / 'hearth-voice-studio')
HAVE_FFMPEG = shutil.which('ffmpeg') is not None


def tone_wav(path, secs=6.0, sr=24000):
    import math
    import array
    a = array.array('h', (int(8000 * math.sin(2 * math.pi * 220 * i / sr)) for i in range(int(secs * sr))))
    with wave.open(str(path), 'wb') as w:
        w.setnchannels(1)
        w.setsampwidth(2)
        w.setframerate(sr)
        w.writeframes(a.tobytes())
    return str(path)


class Base(unittest.TestCase):
    """Every path of the module points into a fresh folder; Kokoro and Ollama answer nowhere."""
    def setUp(self):
        self.d = tempfile.mkdtemp(prefix='vs-test-')
        self.saved = {k: getattr(vs, k) for k in ('SETTINGS', 'WEB_STATE', 'SERVER_MODE_WEB', 'DATA', 'VENV', 'MODELS', 'INSTALLED', 'KOKORO', 'OLLAMA', 'translate')}
        vs.SETTINGS = os.path.join(self.d, 'config', 'nimbus', 'voice-studio.json')
        vs.WEB_STATE = os.path.join(self.d, 'state', 'nimbus', 'voice-studio-web.json')
        vs.SERVER_MODE_WEB = os.path.join(self.d, 'state', 'nimbus', 'server-mode-web.json')
        vs.DATA = os.path.join(self.d, 'data', 'nimbus', 'voice-studio')
        vs.VENV = os.path.join(vs.DATA, 'venv')
        vs.MODELS = os.path.join(vs.DATA, 'models')
        vs.INSTALLED = os.path.join(vs.DATA, 'installed.json')
        vs.KOKORO = 'http://127.0.0.1:9'
        vs.OLLAMA = 'http://127.0.0.1:9'
        vs._KOKORO.update(at=time.time(), voices=[])
        vs.FAILS.clear()
        self.eng = vs.MockEngines(vs.DATA)
        self.studio = vs.Studio(vs.DATA, self.eng)

    def tearDown(self):
        for k, v in self.saved.items():
            setattr(vs, k, v)
        shutil.rmtree(self.d, ignore_errors=True)


# ---------------------------------------------------------------------------------------------------- settings

class Settings(Base):
    def test_only_what_you_change_is_saved_and_defaults_are_never_written(self):
        self.assertEqual(vs.load(), vs.DEFAULTS)
        self.assertFalse(os.path.exists(vs.SETTINGS), 'reading never writes')
        self.assertFalse(vs.set_key('language', 'auto'), 'the default counts as what you have: nothing written')
        self.assertFalse(os.path.exists(vs.SETTINGS))
        self.assertTrue(vs.set_key('language', 'de'))
        self.assertEqual(json.load(open(vs.SETTINGS)), {'language': 'de'})
        self.assertTrue(vs.set_key('speakerLabels', 'off'))
        self.assertEqual(json.load(open(vs.SETTINGS)), {'language': 'de', 'speakerLabels': False}, 'merged, the rest kept')
        self.assertTrue(os.path.exists(vs.SETTINGS + '.bak'), 'the last good version is kept')
        for k, bad in (('language', 'klingon'), ('voice', '../x'), ('translateModel', '-rf'), ('speakerLabels', 'yes'), ('nope', 'on')):
            with self.assertRaises(vs.Fail):
                vs.set_key(k, bad)

    def test_a_broken_file_is_restored_and_defaults_never_go_over_it(self):
        vs.set_key('language', 'fr')
        vs.set_key('voice', 'narrator')
        with open(vs.SETTINGS, 'w') as f:
            f.write('{"language": "fr", "voi')
        self.assertEqual(vs.load()['language'], 'fr', 'restored from .bak (the first change)')
        self.assertNotEqual(json.load(open(vs.SETTINGS)), {}, 'never replaced with defaults')

    def test_deleting_your_default_voice_never_changes_your_settings(self):
        v = self.studio.design_save('Captain', 'an old man with a deep voice')
        vs.set_key('voice', v['id'])
        before = open(vs.SETTINGS).read()
        r = self.studio.delete_voice(v['id'])
        self.assertTrue(r['wasDefault'])
        self.assertEqual(open(vs.SETTINGS).read(), before, 'settings never change by themselves')
        self.assertEqual(self.studio.resolve('', allow_auto=False)['id'], 'narrator', 'speech falls back to the ready-made voice')


# ---------------------------------------------------------------------------------------------------- PIN, sign-ins, key

class Secrets(Base):
    def test_only_the_pins_hash_is_kept_privately_and_a_new_pin_signs_everyone_out(self):
        vs.pin_set('２４６８')                      # a phone's full-width digits count
        raw = open(vs.WEB_STATE).read()
        self.assertNotIn('2468', raw)
        self.assertEqual(stat.S_IMODE(os.stat(vs.WEB_STATE).st_mode), 0o600)
        self.assertTrue(vs.pin_check('2468', vs.secret_read()['pin']))
        tok = vs.session_new('phone')
        self.assertTrue(vs.session_ok(tok))
        vs.pin_set('13579')
        self.assertFalse(vs.session_ok(tok), 'a new PIN signs every device out')
        for bad in ('12', 'abcd', '1234567890123', ''):
            with self.assertRaises(vs.Fail):
                vs.pin_set(bad)

    def test_server_modes_pin_opens_it_until_it_has_its_own(self):
        self.assertEqual(vs.pin_hashes(), [])
        os.makedirs(os.path.dirname(vs.SERVER_MODE_WEB), exist_ok=True)
        with open(vs.SERVER_MODE_WEB, 'w') as f:
            json.dump({'pin': vs.pin_hash('1111'), 'sessions': {}}, f)
        self.assertEqual(vs.pin_hashes()[0][1], 'server')
        tok = vs.session_new('laptop')
        self.assertTrue(vs.session_ok(tok))
        vs.pin_set('2222')
        self.assertEqual(vs.pin_hashes()[0][1], 'own')
        self.assertFalse(vs.session_ok(tok))
        self.assertTrue(vs.pin_clear()['serverPin'])

    def test_the_api_key_is_made_once_compared_whole_and_can_be_replaced(self):
        k = vs.api_key()
        self.assertTrue(k.startswith('hvs-') and len(k) > 30)
        self.assertEqual(vs.api_key(), k)
        self.assertTrue(vs.key_ok('Bearer ' + k))
        for bad in ('Bearer ' + k[:-1], k, 'Basic ' + k, '', 'Bearer ' + k + 'x'):
            self.assertFalse(vs.key_ok(bad), bad)
        k2 = vs.api_key_new()
        self.assertNotEqual(k, k2)
        self.assertFalse(vs.key_ok('Bearer ' + k))

    def test_only_this_pc_and_tailscale(self):
        for ok in ('127.0.0.1', '::1', '100.101.102.103', 'fd7a:115c:a1e0::5', '::ffff:100.64.0.9'):
            self.assertTrue(vs.peer_allowed(ok), ok)
        for bad in ('192.168.1.20', '10.0.0.4', '8.8.8.8', '100.128.0.1', 'nonsense'):
            self.assertFalse(vs.peer_allowed(bad), bad)
        self.assertEqual(vs.listen_addresses()[0], '127.0.0.1')

    def test_writes_must_come_from_the_page_itself(self):
        self.assertFalse(vs.write_allowed({}))
        self.assertTrue(vs.write_allowed({'X-Hearth': '1'}))
        self.assertTrue(vs.write_allowed({'X-Hearth': '1', 'Sec-Fetch-Site': 'same-origin'}))
        self.assertFalse(vs.write_allowed({'X-Hearth': '1', 'Sec-Fetch-Site': 'cross-site'}))
        self.assertTrue(vs.write_allowed({'X-Hearth': '1', 'Origin': 'null'}), 'Origin null is the page under some policies')
        self.assertFalse(vs.write_allowed({'X-Hearth': '1', 'Origin': 'http://evil.example', 'Host': 'pc:8890'}))


# ---------------------------------------------------------------------------------------------------- the page and the API over HTTP

class Http(Base):
    def setUp(self):
        super().setUp()
        self.srv = vs.make_servers(self.studio, ['127.0.0.1'], 0, mock=True)[0]
        self.base = 'http://127.0.0.1:%d' % self.srv.server_address[1]
        vs.pin_set('2468')
        self.key = vs.api_key()

    def tearDown(self):
        self.srv.shutdown()
        self.srv.server_close()
        super().tearDown()

    def req(self, path, body=None, headers=None, method=None, raw=False):
        h = dict(headers or {})
        data = None
        if isinstance(body, (dict, list)):
            data = json.dumps(body).encode()
            h.setdefault('Content-Type', 'application/json')
        elif body is not None:
            data = body
        r = urllib.request.Request(self.base + path, data=data, headers=h, method=method or ('POST' if data is not None else 'GET'))
        try:
            with urllib.request.urlopen(r, timeout=30) as resp:
                b = resp.read()
                return resp.status, (b if raw else json.loads(b or b'null')), resp.headers
        except urllib.error.HTTPError as e:
            b = e.read()
            try:
                return e.code, json.loads(b), e.headers
            except ValueError:
                return e.code, b, e.headers

    def auth(self):
        return {'Authorization': 'Bearer ' + self.key}

    def test_the_page_signs_in_with_the_pin_and_keeps_its_headers_safe(self):
        code, body, h = self.req('/', raw=True)
        self.assertEqual(code, 200)
        self.assertIn(b'Voice Studio', body)
        self.assertEqual(h['Referrer-Policy'], 'same-origin', 'never no-referrer (Origin null broke Server mode 1.6)')
        self.assertIn("frame-ancestors 'none'", h['Content-Security-Policy'])
        self.assertEqual(self.req('/api/state')[0], 401)
        self.assertEqual(self.req('/api/login', {'pin': '2468'})[0], 403, 'no X-Hearth: not from the page')
        self.assertEqual(self.req('/api/login', {'pin': '0000'}, {'X-Hearth': '1'})[0], 401)
        code, _, h = self.req('/api/login', {'pin': '2468'}, {'X-Hearth': '1'})
        self.assertEqual(code, 200)
        cookie = h['Set-Cookie']
        for want in ('HttpOnly', 'SameSite=Strict', 'Path=/'):
            self.assertIn(want, cookie)
        c = {'Cookie': cookie.split(';')[0]}
        code, st, _ = self.req('/api/state', headers=c)
        self.assertEqual(code, 200)
        self.assertTrue(st['caps']['speak'])
        self.assertEqual(self.req('/api/speak', {'input': 'hi'}, c)[0], 403, 'a signed-in write still needs X-Hearth')
        self.assertEqual(self.req('/api/speak', {'input': 'hi'}, dict(c, **{'X-Hearth': '1', 'Sec-Fetch-Site': 'cross-site'}))[0], 403)
        code, wav, h = self.req('/api/speak', {'input': 'Hello there.', 'voice': 'narrator'}, dict(c, **{'X-Hearth': '1'}), raw=True)
        self.assertEqual(code, 200)
        self.assertEqual(wav[:4], b'RIFF')
        self.assertEqual(self.req('/api/logout', {}, dict(c, **{'X-Hearth': '1'}))[0], 200)
        self.assertEqual(self.req('/api/state', headers=c)[0], 401, 'signed out')

    def test_five_wrong_pins_wait_five_minutes(self):
        for _ in range(5):
            self.req('/api/login', {'pin': '0000'}, {'X-Hearth': '1'})
        self.assertEqual(self.req('/api/login', {'pin': '2468'}, {'X-Hearth': '1'})[0], 429)

    def test_openai_compatible_speech_models_and_voices(self):
        self.assertEqual(self.req('/v1/models')[0], 401)
        self.assertEqual(self.req('/v1/models', headers={'Authorization': 'Bearer wrong-key-123'})[0], 401)
        code, m, _ = self.req('/v1/models', headers=self.auth())
        self.assertEqual(code, 200)
        self.assertIn('tts-1', [x['id'] for x in m['data']])
        code, v, _ = self.req('/v1/audio/voices', headers=self.auth())
        ids = [x['id'] for x in v['voices']]
        self.assertIn('narrator', ids)
        code, wav, h = self.req('/v1/audio/speech', {'model': 'tts-1', 'input': 'Hello from Hearth.', 'voice': 'nova', 'response_format': 'wav'}, self.auth(), raw=True)
        self.assertEqual(code, 200)
        self.assertEqual(h['Content-Type'], 'audio/wav')
        with wave.open(io.BytesIO(wav)) as w:
            self.assertEqual(w.getframerate(), 24000)
            self.assertGreater(w.getnframes(), 1000)
        code, pcm, _ = self.req('/v1/audio/speech', {'input': 'Hi.', 'voice': 'alloy', 'response_format': 'pcm'}, self.auth(), raw=True)
        self.assertEqual(code, 200)
        self.assertNotEqual(pcm[:4], b'RIFF')
        if HAVE_FFMPEG:
            code, mp3, h = self.req('/v1/audio/speech', {'input': 'Hi there.', 'voice': 'narrator'}, self.auth(), raw=True)
            self.assertEqual((code, h['Content-Type']), (200, 'audio/mpeg'), 'mp3 by default, like OpenAI')
        code, e, _ = self.req('/v1/audio/speech', {'input': 'Hi', 'voice': 'nobody-here'}, self.auth())
        self.assertEqual(code, 400)
        self.assertIn('no voice called', e['error']['message'])
        self.assertEqual(self.req('/v1/audio/speech', {'input': '', 'voice': 'narrator'}, self.auth())[0], 400)
        # OpenAI's "instructions" become a designed voice.
        self.req('/v1/audio/speech', {'input': 'Hi.', 'voice': 'auto', 'instructions': 'a whispering old woman', 'response_format': 'wav'}, self.auth(), raw=True)
        self.assertEqual(self.eng.calls[-1][3], 'female, elderly, whisper')

    def test_openai_compatible_transcriptions_from_a_form(self):
        clip = tone_wav(os.path.join(self.d, 'c.wav'), 6)
        boundary = 'XyZhearth'
        parts = []
        for name, val in (('model', b'whisper-1'), ('response_format', b'srt')):
            parts.append(b'--' + boundary.encode() + b'\r\nContent-Disposition: form-data; name="' + name.encode() + b'"\r\n\r\n' + val + b'\r\n')
        parts.append(b'--' + boundary.encode() + b'\r\nContent-Disposition: form-data; name="file"; filename="c.wav"\r\nContent-Type: audio/wav\r\n\r\n'
                     + open(clip, 'rb').read() + b'\r\n--' + boundary.encode() + b'--\r\n')
        h = dict(self.auth(), **{'Content-Type': 'multipart/form-data; boundary=' + boundary})
        code, srt, _ = self.req('/v1/audio/transcriptions', b''.join(parts), h, raw=True)
        self.assertEqual(code, 200)
        self.assertIn(b'00:00:00,000 --> 00:00:02,500', srt)
        body = b''.join(parts).replace(b'srt', b'json')
        code, j, _ = self.req('/v1/audio/transcriptions', body, h)
        self.assertEqual(code, 200)
        self.assertIn('castle', j['text'])
        code, _, _ = self.req('/v1/audio/translations', body, h)
        self.assertEqual(code, 200)
        self.assertEqual(self.eng.calls[-1][3], 'translate')

    def test_cloning_needs_consent_and_lands_in_the_library(self):
        clip = open(tone_wav(os.path.join(self.d, 'c.wav'), 7), 'rb').read()
        code, e, _ = self.req('/api/voices/clone?name=Grandpa', clip, self.auth())
        self.assertEqual(code, 400)
        self.assertIn('permission', e['error'])
        self.assertEqual(os.listdir(self.studio.vdir), [], 'nothing kept without consent')
        code, v, _ = self.req('/api/voices/clone?name=Grandpa&consent=1', clip, self.auth())
        self.assertEqual(code, 200)
        self.assertEqual((v['name'], v['kind']), ('Grandpa', 'clone'))
        self.assertTrue(v['consent']['at'])
        self.assertIn('castle', v['refText'], 'the words were written down by themselves')
        code, short, _ = self.req('/api/voices/clone?name=X&consent=1', open(tone_wav(os.path.join(self.d, 's.wav'), 1), 'rb').read(), self.auth())
        self.assertEqual(code, 400)
        self.assertIn('too short', short['error'])
        # every feature and the API use it by name
        self.req('/v1/audio/speech', {'input': 'Hello.', 'voice': 'grandpa', 'response_format': 'wav'}, self.auth(), raw=True)
        self.assertTrue(self.eng.calls[-1][2].endswith(os.path.join(v['id'], 'ref.wav')))
        self.assertIn(v['id'], [x['id'] for x in self.req('/v1/audio/voices', headers=self.auth())[1]['voices']])
        code, r, _ = self.req('/api/voices/%s/rename' % v['id'], {'name': 'Grandad <b>'}, self.auth())
        self.assertEqual(r['name'], 'Grandad b')
        code, wav, _ = self.req('/api/voices/%s/preview' % v['id'], headers=self.auth(), raw=True)
        self.assertEqual(wav[:4], b'RIFF')
        self.assertEqual(self.req('/api/voices/%s/delete' % v['id'], {}, self.auth())[0], 200)
        self.assertEqual(self.studio.voices(), [])

    def test_design_a_voice_from_words_then_keep_it(self):
        code, wav, h = self.req('/api/design/preview', {'description': 'a little girl with a high voice'}, self.auth(), raw=True)
        self.assertEqual(code, 200)
        self.assertEqual(h['X-Instruct'], 'female, child, high pitch')
        code, v, _ = self.req('/api/voices/design', {'name': 'Fairy', 'description': 'female, young adult, whisper'}, self.auth())
        self.assertEqual((v['kind'], v['instruct']), ('design', 'female, young adult, whisper'))
        self.assertTrue(os.path.exists(os.path.join(self.studio.vdir, v['id'], 'ref.wav')), 'a sample is kept so it stays the same person')
        self.assertEqual(self.req('/api/design/preview', {'description': 'something'}, self.auth())[0], 400)

    def test_downloads_are_only_a_jobs_own_files(self):
        j = self.studio.new_audiobook('Book', [{'title': 'One', 'text': 'Hello there. The end.'}], 'narrator')
        self.studio.work_once()
        job = self.studio.job(j['id'])
        name = job['outputs'][0]['file']
        code, data, h = self.req('/api/jobs/%s/files/%s' % (j['id'], urllib.request.quote(name)), headers=self.auth(), raw=True)
        self.assertEqual(code, 200)
        self.assertIn('attachment', h['Content-Disposition'])
        for bad in ('job.json', 'chapters.json', '..%2F..%2Fvoice-studio-web.json', '%2Fetc%2Fpasswd'):
            self.assertEqual(self.req('/api/jobs/%s/files/%s' % (j['id'], bad), headers=self.auth())[0], 404, bad)

    def test_uploads_make_jobs_in_the_queue(self):
        code, j, _ = self.req('/api/jobs/audiobook', {'title': 'Story', 'text': 'Chapter 1\n\nOne.\n\nChapter 2\n\nTwo.', 'voice': 'narrator'}, self.auth())
        self.assertEqual((code, j['state'], j['kind']), (202, 'queued', 'audiobook'))
        code, t, _ = self.req('/api/jobs/transcribe?name=meeting.wav&labels=1', open(tone_wav(os.path.join(self.d, 'm.wav'), 5), 'rb').read(), self.auth())
        self.assertEqual((code, t['kind']), (202, 'transcribe'))
        code, e, _ = self.req('/api/jobs/dub?name=x.exe&target=es', b'MZ', self.auth())
        self.assertEqual(code, 400)
        self.assertEqual(os.listdir(self.studio.updir), [], 'a refused upload leaves nothing behind')
        self.assertEqual(len(self.req('/api/jobs', headers=self.auth())[1]['jobs']), 2)


# ---------------------------------------------------------------------------------------------------- the queue

class Queue(Base):
    def test_an_audiobook_narrates_every_chapter_into_files(self):
        chs = vs.split_chapters('Chapter 1: Dawn\n\nThe gate opened. Snow fell.\n\nChapter 2: Road\n\nThey rode north.')
        self.assertEqual([c['title'] for c in chs], ['Chapter 1: Dawn', 'Chapter 2: Road'])
        j = self.studio.new_audiobook('The Gate', chs, 'narrator')
        self.assertTrue(self.studio.work_once())
        job = self.studio.job(j['id'])
        self.assertEqual(job['state'], 'done', job.get('error'))
        files = [o['file'] for o in job['outputs']]
        ext = 'mp3' if HAVE_FFMPEG else 'wav'
        self.assertIn('ch001.' + ext, files)
        self.assertIn('ch002.' + ext, files)
        self.assertIn('The Gate (chapters).zip', files)
        if HAVE_FFMPEG:
            self.assertIn('The Gate.m4b', files)
        with zipfile.ZipFile(self.studio.jpath(j['id'], 'The Gate (chapters).zip')) as z:
            self.assertEqual(len(z.namelist()), 2)
        self.assertEqual(job['summary']['chapters'], 2)

    def test_work_carries_on_after_a_restart_from_the_chapter_it_reached(self):
        chs = [{'title': 'One', 'text': 'First chapter words.'}, {'title': 'Two', 'text': 'Second chapter words.'}]
        j = self.studio.new_audiobook('Book', chs, 'narrator')
        # Voice Studio stopped half way (Light mode, a restart): chapter one done, the job still "running" on disk.
        jid = j['id']
        job = self.studio.job(jid)
        name = 'ch001.' + ('mp3' if HAVE_FFMPEG else 'wav')
        open(self.studio.jpath(jid, name), 'wb').write(b'x')
        job['chapters'][0].update(done=True, secs=1.0, file=name)
        job['state'] = 'running'
        self.studio.job_save(job)
        fresh = vs.Studio(vs.DATA, self.eng)                    # a new process
        self.assertEqual(fresh.recover(), 1)
        self.assertEqual(fresh.job(jid)['state'], 'queued')
        self.assertIn('restarted', fresh.job(jid)['note'])
        self.eng.calls.clear()
        fresh.work_once()
        spoken = [c[1] for c in self.eng.calls if c[0] == 'speak' and c[1] != vs.SAMPLE_TEXT]
        self.assertEqual(spoken, ['Second chapter words.'], 'chapter one isn\'t narrated again')
        self.assertEqual(fresh.job(jid)['state'], 'done')

    def test_stopping_puts_the_running_job_back_in_the_queue(self):
        j = self.studio.new_audiobook('Book', [{'title': 'One', 'text': 'Words.'}], 'narrator')
        self.studio.stop.set()
        self.studio.work_once()
        self.assertEqual(self.studio.job(j['id'])['state'], 'queued')
        self.assertIn('paused', self.studio.job(j['id'])['step'])

    def test_cancel_retry_and_delete(self):
        j = self.studio.new_audiobook('Book', [{'title': 'One', 'text': 'Words.'}], 'narrator')
        self.studio.job_cancel(j['id'])
        self.assertEqual(self.studio.job(j['id'])['state'], 'cancelled')
        self.assertFalse(self.studio.work_once(), 'nothing waiting')
        self.studio.job_retry(j['id'])
        self.assertTrue(self.studio.work_once())
        self.assertEqual(self.studio.job(j['id'])['state'], 'done')
        self.studio.job_delete(j['id'])
        self.assertIsNone(self.studio.job(j['id']))
        with self.assertRaises(vs.Fail):
            self.studio.job_delete('0123456789ab')

    def test_a_failure_says_why_and_the_queue_goes_on(self):
        def boom(*a, **k):
            raise RuntimeError('CUDA out of memory')
        self.eng.speak = boom
        j = self.studio.new_audiobook('Book', [{'title': 'One', 'text': 'Words.'}], 'narrator')
        self.studio.work_once()
        job = self.studio.job(j['id'])
        self.assertEqual(job['state'], 'failed')
        self.assertIn('CUDA out of memory', job['error'])

    def test_a_transcript_with_speakers_in_every_format(self):
        src = tone_wav(os.path.join(self.d, 'meeting.wav'), 7)
        tmp = os.path.join(self.studio.updir, 'u')
        shutil.copy(src, tmp)
        j = self.studio.new_transcribe(tmp, 'meeting.wav', 'en', True)
        self.studio.work_once()
        job = self.studio.job(j['id'])
        self.assertEqual(job['state'], 'done', job.get('error'))
        self.assertEqual(sorted(o['kind'] for o in job['outputs']), ['json', 'srt', 'txt', 'vtt'])
        vtt = open(self.studio.jpath(j['id'], 'meeting.vtt')).read()
        self.assertTrue(vtt.startswith('WEBVTT'))
        self.assertIn('<v Speaker 2>', vtt)
        self.assertEqual(job['summary']['speakers'], 2)

    @unittest.skipUnless(HAVE_FFMPEG and shutil.which('ffprobe'), 'needs ffmpeg')
    def test_a_video_is_dubbed_in_each_speakers_voice_and_timed_to_the_original(self):
        import subprocess
        video = os.path.join(self.d, 'castle.mp4')
        subprocess.run(['ffmpeg', '-loglevel', 'error', '-y', '-f', 'lavfi', '-i', 'testsrc=d=6:s=160x120:r=10', '-f', 'lavfi', '-i', 'sine=f=300:d=6',
                        '-shortest', '-c:v', 'mpeg4', '-c:a', 'aac', video], check=True)
        asked = []

        def fake_translate(lines, target, model=''):
            asked.append((target, len(lines)))
            return ['[%s] %s' % (target, x) for x in lines], 'qwen3:14b'
        vs.translate = fake_translate
        tmp = os.path.join(self.studio.updir, 'v')
        shutil.copy(video, tmp)
        j = self.studio.new_dub(tmp, 'castle.mp4', 'es', 'clone', 'en')
        self.studio.work_once()
        job = self.studio.job(j['id'])
        self.assertEqual(job['state'], 'done', job.get('error'))
        self.assertEqual(asked, [('es', 3)])
        out = self.studio.jpath(j['id'], 'castle Spanish.mp4')
        probe = subprocess.run(['ffprobe', '-v', 'error', '-show_entries', 'stream=codec_type', '-of', 'csv=p=0', out], capture_output=True, text=True).stdout.split()
        self.assertEqual(sorted(probe), ['audio', 'video'])
        self.assertIn('[es] Welcome to the castle.', open(self.studio.jpath(j['id'], 'castle Spanish.srt')).read())
        self.assertEqual(job['summary']['speakers'], 2, 'each speaker got their own cloned voice')
        durations = [round(c[5], 2) for c in self.eng.calls if c[0] == 'speak']
        self.assertEqual(durations[:2], [2.5, 2.5], 'each line is made to the length of the original line')
        self.assertEqual(len(durations), 3)
        self.assertTrue(job['summary']['background'])
        self.assertFalse(os.path.exists(self.studio.jpath(j['id'], 'vocals.wav')), 'the big in-between files go when it\'s done')


# ---------------------------------------------------------------------------------------------------- text and audio helpers

class Helpers(Base):
    def test_text_is_spoken_in_whole_sentences(self):
        t = 'One. Two! Three? ' + 'word ' * 120
        cs = vs.chunks(t, 100)
        self.assertTrue(all(len(c) <= 100 for c in cs))
        self.assertEqual(cs[0], 'One. Two! Three?')
        self.assertEqual(vs.chunks(''), [])

    def test_chapters_from_text_and_from_an_epub(self):
        self.assertEqual(len(vs.split_chapters('Just a short story.')), 1)
        long = '\n\n'.join(['Paragraph %d. ' % i + 'x' * 900 for i in range(40)])
        parts = vs.split_chapters(long)
        self.assertGreater(len(parts), 1)
        self.assertTrue(parts[0]['title'].startswith('Part'))
        md = vs.split_chapters('# The Start\n\nHello.\n\n# The End\n\nBye.')
        self.assertEqual([c['title'] for c in md], ['The Start', 'The End'])
        ep = os.path.join(self.d, 'b.epub')
        body = '<p>' + 'The wind howled over the moor. ' * 20 + '</p>'
        with zipfile.ZipFile(ep, 'w') as z:
            z.writestr('mimetype', 'application/epub+zip')
            z.writestr('META-INF/container.xml', '<container><rootfiles><rootfile full-path="OEBPS/c.opf"/></rootfiles></container>')
            z.writestr('OEBPS/c.opf', '<package xmlns="http://www.idpf.org/2007/opf"><metadata xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:title>Moor</dc:title></metadata>'
                       '<manifest><item id="n" href="nav.xhtml" media-type="application/xhtml+xml" properties="nav"/>'
                       '<item id="a" href="a.xhtml" media-type="application/xhtml+xml"/><item id="b" href="b.xhtml" media-type="application/xhtml+xml"/>'
                       '<item id="c" href="cover.xhtml" media-type="application/xhtml+xml"/></manifest>'
                       '<spine><itemref idref="n"/><itemref idref="c"/><itemref idref="a"/><itemref idref="b"/></spine></package>')
            z.writestr('OEBPS/nav.xhtml', '<html><body><h1>Contents</h1>' + body + '</body></html>')
            z.writestr('OEBPS/cover.xhtml', '<html><body><p>Cover</p></body></html>')
            z.writestr('OEBPS/a.xhtml', '<html><head><title>x</title><style>p{}</style></head><body><h1>The Storm</h1>' + body + '</body></html>')
            z.writestr('OEBPS/b.xhtml', '<html><body><h2>After</h2>' + body + '<script>alert(1)</script></body></html>')
        title, chs = vs.epub_chapters(ep)
        self.assertEqual(title, 'Moor')
        self.assertEqual([c['title'] for c in chs], ['The Storm', 'After'], 'the contents page and the cover are left out')
        self.assertNotIn('alert', chs[1]['text'])
        with self.assertRaises(vs.Fail):
            vs.epub_chapters(tone_wav(os.path.join(self.d, 'x.wav'), 1))

    def test_subtitles(self):
        segs = [{'start': 0, 'end': 1.5, 'text': 'Hi', 'speaker': 'Speaker 1'}, {'start': 3661.25, 'end': 3662, 'text': 'Bye'}]
        srt = vs.to_srt(segs)
        self.assertIn('1\n00:00:00,000 --> 00:00:01,500\n[Speaker 1] Hi', srt)
        self.assertIn('01:01:01,250 --> 01:01:02,000', srt)
        self.assertIn('00:00:00.000 --> 00:00:01.500\n<v Speaker 1>Hi', vs.to_vtt(segs))

    def test_a_description_becomes_omnivoices_words(self):
        self.assertEqual(vs.describe_to_instruct('A grumpy old pirate with a deep, gravelly voice'), 'male, elderly, low pitch')
        self.assertEqual(vs.describe_to_instruct('young woman, British, whispering'), 'female, young adult, whisper, british accent')
        self.assertEqual(vs.describe_to_instruct('female, teenager, very high pitch, korean accent'), 'female, teenager, very high pitch, korean accent')
        self.assertEqual(vs.describe_to_instruct('???'), '')

    def test_speakers_are_told_apart_by_their_voice_fingerprints(self):
        a, b = [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]
        self.assertEqual(vs.cluster([a, b, [0.95, 0.05, 0], [0.1, 0.9, 0], a]), [0, 1, 0, 1, 0])
        self.assertEqual(len(set(vs.cluster([[float(i == k) for i in range(12)] for k in range(12)], most=8))), 8)

    def test_lines_are_placed_and_fitted_to_their_slot(self):
        track = bytearray()
        vs.place(track, b'\x01\x00' * 100, 0.5, sr=1000)
        self.assertEqual(len(track), 1200)
        self.assertEqual(track[1000:1002], b'\x01\x00')
        long = vs.MockEngines(self.d).speak('x' * 100)[0]               # about 5.5 s
        fitted = vs.fit(long, 24000, 2.0, self.d)
        self.assertLessEqual(vs.secs(fitted), 2.0 + 1e-6)
        short = b'\0\0' * 24000
        self.assertEqual(vs.fit(short, 24000, 3.0, self.d), short, 'never slowed down')

    def test_translation_keeps_every_line_and_asks_again_line_by_line(self):
        calls = []

        def ask(method, path, body, timeout=0):
            calls.append(body)
            lines = json.loads(body['messages'][1]['content'].split('\n\n', 1)[1])
            if len(lines) > 1:
                return {'message': {'content': json.dumps({'lines': lines[:-1]})}}      # one short: asked again one by one
            return {'message': {'content': json.dumps({'lines': ['ES ' + lines[0]]})}}
        out, model = vs.translate(['a', 'b', 'c'], 'es', 'qwen3:14b', ask=ask)
        self.assertEqual(out, ['ES a', 'ES b', 'ES c'])
        self.assertEqual(calls[0]['model'], 'qwen3:14b')
        self.assertIs(calls[0]['think'], False)


# ---------------------------------------------------------------------------------------------------- install and remove

class Install(Base):
    def test_install_makes_a_venv_from_the_official_indexes_and_notes_what_didnt_install(self):
        calls = []
        os.makedirs(vs.DATA, exist_ok=True)

        def runner(cmd, timeout=0, inp=None, env=None, cwd=None):
            calls.append(cmd)
            if cmd[:3] == ['/usr/bin/python3', '-m', 'venv']:
                os.makedirs(os.path.join(vs.VENV, 'bin'), exist_ok=True)
                open(os.path.join(vs.VENV, 'bin', 'python'), 'w').close()
            if 'speechbrain==1.0.3' in cmd:
                return 1, b'', 'no wheel'
            if cmd[-1].startswith('import json, torch'):
                return 0, b'{"torch": "2.9.0+cu128", "cuda": true, "gpu": "RTX 5070"}\n', ''
            return 0, b'', ''
        old = vs.free_gb
        vs.free_gb = lambda p: 100
        try:
            rec = vs.install(runner=runner)
        finally:
            vs.free_gb = old
        flat = [' '.join(c) for c in calls]
        self.assertTrue(any('--index-url https://download.pytorch.org/whl/cu128 torch torchaudio' in c for c in flat))
        self.assertTrue(any('omnivoice' in c and 'faster-whisper==1.1.1' in c for c in flat))
        self.assertTrue(any('--no-deps demucs==4.0.1' in c for c in flat))
        self.assertFalse(any('sudo' in c or 'shell' in c for c in flat))
        # pip's install options come after the command: "pip --progress-bar off install" stops with "no such option".
        for c in calls:
            if '--progress-bar' in c:
                self.assertLess(c.index('install'), c.index('--progress-bar'), ' '.join(c))
        self.assertTrue(rec['cuda'])
        self.assertTrue(any('Speaker labels' in n for n in rec['notes']))
        self.assertTrue(vs.installed())

    def test_install_refuses_without_room(self):
        old = vs.free_gb
        vs.free_gb = lambda p: 3
        try:
            with self.assertRaises(vs.Fail):
                vs.install(runner=lambda *a, **k: (0, b'', ''))
        finally:
            vs.free_gb = old

    def test_remove_keeps_your_voices_unless_you_confirmed_twice(self):
        v = self.studio.design_save('Keep me', 'an old man')
        for n in ('venv/bin', 'models/hf'):
            os.makedirs(os.path.join(vs.DATA, n), exist_ok=True)
        r = vs.remove(False)
        self.assertIn('venv', r['removed'])
        self.assertIn('models', r['removed'])
        self.assertTrue(self.studio.voice(v['id']), 'your voices stay')
        r = vs.remove(True)
        self.assertIn('voices', r['removed'])
        self.assertFalse(os.path.exists(os.path.join(vs.DATA, 'voices')))


# ---------------------------------------------------------------------------------------------------- how it fits Hearth

class FitsHearth(unittest.TestCase):
    def test_the_home_server_treats_it_like_ai_pictures(self):
        hs = load('hearth_home_server_vs', SHELL / 'hearth-home-server')
        self.assertEqual(hs.DEFAULTS['studio'], {'on': False, 'pause': True}, 'off until you turn it on; stops while you play')
        self.assertEqual(hs.NESTED['studioOn'], ('studio', 'on', 'bool'))
        self.assertEqual(hs.AI_UNITS['voicestudio'], 'hearth-voice-studio.service')
        old = hs.studio_installed
        hs.studio_installed = lambda: False
        try:
            with self.assertRaises(hs.Fail):
                hs.user_app_precheck('studioOn', 'on')
        finally:
            hs.studio_installed = old
        src = (SHELL / 'hearth-home-server').read_text()
        for cmd in ("'studio-install'", "'studio-remove', 'voices', 'sure', 'really-sure'", "'studio-pin'", "'studio-key'", "'studio-key-new'"):
            self.assertIn(cmd, src)
        self.assertIn("8890: 'Voice Studio'", src)

    def test_root_puts_its_port_in_the_tailscale_only_rule_and_light_mode_stops_it(self):
        hh = load('hearth_home_server_helper_vs', ROOT / 'os' / 'system_files' / 'usr' / 'libexec' / 'nimbus' / 'hearth-home-server-helper')
        self.assertEqual(hh.USER_APPS['voicestudio'], ('studio', 'hearth-voice-studio.service', True))
        p = hh.services_plan({'studio': {'on': True}}, 'owner')
        self.assertEqual(p['ports']['tcp'], [8890])
        self.assertEqual(p['studio'], {'on': True, 'pause': True})
        self.assertTrue(hh.services_timer_needed({'studio': {'on': True}}), 'Light mode needs its timer')
        self.assertEqual(hh.services_plan({}, 'owner')['ports']['tcp'], [])

    def test_its_unit_is_never_part_of_starting_and_the_image_bakes_nothing_in(self):
        unit = (UNITS / 'hearth-voice-studio.service').read_text()
        self.assertNotIn('[Install]', unit)
        self.assertIn('ConditionPathExists=%h/.local/share/nimbus/voice-studio/venv/bin/python', unit)
        self.assertIn('ai-run voicestudio', unit)
        timer = (UNITS / 'hearth-voice-studio.timer').read_text()
        self.assertIn('WantedBy=timers.target', timer)
        b = (ROOT / 'os-bases' / 'peppermint' / 'build.sh').read_text()
        self.assertIn('hearth-voice-studio.timer', [l for l in b.splitlines() if 'systemctl --global disable' in l and 'hearth-kokoro.timer' in l][0])
        self.assertNotIn('omnivoice', b.lower())
        y = (ROOT / '.github' / 'workflows' / 'base-peppermint.yml').read_text()
        self.assertIn('hearth-comfyui hearth-kokoro hearth-voice-studio', y)
        self.assertIn('test_hearth_voice_studio.py', (ROOT / '.github' / 'workflows' / 'home-server.yml').read_text())

    def test_settings_offer_voice_studio_and_keep_kokoro(self):
        svc = (SHELL / 'SetHomeServices.qml').read_text()
        for want in ('"studio-install"', '"studioOn"', '"studioPause"', '"studio-pin"', '"studio-key"', '"studio-key-new"', '"studio-remove"',
                     '"studio-remove", "voices", "sure", "really-sure"', 'SetPinPad', 'OpenAI-compatible speech API', 'CC BY-NC',
                     '"voices-install"', '"voicesOn"', 'title: "Install AI voices (Kokoro)"'):
            self.assertIn(want, svc)
        # The PIN goes to the helper on stdin (SetJob's input), never on its command line.
        self.assertIn('act.run(["studio-pin"], () =>', svc)
        self.assertIn('page.say(e, true); }, pin);', svc.split('act.run(["studio-pin"]')[1].split('SetRow')[0])
        self.assertIn('Voice Studio', (SHELL / 'SetHomeAI.qml').read_text())

    def test_no_shell_no_listening_everywhere_and_no_copied_code(self):
        src = (SHELL / 'hearth-voice-studio').read_text()
        self.assertNotIn('shell=True', src)
        self.assertNotIn('0.0.0.0', src)
        self.assertNotIn('voicestudio.sh', src.lower())
        self.assertNotIn('AGPL', src.replace('Nothing AGPL', ''))
        page = (SHELL / 'voice-studio-web.html').read_text()
        self.assertIn('/*HEARTH_DATA*/null', page)
        self.assertIn("'X-Hearth': '1'", page)
        self.assertNotIn('http://', page.replace('http://www.w3.org', ''), 'no outside resources')


if __name__ == '__main__':
    unittest.main()
