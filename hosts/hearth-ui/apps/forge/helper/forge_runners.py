"""forge_runners: how Hearth Forge makes with each model, through the model's own code (docs/mockups/checklists/
hearth-forge.md; models.json's "runners").

Built into the app with hearth-forge.py and written next to it. hearth-forge.py `run JOBDIR` loads this inside the
model's own Python environment (~/Hearth/Forge/runtimes/<runner>: the maker's repo at its pinned commit, the maker's
pinned requirements) and calls make(ctx). Each runner below uses the maker's own classes or its own command line,
exactly as its README shows, with the maker's low-memory or offload settings when the card is smaller than the
maker's figure. Weights only ever come from what Get downloaded and checked (HF_HUB_OFFLINE=1: the local snapshot of
each Hugging Face repo is passed in by its path, or found in Hearth's own Hugging Face folder).

Every runner returns what it made, and finish() normalises it:
  mesh       a trimesh object or a mesh file (.glb .obj .ply mesh): kept as model.glb (or coarse.glb for a refiner)
  splat      a Gaussian splat (.ply): kept as splat.ply; hearth-forge.py also makes a mesh from it (so Send it to works)
  shapeOnly  True when it has no colour of its own: Hunyuan3D 2.0's painter paints it after, when it's got
  note       anything the person should be told (shown under the model)

ctx (made by hearth-forge.py) has: d, work, ask, m, run, src, py, bin, seed, res, steps, detail, tex, tsize, vram,
photo, photos, words, out, coarse, stage(text, pct), snapshot(repo), links(mapping, folder), cli(argv, cwd, env),
cutout(), timing, Fail.
"""
import glob
import os
import shutil
import sys

# how many Gaussians / faces / which sizes for Draft, Good, High, Highest
GAUSSIANS = {'draft': 32768, 'good': 65536, 'high': 131072, 'highest': 262144}
FACES = {'draft': 100000, 'good': 300000, 'high': 500000, 'highest': 1000000}
WORLD = {'draft': 518, 'good': 700, 'high': 952, 'highest': 952}


def _low(ctx, under):
    """The maker's low-memory or offload setting is used when this card has less memory than `under` GB."""
    return ctx.vram < float(ctx.run.get('lowMemoryUnder', under))


def _torch():
    import torch
    return torch


def _gen(ctx, device='cpu'):
    return _torch().Generator(device=device).manual_seed(ctx.seed)


def _dev():
    return 'cuda' if _torch().cuda.is_available() else 'cpu'


def _path(*parts):
    p = os.path.join(*parts)
    if not os.path.exists(p):
        raise FileNotFoundError(p)
    return p


def _find(folder, pattern):
    found = sorted(glob.glob(os.path.join(folder, pattern), recursive=True))
    return found[0] if found else ''


def _syspath(ctx, *sub):
    for s in sub:
        p = os.path.join(ctx.src, s) if s else ctx.src
        if p not in sys.path:
            sys.path.insert(0, p)


# ------------------------------------------------------------------ objects with shape and textures

def hunyuan3d21(ctx):
    """Hunyuan3D 2.1's shape (hy3dshape, as its demo.py), its own background removal; on a card under 16 GB its
    pipeline's model offload (each part on the card only while it runs). Its PBR painter needs 29 GB, so on 12 GB
    the shape is painted by Hunyuan3D 2.0 (as the board says)."""
    _syspath(ctx, 'hy3dshape')
    os.environ['HY3DGEN_MODELS'] = ctx.links({'tencent/Hunyuan3D-2.1': ctx.snapshot('tencent/Hunyuan3D-2.1')}, 'hy3dgen')
    from hy3dshape.rembg import BackgroundRemover
    from hy3dshape.pipelines import Hunyuan3DDiTFlowMatchingPipeline
    ctx.stage('taking out the background', 4)
    img = ctx.cutout(remover=lambda im: BackgroundRemover()(im.convert('RGB')))
    ctx.stage('making the shape', 10)
    pipe = Hunyuan3DDiTFlowMatchingPipeline.from_pretrained('tencent/Hunyuan3D-2.1', subfolder='hunyuan3d-dit-v2-1')
    if _low(ctx, 16):
        pipe.enable_model_cpu_offload()
    mesh = pipe(image=img, num_inference_steps=ctx.steps, octree_resolution=ctx.res, guidance_scale=5.0, generator=_gen(ctx), output_type='trimesh')[0]
    return {'mesh': mesh, 'shapeOnly': True}


def trellis2(ctx):
    """TRELLIS.2 (as its example.py): its low_vram mode and the 1024 pipeline on a card under 24 GB (the board's
    "Fits at 1024 only"), 1536 cascade on a bigger one; its PBR mesh baked by o_voxel to a .glb."""
    _syspath(ctx)
    from trellis2.pipelines import Trellis2ImageTo3DPipeline
    import o_voxel
    from PIL import Image
    ctx.stage('loading TRELLIS.2', 6)
    pipe = Trellis2ImageTo3DPipeline.from_pretrained(ctx.snapshot('microsoft/TRELLIS.2-4B'))
    low = _low(ctx, 24)
    pipe.low_vram = low
    pipe.cuda()
    ctx.stage('making the shape and its materials', 15)
    mesh = pipe.run(Image.open(ctx.photo), seed=ctx.seed, pipeline_type=ctx.run.get('small', '1024') if low else ctx.run.get('big', '1536_cascade'))[0]
    mesh.simplify(16777216)
    ctx.stage('baking its materials', 70)
    glb = o_voxel.postprocess.to_glb(vertices=mesh.vertices, faces=mesh.faces, attr_volume=mesh.attrs, coords=mesh.coords, attr_layout=mesh.layout,
                                     voxel_size=mesh.voxel_size, aabb=[[-0.5, -0.5, -0.5], [0.5, 0.5, 0.5]], decimation_target=FACES.get(ctx.detail, 500000),
                                     texture_size=ctx.tsize, remesh=True, remesh_band=1, remesh_project=0, verbose=False)
    return {'mesh': glb, 'note': 'TRELLIS.2 at 1024 with its low-memory mode' if low else ''}


def trellis(ctx):
    """TRELLIS (as its example.py, example_text.py, example_multi_image.py): a photo, several photos or words
    (TRELLIS-text-xlarge, its own words → 3D). Only the gaussians and the mesh are decoded (no radiance field), which
    keeps it within 12 GB; its .glb baked by its own postprocessing_utils.to_glb, and its splat kept too."""
    _syspath(ctx)
    from trellis.utils import postprocessing_utils
    from PIL import Image
    formats = ['gaussian', 'mesh']
    ss = {'steps': max(12, ctx.steps // 2), 'cfg_strength': 7.5}
    slat = {'steps': max(12, ctx.steps // 2), 'cfg_strength': 3.0}
    if ctx.ask.get('mode') in ('words', 'picture') and not os.path.isfile(ctx.photo):
        from trellis.pipelines import TrellisTextTo3DPipeline
        ctx.stage('loading TRELLIS (words)', 6)
        pipe = TrellisTextTo3DPipeline.from_pretrained(ctx.snapshot('microsoft/TRELLIS-text-xlarge'))
        pipe.cuda()
        ctx.stage('making it from your words', 15)
        out = pipe.run(ctx.words, seed=ctx.seed, formats=formats, sparse_structure_sampler_params=ss, slat_sampler_params=slat)
    else:
        from trellis.pipelines import TrellisImageTo3DPipeline
        ctx.stage('loading TRELLIS', 6)
        pipe = TrellisImageTo3DPipeline.from_pretrained(ctx.snapshot('microsoft/TRELLIS-image-large'))
        pipe.cuda()
        ctx.stage('making the shape and its textures', 15)
        if len(ctx.photos) > 1:
            out = pipe.run_multi_image([Image.open(p) for p in ctx.photos], seed=ctx.seed, formats=formats, sparse_structure_sampler_params=ss, slat_sampler_params=slat)
        else:
            out = pipe.run(Image.open(ctx.photo), seed=ctx.seed, formats=formats, sparse_structure_sampler_params=ss, slat_sampler_params=slat)
    splat = os.path.join(ctx.work, 'trellis.ply')
    out['gaussian'][0].save_ply(splat)
    ctx.stage('baking its textures', 70)
    glb = postprocessing_utils.to_glb(out['gaussian'][0], out['mesh'][0], simplify=0.95, texture_size=min(ctx.tsize, 2048))
    return {'mesh': glb, 'splat': splat}


def spar3d(ctx):
    """SPAR3D through its own run.py; --low-vram-mode on a card under 14 GB (Stability's own "10.5 GB · ~7 GB")."""
    out = os.path.join(ctx.work, 'spar3d')
    args = [ctx.py, os.path.join(ctx.src, 'run.py'), ctx.photo, '--output-dir', out, '--pretrained-model', ctx.snapshot('stabilityai/stable-point-aware-3d'),
            '--texture-resolution', str(min(ctx.tsize, 2048)), '--device', 'cuda']
    if _low(ctx, 14):
        args.append('--low-vram-mode')
    ctx.stage('making the shape and its textures (SPAR3D)', 10)
    ctx.cli(args, cwd=ctx.src)
    return {'mesh': _path(out, '0', 'mesh.glb')}


def pixal3d(ctx):
    """Pixal3D through its own inference.py: --low_vram (1024, models loaded on demand) on a card under 24 GB."""
    out = os.path.join(ctx.work, 'pixal3d.glb')
    args = [ctx.py, os.path.join(ctx.src, 'inference.py'), '--image', ctx.photo, '--output', out, '--seed', str(ctx.seed), '--model_path', ctx.snapshot('TencentARC/Pixal3D')]
    if _low(ctx, 24):
        args += ['--low_vram', '--resolution', '1024']
    ctx.stage('making the shape and its materials (Pixal3D)', 10)
    ctx.cli(args, cwd=ctx.src)
    return {'mesh': _path(out)}


def step1x3d(ctx):
    """Step1X-3D (as its inference.py). StepFun's figure is 27 GB for shape and texture: on a card that big both are
    its own; on a smaller card its geometry pipeline's own model offload makes the shape (each part on the card only
    while it runs) and Hunyuan3D 2.0 paints it, and Hearth says so. A bigger PC of yours makes it whole over Hearth
    Link (Settings › Where models are made)."""
    _syspath(ctx)
    torch = _torch()
    from step1x3d_geometry.models.pipelines.pipeline import Step1X3DGeometryPipeline
    snap = ctx.snapshot('stepfun-ai/Step1X-3D')
    full = ctx.vram >= float(ctx.run.get('fullFrom', 27))
    ctx.stage('taking out the background', 4)
    img = ctx.cutout()
    cut = os.path.join(ctx.d, 'photo-cut.png')
    ctx.stage('making the shape', 10)
    geo = Step1X3DGeometryPipeline.from_pretrained(snap, subfolder='Step1X-3D-Geometry-1300m')
    if full:
        geo = geo.to('cuda')
    else:
        geo.enable_model_cpu_offload()
    out = geo(cut, guidance_scale=7.5, num_inference_steps=ctx.steps, generator=torch.Generator(device='cuda').manual_seed(ctx.seed))
    mesh = out.mesh[0]
    del geo
    torch.cuda.empty_cache()
    if not full or ctx.tex == 'none':
        note = '' if full else 'Step1X-3D\'s own textures need 27 GB: its shape made with its offload, painted by Hunyuan3D 2.0 (a bigger PC makes it whole)'
        return {'mesh': mesh, 'shapeOnly': True, 'note': note}
    from step1x3d_texture.pipelines.step1x_3d_texture_synthesis_pipeline import Step1X3DTexturePipeline
    from step1x3d_geometry.models.pipelines.pipeline_utils import reduce_face, remove_degenerate_face
    ctx.stage('painting its textures', 55)
    tex = Step1X3DTexturePipeline.from_pretrained(snap, subfolder='Step1X-3D-Texture')
    mesh = reduce_face(remove_degenerate_face(mesh))
    return {'mesh': tex(cut, mesh, seed=ctx.seed)}


# ------------------------------------------------------------------ quick drafts and shape only

def triposg(ctx):
    """TripoSG (its scripts/inference_triposg.py's own run_triposg, with its own BriaRMBG cut-out), half precision."""
    _syspath(ctx, '', 'scripts')
    torch = _torch()
    from triposg.pipelines.pipeline_triposg import TripoSGPipeline
    from briarmbg import BriaRMBG
    from inference_triposg import run_triposg
    ctx.stage('loading TripoSG', 6)
    rmbg = BriaRMBG.from_pretrained(ctx.snapshot('briaai/RMBG-1.4')).to('cuda')
    rmbg.eval()
    pipe = TripoSGPipeline.from_pretrained(ctx.snapshot('VAST-AI/TripoSG')).to('cuda', torch.float16)
    ctx.stage('making the shape', 15)
    mesh = run_triposg(pipe, image_input=ctx.photo, rmbg_net=rmbg, seed=ctx.seed, num_inference_steps=ctx.steps, guidance_scale=7.0, faces=-1)
    return {'mesh': mesh, 'shapeOnly': True}


def direct3ds2(ctx):
    """Direct3D-S2 (as its README): sdf_resolution 512 on a card under 24 GB (the board's "Fits at 512 only"), 1024 on
    a bigger one; inside taken out (remove_interior)."""
    _syspath(ctx)
    from direct3d_s2.pipeline import Direct3DS2Pipeline
    ctx.stage('taking out the background', 4)
    ctx.cutout()
    ctx.stage('loading Direct3D-S2', 8)
    pipe = Direct3DS2Pipeline.from_pretrained(os.path.join(ctx.snapshot('wushuang98/Direct3D-S2'), 'direct3d-s2-v-1-1'))
    pipe.to('cuda:0')
    sdf = 1024 if ctx.vram >= float(ctx.run.get('bigFrom', 24)) else 512
    ctx.stage('making the shape at %d' % sdf, 15)
    mesh = pipe(os.path.join(ctx.d, 'photo-cut.png'), sdf_resolution=sdf, remove_interior=True, remesh=False, generator=_gen(ctx, 'cuda'))['mesh']
    return {'mesh': mesh, 'shapeOnly': True}


def hi3dgen(ctx):
    """Hi3DGen (as its app.py): its StableNormal turbo normals from the photo (StableNormal at a pinned commit in
    the runtime's own torch hub), then its normal-bridged TRELLIS shape."""
    _syspath(ctx)
    torch = _torch()
    from hi3dgen.pipelines import Hi3DGenPipeline
    from PIL import Image
    weights = ctx.links({'trellis-normal-v0-1': ctx.snapshot('Stable-X/trellis-normal-v0-1'), 'yoso-normal-v1-8-1': ctx.snapshot('Stable-X/yoso-normal-v1-8-1'),
                         'BiRefNet': ctx.snapshot('ZhengPeng7/BiRefNet')}, 'weights')
    ctx.stage('loading Hi3DGen', 6)
    pipe = Hi3DGenPipeline.from_pretrained(os.path.join(weights, 'trellis-normal-v0-1'))
    pipe.cuda()
    normal = torch.hub.load(os.path.join(torch.hub.get_dir(), 'hugoycj_StableNormal_main'), 'StableNormal_turbo', yoso_version='yoso-normal-v1-8-1',
                            source='local', local_cache_dir=weights, pretrained=True)
    ctx.stage('working out its normals', 15)
    img = pipe.preprocess_image(Image.open(ctx.photo), resolution=1024)
    nimg = normal(img, resolution=768, match_input_resolution=True, data_type='object')
    nimg.save(os.path.join(ctx.d, 'photo-normals.png'))
    ctx.stage('making the shape', 30)
    out = pipe.run(nimg, seed=ctx.seed, formats=['mesh'], preprocess_image=False, sparse_structure_sampler_params={'steps': 50, 'cfg_strength': 3},
                   slat_sampler_params={'steps': 6, 'cfg_strength': 3})
    return {'mesh': out['mesh'][0].to_trimesh(transform_pose=True), 'shapeOnly': True}


def hi3d(ctx):
    """Hi3D 1.0 (hi3d-official/Hi3Dv1.0's own demo/refine_mesh.py): another model's coarse shape (made first by
    Hunyuan3D 2.0 mini, 2.0 or Turbo, whichever is got, or the made model you picked) made finer from the photo.
    --sequential-cpu-offload on a card under 48 GB (Hi3D validated 1536 only on 96 GB); 1536 only on a big card."""
    if not ctx.coarse:
        raise ctx.Fail('Hi3D 1.0 makes another model\'s shape finer: get Hunyuan3D 2.0 mini (or 2.0) for its first shape')
    out = os.path.join(ctx.work, 'hi3d.glb')
    big = ctx.vram >= 48
    res = {'draft': 512, 'good': 1024, 'high': 1024, 'highest': 1536 if big else 1024}.get(ctx.detail, 1024)
    args = [ctx.py, os.path.join(ctx.src, 'demo', 'refine_mesh.py'), '--model', ctx.snapshot('hi3d-official/Hi3Dv1.0'), '--image', ctx.photo,
            '--coarse-mesh', ctx.coarse, '--output', out, '--resolution', str(res), '--num-inference-steps', '25', '--seed', str(ctx.seed),
            '--local-files-only', '--dino-model', ctx.snapshot('facebook/dinov2-large'), '--clip-model', ctx.snapshot('openai/clip-vit-large-patch14'), '--remove-background']
    if not big:
        args.append('--sequential-cpu-offload')
    ctx.stage('making its shape finer (Hi3D 1.0, %d)' % res, 40)
    ctx.cli(args, cwd=ctx.src)
    return {'mesh': _path(out), 'shapeOnly': True}


def ultrashape(ctx):
    """UltraShape 1.0 (its own scripts/infer_dit_refine.py): a coarse shape (Hunyuan3D 2.1's, as UltraShape's README
    says, or 2.0's) made finer; its README's low-VRAM settings (--low_vram, 8192 latents, chunks of 2048) on a card
    under 24 GB."""
    if not ctx.coarse:
        raise ctx.Fail('UltraShape makes another model\'s shape finer: get Hunyuan3D 2.1 (or 2.0) for its first shape')
    out = os.path.join(ctx.work, 'ultrashape')
    ckpt = _find(ctx.snapshot('infinith/UltraShape'), '**/*.pt')
    if not ckpt:
        raise ctx.Fail('UltraShape\'s checkpoint (.pt) isn\'t in what was got')
    oct_res = {'draft': 512, 'good': 768, 'high': 1024, 'highest': 1024}.get(ctx.detail, 1024)
    args = [ctx.py, os.path.join(ctx.src, 'scripts', 'infer_dit_refine.py'), '--config', os.path.join(ctx.src, 'configs', 'infer_dit_refine.yaml'), '--ckpt', ckpt,
            '--image', ctx.photo, '--mesh', ctx.coarse, '--output_dir', out, '--steps', str(ctx.steps), '--seed', str(ctx.seed), '--octree_res', str(oct_res), '--remove_bg']
    if _low(ctx, 24):
        args += ['--low_vram', '--num_latents', '8192', '--chunk_size', '2048']
    ctx.stage('making its shape finer (UltraShape)', 40)
    ctx.cli(args, cwd=ctx.src)
    got = _find(out, '*_refined.glb')
    if not got:
        raise ctx.Fail('UltraShape made nothing')
    return {'mesh': got, 'shapeOnly': True}


def sam3dobjects(ctx):
    """SAM 3D Objects (as its demo.py): the object picked in the photo (the cut-out's own mask), its splat and its
    mesh. Meta's figure is a card with 32 GB and it has no low-memory mode, so hearth-forge.py sends a smaller card
    to your bigger PC."""
    _syspath(ctx, 'notebook')
    import numpy as np
    ctx.links({'hf': os.path.join(ctx.snapshot('facebook/sam-3d-objects'), 'checkpoints')}, os.path.join(ctx.src, 'checkpoints'))
    from inference import Inference
    ctx.stage('picking out the object', 4)
    cut = ctx.cutout()
    rgba = np.array(cut.convert('RGBA'))
    mask = rgba[..., 3] > 127
    ctx.stage('loading SAM 3D Objects', 8)
    inf = Inference(os.path.join(ctx.src, 'checkpoints', 'hf', 'pipeline.yaml'), compile=False)
    ctx.stage('making it', 20)
    out = inf(rgba[..., :3], mask, seed=ctx.seed)
    splat = os.path.join(ctx.work, 'sam3d.ply')
    out['gs'].save_ply(splat)
    res = {'splat': splat}
    if out.get('glb') is not None:
        res['mesh'] = out['glb']
    return res


# ------------------------------------------------------------------ people

def sam3dbody(ctx):
    """SAM 3D Body (as its demo.py): ViTDet finds each person, MoGe-2 the camera, then each person's posed body mesh
    (Meta's MHR body), all of them in one model, turned upright (the camera looks down +z with y down)."""
    _syspath(ctx)
    import numpy as np
    import trimesh
    from sam_3d_body import load_sam_3d_body, SAM3DBodyEstimator
    from tools.build_detector import HumanDetector
    from tools.build_fov_estimator import FOVEstimator
    dev = _dev()
    snap = ctx.snapshot('facebook/sam-3d-body-dinov3')
    ctx.stage('loading SAM 3D Body', 6)
    model, cfg = load_sam_3d_body(os.path.join(snap, 'model.ckpt'), device=dev, mhr_path=os.path.join(snap, 'assets', 'mhr_model.pt'))
    det = HumanDetector(name='vitdet', device=dev)
    fov = FOVEstimator(name='moge2', device=dev, path=ctx.snapshot('Ruicheng/moge-2-vitl-normal'))
    est = SAM3DBodyEstimator(sam_3d_body_model=model, model_cfg=cfg, human_detector=det, human_segmentor=None, fov_estimator=fov)
    ctx.stage('finding the people and their pose', 30)
    people = est.process_one_image(ctx.photo, bbox_thr=0.5, use_mask=False)
    if not people:
        raise ctx.Fail('SAM 3D Body found no one in the photo')
    flip = np.array([1.0, -1.0, -1.0])
    meshes = [trimesh.Trimesh((np.asarray(p['pred_vertices']) + np.asarray(p['pred_cam_t'])) * flip, est.faces, process=False) for p in people]
    return {'mesh': trimesh.util.concatenate(meshes), 'shapeOnly': True, 'note': '%d %s' % (len(meshes), 'person' if len(meshes) == 1 else 'people')}


# ------------------------------------------------------------------ places and worlds (splats)

def triposplat(ctx):
    """TripoSplat (as its run_example.py): one photo → Gaussians (its own BiRefNet cut-out); how many from Detail."""
    _syspath(ctx)
    from triposplat import TripoSplatPipeline
    s = ctx.snapshot('VAST-AI/TripoSplat')
    ctx.stage('loading TripoSplat', 6)
    pipe = TripoSplatPipeline(ckpt_path=os.path.join(s, 'diffusion_models', 'triposplat_fp16.safetensors'), decoder_path=os.path.join(s, 'vae', 'triposplat_vae_decoder_fp16.safetensors'),
                              dinov3_path=os.path.join(s, 'clip_vision', 'dino_v3_vit_h.safetensors'), flux2_vae_encoder_path=os.path.join(s, 'vae', 'flux2-vae.safetensors'),
                              rmbg_path=os.path.join(s, 'background_removal', 'birefnet.safetensors'), device=_dev())
    n = GAUSSIANS.get(ctx.detail, 131072)
    ctx.stage('making %d splats' % n, 15)
    g, prepared = pipe.run(ctx.photo, seed=ctx.seed, num_gaussians=n)
    prepared.save(os.path.join(ctx.d, 'photo-cut.png'))
    splat = os.path.join(ctx.work, 'triposplat.ply')
    g.save_ply(splat)
    return {'splat': splat}


def _inputs(ctx):
    """The photos (or a video) in a folder of their own, as the world models want them."""
    d = os.path.join(ctx.work, 'input')
    os.makedirs(d, exist_ok=True)
    for i, p in enumerate(ctx.photos or [ctx.photo]):
        shutil.copyfile(p, os.path.join(d, 'view-%02d%s' % (i, os.path.splitext(p)[1] or '.png')))
    return d


def hyworld2(ctx):
    """HY-World 2.0's WorldMirror 2.0 (its own Python API, DOCUMENTATION.md): your photos (or a picture from your
    words) → a world as Gaussians and points, in bf16 (its enable_bf16) with a smaller inference size on a small
    card. Its panorama and world generation (HY-Pano 2.0 on HunyuanImage-3 or Qwen-Image-Edit) are far beyond 12 GB
    and aren't got."""
    _syspath(ctx)
    from hyworld2.worldrecon.pipeline import WorldMirrorPipeline
    ctx.stage('loading WorldMirror 2.0', 6)
    pipe = WorldMirrorPipeline.from_pretrained(ctx.snapshot('tencent/HY-World-2.0'), subfolder='HY-WorldMirror-2.0', enable_bf16=True)
    size = WORLD.get(ctx.detail, 952) if ctx.vram >= 16 else min(WORLD.get(ctx.detail, 700), 700)
    ctx.stage('making the world', 20)
    out = pipe(_inputs(ctx), output_path=os.path.join(ctx.work, 'world'), target_size=size, save_depth=False, save_normal=False, save_gs=True,
               save_camera=True, save_points=True)
    if not out:
        raise ctx.Fail('HY-World skipped it')
    return {'splat': _path(out, 'gaussians.ply')}


def lyra2(ctx):
    """Lyra 2.0 (its own two steps, README): a zoom-in, zoom-out video of your photo's world
    (lyra2_zoomgs_inference), then that video turned into Gaussians (vipe_da3_gs_recon). On a card under 40 GB its
    own --offload, --offload_when_prompt and --offload_da3_diffusion, and --use_dmd (its 4-step distilled sampler)."""
    root = ctx.src
    env = {'PYTHONPATH': root, 'PYTORCH_CUDA_ALLOC_CONF': 'expandable_segments:True'}
    snap = ctx.snapshot('nvidia/Lyra-2.0')
    vid = os.path.join(ctx.work, 'zoomgs')
    words = (ctx.words or ctx.ask.get('title') or 'a place').strip()
    args = [ctx.py, '-m', 'lyra_2._src.inference.lyra2_zoomgs_inference', '--input_image_path', _inputs(ctx), '--sample_id', '0', '--experiment', 'lyra2',
            '--checkpoint_dir', os.path.join(snap, 'checkpoints', 'model'), '--prompt', words, '--output_path', vid, '--num_frames_zoom_in', '81',
            '--num_frames_zoom_out', '241', '--zoom_in_strength', '0.5', '--zoom_out_strength', '1.5', '--seed', str(ctx.seed),
            '--da3_model_name', ctx.snapshot('depth-anything/DA3NESTED-GIANT-LARGE-1.1'), '--da3_model_path_custom', os.path.join(snap, 'checkpoints', 'recon', 'model.pt')]
    if _low(ctx, 40):
        args += ['--offload', '--offload_when_prompt', '--offload_da3_diffusion', '--use_dmd']
    ctx.stage('making the video of its world (Lyra 2.0)', 10)
    ctx.cli(args, cwd=root, env=env)
    video = _path(vid, 'videos', '0.mp4')
    rec = os.path.join(ctx.work, 'recon')
    ctx.stage('turning the video into splats', 60)
    ctx.cli([ctx.py, '-m', 'lyra_2._src.inference.vipe_da3_gs_recon', '--input_video_path', video, '--output_dir', rec], cwd=root, env=env)
    return {'splat': _path(rec, 'reconstructed_scene.ply')}


def sharp(ctx):
    """SHARP (Apple's own `sharp predict`): one photo → a scene as Gaussians, on the Nvidia card (or the CPU)."""
    src = _inputs(ctx)
    out = os.path.join(ctx.work, 'sharp')
    ckpt = _find(ctx.snapshot('apple/Sharp'), '**/*.pt')
    if not ckpt:
        raise ctx.Fail('SHARP\'s checkpoint (.pt) isn\'t in what was got')
    args = [os.path.join(ctx.bin, 'sharp'), 'predict', '-i', src, '-o', out, '-c', ckpt, '--device', 'cuda' if ctx.vram > 0 else 'cpu']
    ctx.stage('making the scene (SHARP)', 15)
    ctx.cli(args, cwd=ctx.src)
    got = _find(out, '*.ply')
    if not got:
        raise ctx.Fail('SHARP made nothing')
    return {'splat': got}


RUNNERS = {'hunyuan3d21': hunyuan3d21, 'trellis2': trellis2, 'trellis': trellis, 'spar3d': spar3d, 'pixal3d': pixal3d, 'step1x3d': step1x3d,
           'triposg': triposg, 'direct3ds2': direct3ds2, 'hi3dgen': hi3dgen, 'hi3d': hi3d, 'ultrashape': ultrashape, 'sam3dobjects': sam3dobjects,
           'sam3dbody': sam3dbody, 'triposplat': triposplat, 'hyworld2': hyworld2, 'lyra2': lyra2, 'sharp': sharp}


def make(ctx):
    fn = RUNNERS.get(ctx.m.get('runner'))
    if fn is None:
        raise ctx.Fail('no runner for %s' % ctx.m.get('id'))
    return finish(ctx, fn(ctx) or {})


def finish(ctx, res):
    """What the runner made, kept the same way for every model: model.glb (or coarse.glb), splat.ply, result.json's
    facts. A mesh file of another kind is turned into a .glb by trimesh (in every runtime)."""
    mesh = res.get('mesh')
    if mesh is not None:
        if isinstance(mesh, str):
            if mesh.lower().endswith('.glb'):
                shutil.move(mesh, ctx.out)
            else:
                import trimesh
                trimesh.load(mesh, force='mesh').export(ctx.out)
        else:
            mesh.export(ctx.out)
    splat = res.get('splat')
    if splat:
        shutil.copyfile(splat, os.path.join(ctx.d, 'splat.ply'))
    if mesh is None and not splat:
        raise ctx.Fail('%s made nothing' % ctx.m.get('name'))
    return {'mesh': mesh is not None, 'splat': bool(splat), 'shapeOnly': bool(res.get('shapeOnly')), 'note': res.get('note') or ''}
