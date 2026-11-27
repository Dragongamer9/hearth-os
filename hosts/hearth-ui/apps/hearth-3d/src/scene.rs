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

// ------------------------------------------------------------------ editing a mesh itself (Tab, Sculpt, Paint)

/// One object's own mesh, as Blender keeps it (before the changes on top): its points, edges and faces, and the
/// colours painted on its points (Blender's colour attribute "Hearth paint"). Edit, Sculpt and Paint change these
/// here, in Rust, as you work; Blender writes them into the .blend when you're done (apply_code).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EditMesh {
    pub obj: String,
    pub verts: Vec<[f32; 3]>,
    pub faces: Vec<Vec<u32>>,
    pub edges: Vec<[u32; 2]>,
    pub colors: Vec<[f32; 3]>,
}

/// Blender's script that writes an object's own mesh as JSON (args after `--`: the JSON file, the object).
pub const MESH_DUMP: &str = r#"import bpy, json, sys
# HEARTH_MESH_DUMP
a = sys.argv[sys.argv.index('--') + 1:]
o = bpy.data.objects.get(a[1])
me = o.data
ca = me.color_attributes.get('Hearth paint') if hasattr(me, 'color_attributes') else None
cols = [round(c, 4) for d in ca.data for c in tuple(d.color)[:3]] if ca is not None and ca.domain == 'POINT' else []
json.dump({'obj': o.name, 'verts': [round(c, 5) for v in me.vertices for c in v.co], 'faces': [list(p.vertices) for p in me.polygons],
           'edges': [list(e.vertices) for e in me.edges], 'colors': cols}, open(a[0], 'w'))
"#;

impl EditMesh {
    pub fn parse(j: &Value) -> EditMesh {
        let flat = |k: &str| -> Vec<[f32; 3]> { j.get(k).and_then(Value::as_array).map(|v| v.chunks(3).filter(|c| c.len() == 3).map(|c| [c[0].as_f64().unwrap_or(0.0) as f32, c[1].as_f64().unwrap_or(0.0) as f32, c[2].as_f64().unwrap_or(0.0) as f32]).collect()).unwrap_or_default() };
        let verts = flat("verts");
        let n = verts.len() as u32;
        let faces: Vec<Vec<u32>> = j.get("faces").and_then(Value::as_array).map(|a| a.iter().filter_map(|f| f.as_array().map(|f| f.iter().filter_map(|x| x.as_u64()).map(|x| x as u32).collect::<Vec<u32>>())).filter(|f: &Vec<u32>| f.len() >= 3 && f.iter().all(|i| *i < n)).collect()).unwrap_or_default();
        let mut edges: Vec<[u32; 2]> = j.get("edges").and_then(Value::as_array).map(|a| a.iter().filter_map(|e| e.as_array()).filter(|e| e.len() == 2).map(|e| [e[0].as_u64().unwrap_or(0) as u32, e[1].as_u64().unwrap_or(0) as u32]).filter(|e| e[0] < n && e[1] < n).collect()).unwrap_or_default();
        if edges.is_empty() {
            // from the faces (a stand-in, or an old file)
            for f in &faces {
                for i in 0..f.len() {
                    let (a, b) = (f[i], f[(i + 1) % f.len()]);
                    let e = [a.min(b), a.max(b)];
                    if !edges.contains(&e) {
                        edges.push(e);
                    }
                }
            }
        }
        let mut colors = flat("colors");
        if colors.len() != verts.len() {
            colors = Vec::new();
        }
        EditMesh { obj: j["obj"].as_str().unwrap_or("").to_string(), verts, faces, edges, colors }
    }
    /// The faces as triangles (a fan each), with the face each came from.
    pub fn tris(&self) -> Vec<([u32; 3], usize)> {
        let mut v = Vec::new();
        for (fi, f) in self.faces.iter().enumerate() {
            for k in 1..f.len().saturating_sub(1) {
                v.push(([f[0], f[k], f[k + 1]], fi));
            }
        }
        v
    }
    /// Each point's normal (the faces round it), in the mesh's own space.
    pub fn normals(&self) -> Vec<[f32; 3]> {
        let mut n = vec![[0.0f32; 3]; self.verts.len()];
        for (t, _) in self.tris() {
            let (a, b, c) = (self.verts[t[0] as usize], self.verts[t[1] as usize], self.verts[t[2] as usize]);
            let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
            let w = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
            let cr = [u[1] * w[2] - u[2] * w[1], u[2] * w[0] - u[0] * w[2], u[0] * w[1] - u[1] * w[0]];
            for i in t {
                let x = &mut n[i as usize];
                x[0] += cr[0];
                x[1] += cr[1];
                x[2] += cr[2];
            }
        }
        for x in n.iter_mut() {
            let l = (x[0] * x[0] + x[1] * x[1] + x[2] * x[2]).sqrt().max(1e-9);
            *x = [x[0] / l, x[1] / l, x[2] / l];
        }
        n
    }
    /// Each point's neighbours (along the edges).
    pub fn neighbours(&self) -> Vec<Vec<u32>> {
        let mut nb = vec![Vec::new(); self.verts.len()];
        for e in &self.edges {
            nb[e[0] as usize].push(e[1]);
            nb[e[1] as usize].push(e[0]);
        }
        nb
    }
    /// The JSON Blender's apply_code reads: the points (and the paint when there is any).
    pub fn to_json(&self) -> Value {
        let mut j = serde_json::json!({"obj": self.obj, "co": self.verts.iter().flat_map(|v| v.iter().copied()).collect::<Vec<f32>>()});
        if !self.colors.is_empty() {
            j["colors"] = serde_json::json!(self.colors.iter().flat_map(|v| v.iter().copied()).collect::<Vec<f32>>());
        }
        j
    }
}

/// The edit Blender makes when you're done: the points and paint from `file` (EditMesh::to_json) into the object's
/// mesh; the paint becomes Blender's colour attribute "Hearth paint", shown by the material when nothing else colours
/// it. `then`: Blender's own mesh tools on the picked points afterwards ("delete": the points go, "extrude": the
/// picked faces pulled out along their normal by `amount`), as bmesh does them.
pub fn apply_code(file: &str, then: &str, picked: &[u32], amount: f32) -> String {
    format!(
        r#"import bmesh
j = json.load(open({f}))
o = ob(j['obj'])
me = o.data
if len(j.get('co', [])) == len(me.vertices) * 3:
    me.vertices.foreach_set('co', j['co'])
cs = j.get('colors') or []
if len(cs) == len(me.vertices) * 3:
    ca = me.color_attributes.get('Hearth paint') or me.color_attributes.new('Hearth paint', 'FLOAT_COLOR', 'POINT')
    rgba = []
    for i in range(len(me.vertices)):
        rgba += cs[i * 3:i * 3 + 3] + [1.0]
    ca.data.foreach_set('color', rgba)
    try:
        me.color_attributes.active_color = ca
    except Exception:
        pass
    for s in o.material_slots:
        m = s.material
        if m and m.use_nodes:
            n = m.node_tree.nodes.get('Principled BSDF')
            if n and not n.inputs['Base Color'].is_linked:
                a = m.node_tree.nodes.new('ShaderNodeVertexColor')
                a.layer_name = 'Hearth paint'
                m.node_tree.links.new(a.outputs['Color'], n.inputs['Base Color'])
me.update()
then, picked = {then}, {picked:?}
if then and picked:
    bm = bmesh.new()
    bm.from_mesh(me)
    bm.verts.ensure_lookup_table()
    vs = [bm.verts[i] for i in picked if i < len(bm.verts)]
    if then == 'delete':
        bmesh.ops.delete(bm, geom=vs, context='VERTS')
    elif then == 'extrude':
        fs = [f for f in bm.faces if all(v in vs for v in f.verts)]
        if fs:
            r = bmesh.ops.extrude_face_region(bm, geom=fs)
            nv = [g for g in r['geom'] if isinstance(g, bmesh.types.BMVert)]
            nrm = sum((f.normal for f in fs), fs[0].normal * 0) / len(fs)
            bmesh.ops.translate(bm, verts=nv, vec=nrm * {amount})
    bm.to_mesh(me)
    bm.free()
    me.update()
"#,
        f = py(file),
        then = py(then),
        picked = picked,
        amount = amount
    )
}

#[cfg(test)]
mod edit_tests {
    use super::*;
    #[test]
    fn an_objects_own_mesh() {
        let j = serde_json::json!({"obj": "Body", "verts": [0, 0, 0, 1, 0, 0, 1, 1, 0, 0, 1, 0], "faces": [[0, 1, 2, 3]], "edges": []});
        let m = EditMesh::parse(&j);
        assert_eq!(m.verts.len(), 4);
        assert_eq!(m.edges.len(), 4, "edges from the faces");
        assert_eq!(m.tris().len(), 2);
        assert!((m.normals()[0][2] - 1.0).abs() < 1e-5);
        assert_eq!(m.neighbours()[0].len(), 2);
        assert_eq!(m.to_json()["co"].as_array().unwrap().len(), 12);
        let c = apply_code("/tmp/x.json", "extrude", &[0, 1, 2, 3], 0.3);
        assert!(c.contains("bmesh.ops.extrude_face_region") && c.contains("[0, 1, 2, 3]"));
    }
}
