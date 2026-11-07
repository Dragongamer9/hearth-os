"""Tests for AI video (Hearth OS 1.7): Wan 2.2 for Hearth's ComfyUI, only when asked (Settings › Home server ›
Services › AI pictures › Add video). The GGUF loader at a pinned commit, the 14B models sized by the graphics card
(Q4_K_M from 12 GB, Q3_K_M from 8 GB, nothing under that), the text encoder and VAE, each file checked against the size
and sha256 Hugging Face lists before it's put in place (resuming a .part), files already there kept, removing only
what Hearth put there, and no LoRA downloaded. The network and the system's commands are replaced here."""
import sys
sys.dont_write_bytecode = True
import hashlib
import io
import json
import os
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from test_hearth_home_server import hs, SHELL  # noqa: E402
import test_hearth_home_ai as ai  # noqa: E402

BLOBS = {}


def blob(name):
    return BLOBS.setdefault(name, ('wan:' + name).encode() * 3)


def listing_for(quant='Q4_K_M', broken=None):
    def listing(repo, folder):
        out = []
        if repo.startswith('QuantStack/'):
            kind = 'I2V' if 'I2V' in repo else 'T2V'
            for q in ('Q3_K_M', 'Q4_K_M', 'Q5_K_M'):
                path = '%s/Wan2.2-%s-A14B-%s-%s.gguf' % (folder, kind, folder, q)
                b = blob(path)
                out.append({'path': path, 'size': len(b), 'sha256': hashlib.sha256(b).hexdigest()})
        else:
            for name in ('umt5_xxl_fp8_e4m3fn_scaled.safetensors', 'umt5_xxl_fp16.safetensors', 'wan_2.1_vae.safetensors'):
                path = folder + '/' + name
                b = blob(path)
                out.append({'path': path, 'size': len(b), 'sha256': hashlib.sha256(b).hexdigest()})
        if broken:
            out = [f for f in out if broken not in f['path']]
        return out
    return listing


class FakeFetch:
    def __init__(self):
        self.got = []

    def __call__(self, f, dest, progress=lambda d: None):
        self.got.append(f)
        data = blob(f['path'])
        progress(len(data))
        with open(dest, 'wb') as out:
            out.write(data)


class Video(unittest.TestCase):
    # The same stand-ins as test_hearth_home_ai's MantelAI (its own tests aren't run again here).
    make_comfy = ai.MantelAI.make_comfy

    def setUp(self):
        ai.MantelAI.setUp(self)
        for k in ('gpu_vram_gb',):
            self.saved[k] = getattr(hs, k)
        hs.gpu_vram_gb = lambda: 12.0
        self.make_comfy()

    def files_in(self, sub):
        p = os.path.join(self.cd, 'models', sub)
        return sorted(os.listdir(p)) if os.path.isdir(p) else []

    def test_sized_by_the_graphics_card(self):
        self.assertEqual(hs.video_quant(12.0), 'Q4_K_M')
        self.assertEqual(hs.video_quant(16.0), 'Q4_K_M')
        self.assertEqual(hs.video_quant(11.0), 'Q3_K_M')
        self.assertEqual(hs.video_quant(8.0), 'Q3_K_M')
        self.assertIsNone(hs.video_quant(6.0))
        self.assertIsNone(hs.video_quant(0))

    def test_only_the_pinned_files_and_every_one_has_a_checksum(self):
        files = hs.video_files('Q4_K_M', False, listing_for())
        names = sorted(f['name'] for f in files)
        self.assertEqual(names, ['Wan2.2-I2V-A14B-HighNoise-Q4_K_M.gguf', 'Wan2.2-I2V-A14B-LowNoise-Q4_K_M.gguf',
                                 'Wan2.2-T2V-A14B-HighNoise-Q4_K_M.gguf', 'Wan2.2-T2V-A14B-LowNoise-Q4_K_M.gguf',
                                 'umt5_xxl_fp8_e4m3fn_scaled.safetensors', 'wan_2.1_vae.safetensors'])
        self.assertEqual(len(hs.video_files('Q3_K_M', True, listing_for())), 4)
        with self.assertRaises(hs.Fail):
            hs.video_files('Q4_K_M', False, listing_for(broken='LowNoise-Q4_K_M'))
        bad = lambda repo, folder: [dict(f, sha256='') for f in listing_for()(repo, folder)]
        with self.assertRaises(hs.Fail):
            hs.video_files('Q4_K_M', False, bad)

    def test_download_puts_everything_in_place_and_starts_nothing(self):
        fetch = FakeFetch()
        r = hs.video_job(False, listing_for(), fetch)
        self.assertEqual(r, {'installed': True, 'quant': 'Q4_K_M'})
        self.assertEqual(len(self.files_in('unet')), 4)
        self.assertEqual(self.files_in('text_encoders'), ['umt5_xxl_fp8_e4m3fn_scaled.safetensors'])
        self.assertEqual(self.files_in('vae'), ['wan_2.1_vae.safetensors'])
        self.assertTrue(os.path.isdir(os.path.join(self.cd, 'custom_nodes', 'ComfyUI-GGUF')))
        self.assertTrue(self.c.ran('git', '-C'), 'the loader is checked out at its pinned commit')
        self.assertIn(hs.VIDEO_NODE_COMMIT, sum(self.c.ran('git', '-C'), []))
        self.assertFalse(self.c.ran('systemctl', '--user', 'restart'), 'ComfyUI wasn\'t running: nothing starts')
        v = hs.video_view(self.cd)
        self.assertTrue(v['installed'])
        self.assertEqual(v['quant'], 'Q4_K_M')
        self.assertFalse(self.files_in('loras'), 'no LoRA is downloaded')
        self.assertFalse([f for f in fetch.got if 'lora' in f['path'].lower()])

    def test_a_running_comfyui_is_restarted_once_to_see_the_loader(self):
        self.active.add(hs.AI_UNITS['comfyui'])
        hs.video_job(False, listing_for(), FakeFetch())
        self.assertEqual(len(self.c.ran('systemctl', '--user', 'restart')), 1)

    def test_files_already_there_are_kept_and_removing_takes_only_what_hearth_put_there(self):
        os.makedirs(os.path.join(self.cd, 'models', 'vae'))
        mine = os.path.join(self.cd, 'models', 'vae', 'wan_2.1_vae.safetensors')
        open(mine, 'w').write('the owner\'s own')
        os.makedirs(os.path.join(self.cd, 'models', 'loras'))
        lora = os.path.join(self.cd, 'models', 'loras', 'mine.safetensors')
        open(lora, 'w').write('x')
        fetch = FakeFetch()
        hs.video_job(True, listing_for(), fetch)
        self.assertNotIn('wan_2.1_vae.safetensors', [f['name'] for f in fetch.got], 'already there: not downloaded again')
        self.assertTrue(hs.video_view(self.cd)['installed'])
        r = hs.video_remove()
        self.assertEqual(r['removed'], 3)
        self.assertTrue(os.path.exists(mine), 'the owner\'s own file stays')
        self.assertTrue(os.path.exists(lora), 'his LoRAs stay')
        self.assertEqual(self.files_in('unet'), [])
        self.assertFalse(os.path.isdir(os.path.join(self.cd, 'custom_nodes', 'ComfyUI-GGUF')))
        self.assertFalse(hs.video_view(self.cd)['installed'])

    def test_an_existing_loader_is_never_removed(self):
        node = os.path.join(self.cd, 'custom_nodes', 'ComfyUI-GGUF')
        os.makedirs(node)
        hs.video_job(False, listing_for(), FakeFetch())
        hs.video_remove()
        self.assertTrue(os.path.isdir(node))

    def test_it_says_why_it_cant(self):
        hs.gpu_vram_gb = lambda: 6.0
        with self.assertRaisesRegex(hs.Fail, 'at least 8 GB'):
            hs.video_job(False, listing_for(), FakeFetch())
        hs.gpu_vram_gb = lambda: 12.0
        hs.free_gb = lambda p: 1
        with self.assertRaisesRegex(hs.Fail, 'free space'):
            hs.video_job(False, listing_for(), FakeFetch())
        self.assertIn('free space', hs.ai_state()['comfyvideo']['error'])

    def test_needs_comfyui_first(self):
        import shutil
        shutil.rmtree(os.path.join(self.cd, 'venv'))
        with self.assertRaisesRegex(hs.Fail, 'ComfyUI'):
            hs.video_job(False, listing_for(), FakeFetch())

    def test_eight_to_eleven_gb_gets_the_smaller_files(self):
        hs.gpu_vram_gb = lambda: 10.0
        fetch = FakeFetch()
        hs.video_job(False, listing_for(), fetch)
        self.assertTrue(all('Q3_K_M' in f['name'] for f in fetch.got if f['name'].endswith('.gguf')))


class Resp(io.BytesIO):
    def __init__(self, data, status):
        super().__init__(data)
        self.status = status

    def __enter__(self):
        return self

    def __exit__(self, *a):
        self.close()


class Fetch(unittest.TestCase):
    def setUp(self):
        import tempfile
        import shutil
        self.d = tempfile.mkdtemp()
        self.addCleanup(shutil.rmtree, self.d)
        self.data = b'0123456789' * 100
        self.f = {'repo': 'QuantStack/x', 'path': 'LowNoise/a.gguf', 'name': 'a.gguf', 'size': len(self.data),
                  'sha256': hashlib.sha256(self.data).hexdigest()}
        self.dest = os.path.join(self.d, 'a.gguf')

    def test_resumes_a_part_and_checks_the_checksum(self):
        open(self.dest + '.part', 'wb').write(self.data[:400])
        seen = []

        def opener(req, timeout=0):
            seen.append(req.get_header('Range'))
            return Resp(self.data[400:], 206)
        hs.fetch_file(self.f, self.dest, opener=opener)
        self.assertEqual(seen, ['bytes=400-'])
        self.assertEqual(open(self.dest, 'rb').read(), self.data)
        self.assertFalse(os.path.exists(self.dest + '.part'))

    def test_a_wrong_file_is_thrown_away_never_used(self):
        def opener(req, timeout=0):
            return Resp(b'X' * len(self.data), 200)
        with self.assertRaisesRegex(hs.Fail, 'checksum'):
            hs.fetch_file(self.f, self.dest, opener=opener)
        self.assertFalse(os.path.exists(self.dest))
        self.assertFalse(os.path.exists(self.dest + '.part'))

    def test_only_from_hugging_face(self):
        urls = []

        def opener(req, timeout=0):
            urls.append(req.full_url)
            return Resp(self.data, 200)
        hs.fetch_file(self.f, self.dest, opener=opener)
        self.assertEqual(urls, ['https://huggingface.co/QuantStack/x/resolve/main/LowNoise/a.gguf'])


class Wiring(unittest.TestCase):
    def test_the_page_and_the_commands(self):
        q = (SHELL / 'SetHomeServices.qml').read_text()
        self.assertIn('act.run(["video-install"]', q)
        self.assertIn('act.run(["video-install", "i2v-only"]', q)
        self.assertIn('act.run(["video-remove"]', q)
        src = (SHELL / 'hearth-home-server').read_text()
        self.assertIn("elif a in (['video-install'], ['video-install', 'i2v-only']):", src)
        code = src.split('# ------------------------------------------------------------------ AI video')[1].split('def video_view')[0]
        self.assertNotIn('loras', code.replace('models (loras', ''), 'no LoRA is downloaded')
        self.assertNotIn('rzgar', src)


if __name__ == '__main__':
    unittest.main()
