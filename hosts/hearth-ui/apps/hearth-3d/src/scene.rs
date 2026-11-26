//! The scene, as Blender (GPL-2+) has it: Hearth 3D runs Blender in the background (`blender -b FILE --python-expr`)
//! to read a .blend (every object, its place, turn and size, its changes on top (modifiers), its mesh with them
//! applied, its colour, the collections, the animation's frames and keys) and to change it (the edits, then Blender
//! saves the file the way it always does, keeping the last one as FILE.blend1). Import, export, Send to Godot (glTF),
//! Send to Hearth CAD (STL) and rendering a frame are Blender's own operators. Hearth 3D draws what comes back
//! (view3d.rs); nothing about the .blend is guessed.

use serde_json::Value;

#[derive(Debug, Clone, Default)]
pub struct Mod {
    pub name: String,
    pub kind: String,
    pub words: String,
    pub show: bool,
}

#[derive(Debug, Clone, Default)]
pub struct Obj {
    pub name: String,
    pub kind: String,
    pub parent: String,
    pub hidden: bool,
    pub render: bool,
    pub loc: [f32; 3],
    pub rot: [f32; 3],
    pub scale: [f32; 3],
    pub mods: Vec<Mod>,
    pub color: [f32; 3],
    pub faces: u64,
    pub points: u64,
    /// its mesh (the changes on top applied), in its own space
    pub verts: Vec<[f32; 3]>,
    pub tris: Vec<[u32; 3]>,
    /// where it is now (column-major 4 × 4), and at each frame of the animation (when it moves)
    pub mat: [f32; 16],
    pub anim: Vec<[f32; 16]>,
    pub keys: Vec<i64>,
}

#[derive(Debug, Clone, Default)]
pub struct Coll {
    pub name: String,
    pub children: Vec<Coll>,
    pub objects: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct Scene {
    pub objects: Vec<Obj>,
    pub root: Coll,
    pub frame: i64,
    pub start: i64,
    pub end: i64,
    pub fps: i64,
    pub action: String,
    pub world: [f32; 3],
}

fn f3(v: &Value, d: [f32; 3]) -> [f32; 3] {
    match v.as_array() {
        Some(a) if a.len() >= 3 => [a[0].as_f64().unwrap_or(0.0) as f32, a[1].as_f64().unwrap_or(0.0) as f32, a[2].as_f64().unwrap_or(0.0) as f32],
        _ => d,
    }
}
fn m16(v: &Value) -> Option<[f32; 16]> {
    let a = v.as_array()?;
    if a.len() != 16 {
        return None;
    }
    let mut m = [0.0f32; 16];
    for (i, x) in a.iter().enumerate() {
        m[i] = x.as_f64().unwrap_or(0.0) as f32;
    }
    Some(m)
}
pub const IDENTITY: [f32; 16] = [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0];

fn coll(v: &Value) -> Coll {
    Coll {
        name: v.get("name").and_then(Value::as_str).unwrap_or("").to_string(),
        children: v.get("children").and_then(Value::as_array).map(|a| a.iter().map(coll).collect()).unwrap_or_default(),
        objects: v.get("objects").and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_str).map(str::to_string).collect()).unwrap_or_default(),
    }
}

/// What the export script wrote.
pub fn parse(j: &Value) -> Scene {
    let objects = j
        .get("objects")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .map(|o| {
                    let verts: Vec<[f32; 3]> = o.get("verts").and_then(Value::as_array).map(|v| v.chunks(3).filter(|c| c.len() == 3).map(|c| [c[0].as_f64().unwrap_or(0.0) as f32, c[1].as_f64().unwrap_or(0.0) as f32, c[2].as_f64().unwrap_or(0.0) as f32]).collect()).unwrap_or_default();
                    let n = verts.len() as u32;
                    let tris: Vec<[u32; 3]> = o.get("tris").and_then(Value::as_array).map(|v| v.chunks(3).filter(|c| c.len() == 3).map(|c| [c[0].as_u64().unwrap_or(0) as u32, c[1].as_u64().unwrap_or(0) as u32, c[2].as_u64().unwrap_or(0) as u32]).filter(|t| t.iter().all(|i| *i < n)).collect()).unwrap_or_default();
                    Obj {
                        name: o.get("name").and_then(Value::as_str).unwrap_or("").to_string(),
                        kind: o.get("type").and_then(Value::as_str).unwrap_or("").to_string(),
                        parent: o.get("parent").and_then(Value::as_str).unwrap_or("").to_string(),
                        hidden: o.get("hide").and_then(Value::as_bool).unwrap_or(false),
                        render: o.get("render").and_then(Value::as_bool).unwrap_or(true),
                        loc: f3(&o["loc"], [0.0; 3]),
                        rot: f3(&o["rot"], [0.0; 3]),
                        scale: f3(&o["scale"], [1.0; 3]),
                        mods: o.get("mods").and_then(Value::as_array).map(|a| a.iter().map(|m| Mod { name: m["name"].as_str().unwrap_or("").into(), kind: m["type"].as_str().unwrap_or("").into(), words: m["words"].as_str().unwrap_or("").into(), show: m["show"].as_bool().unwrap_or(true) }).collect()).unwrap_or_default(),
                        color: f3(&o["color"], [0.6, 0.6, 0.6]),
                        faces: o.get("faces").and_then(Value::as_u64).unwrap_or(0),
                        points: o.get("points").and_then(Value::as_u64).unwrap_or(0),
                        verts,
                        tris,
                        mat: m16(&o["mat"]).unwrap_or(IDENTITY),
                        anim: o.get("anim").and_then(Value::as_array).map(|a| a.iter().filter_map(m16).collect()).unwrap_or_default(),
                        keys: o.get("keys").and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_i64).collect()).unwrap_or_default(),
                    }
                })
                .collect()
        })
        .unwrap_or_default();
    Scene {
        objects,
        root: coll(&j["collections"]),
        frame: j["frame"].as_i64().unwrap_or(1),
        start: j["start"].as_i64().unwrap_or(1),
        end: j["end"].as_i64().unwrap_or(250),
        fps: j["fps"].as_i64().unwrap_or(24).max(1),
        action: j["action"].as_str().unwrap_or("").to_string(),
        world: f3(&j["world"], [0.05, 0.05, 0.05]),
    }
}

/// A Python string literal (for the edits).
pub fn py(s: &str) -> String {
    format!("'{}'", s.replace('\\', "\\\\").replace('\'', "\\'").replace('\n', " "))
}

/// The script Blender runs: the edits (Python, using `bpy` and `ob(name)`), saving when there were any, then the
/// scene written as JSON to the path after `--`.
pub fn script(edits: &str, save: bool) -> String {
    format!(
        r#"import bpy, json, sys, math
out = sys.argv[sys.argv.index('--') + 1]
def ob(n):
    return bpy.data.objects.get(n)
{edits}
if {save}:
    bpy.ops.wm.save_mainfile()
sc = bpy.context.scene
dg = bpy.context.evaluated_depsgraph_get()
def tree(c):
    return {{'name': c.name, 'children': [tree(x) for x in c.children], 'objects': [o.name for o in c.objects]}}
def words(m):
    t = m.type
    try:
        if t == 'MIRROR':
            return ''.join(a for a, on in zip('XYZ', m.use_axis) if on)
        if t == 'BEVEL':
            return '%g cm · %d steps' % (round(m.width * 100, 1), m.segments)
        if t == 'SUBSURF':
            return '%d levels' % m.levels
        if t == 'ARRAY':
            return '%d copies' % m.count
        if t == 'SOLIDIFY':
            return '%g cm' % round(m.thickness * 100, 1)
        if t == 'NODES':
            for k in m.keys():
                v = m[k]
                if isinstance(v, float) and 0 < v < 3.2:
                    return '%d°' % round(math.degrees(v))
    except Exception:
        pass
    return ''
def colour(o):
    try:
        for s in o.material_slots:
            if s.material:
                n = s.material.node_tree.nodes.get('Principled BSDF') if s.material.use_nodes else None
                c = n.inputs['Base Color'].default_value if n else s.material.diffuse_color
                return [c[0], c[1], c[2]]
    except Exception:
        pass
    if o.type == 'LIGHT':
        return list(o.data.color)
    return [0.48, 0.54, 0.42]
def flat(m):
    return [m[r][c] for c in range(4) for r in range(4)]
budget = 60000
objs = []
for o in sc.objects:
    d = {{'name': o.name, 'type': o.type, 'parent': o.parent.name if o.parent else '', 'hide': o.hide_get(), 'render': not o.hide_render,
         'loc': list(o.location), 'rot': [math.degrees(a) for a in o.rotation_euler], 'scale': list(o.scale),
         'mods': [{{'name': m.name, 'type': m.type, 'show': m.show_viewport, 'words': words(m)}} for m in getattr(o, 'modifiers', [])],
         'color': colour(o), 'mat': flat(o.matrix_world)}}
    if o.type == 'MESH':
        d['faces'] = len(o.data.polygons)
        d['points'] = len(o.data.vertices)
        if budget > 0:
            e = o.evaluated_get(dg)
            me = e.to_mesh()
            me.calc_loop_triangles()
            tris = [list(t.vertices) for t in me.loop_triangles][:budget]
            budget -= len(tris)
            d['verts'] = [round(c, 4) for v in me.vertices for c in v.co]
            d['tris'] = [i for t in tris for i in t]
            e.to_mesh_clear()
    ad = o.animation_data
    if ad and ad.action:
        d['keys'] = sorted({{int(k.co[0]) for fc in ad.action.fcurves for k in fc.keyframe_points}})
    objs.append(d)
moving = [d for d, o in zip(objs, sc.objects) if o.animation_data and o.animation_data.action or (o.parent and o.parent.animation_data)]
here = sc.frame_current
if moving and sc.frame_end - sc.frame_start <= 400:
    for f in range(sc.frame_start, sc.frame_end + 1):
        sc.frame_set(f)
        for d in moving:
            d.setdefault('anim', []).append(flat(bpy.data.objects[d['name']].matrix_world))
    sc.frame_set(here)
act = next((o.animation_data.action.name for o in sc.objects if o.animation_data and o.animation_data.action), '')
w = sc.world.color if sc.world else (0.05, 0.05, 0.05)
json.dump({{'collections': tree(sc.collection), 'frame': here, 'start': sc.frame_start, 'end': sc.frame_end, 'fps': sc.render.fps,
           'action': act, 'world': [w[0], w[1], w[2]], 'objects': objs}}, open(out, 'w'))
"#,
        edits = edits,
        save = if save { "True" } else { "False" }
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reads_what_blender_wrote() {
        let j = serde_json::json!({"collections": {"name": "Scene Collection", "children": [{"name": "Mech", "children": [], "objects": ["Body"]}], "objects": []}, "frame": 32, "start": 1, "end": 96, "fps": 24, "action": "Walk cycle",
            "objects": [{"name": "Body", "type": "MESH", "loc": [0, 0, 2.4], "rot": [0, 0, 35], "scale": [1, 1, 1], "faces": 2412, "points": 4880, "verts": [0, 0, 0, 1, 0, 0, 0, 1, 0], "tris": [0, 1, 2, 0, 1, 9], "mods": [{"name": "Mirror", "type": "MIRROR", "words": "X", "show": true}], "keys": [0, 8]}]});
        let s = parse(&j);
        assert_eq!(s.objects[0].tris.len(), 1, "a triangle past the points is left out");
        assert_eq!(s.objects[0].rot[2], 35.0);
        assert_eq!(s.root.children[0].objects, ["Body"]);
        assert_eq!((s.frame, s.end, s.fps), (32, 96, 24));
        assert_eq!(py("it's"), "'it\\'s'");
        assert!(script("ob('Body').location.z = 2.0", true).contains("if True:\n    bpy.ops.wm.save_mainfile()"));
    }
}
