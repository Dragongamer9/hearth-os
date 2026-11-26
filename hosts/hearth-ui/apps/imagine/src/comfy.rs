//! ComfyUI (GPL-3, github.com/comfyanonymous/ComfyUI) through its own local HTTP API and WebSocket, on the Nvidia PC
//! where the Home server helper runs it (hearth-comfyui.service, port 8188, on 127.0.0.1 and the PC's Tailscale address
//! only): /system_stats (the graphics card), /object_info (the models it has), /prompt (a graph to run), /queue,
//! /history, /view (a picture), /upload/image, and /ws (which step each picture is on, and when it's done).
//!
//! The graph Hearth Imagine sends is ComfyUI's own API format, built from Type and make's choices: Load model →
//! Describe it / Leave out → Empty picture → Make (KSampler) → Turn into pictures (VAE decode) → Save, with Bigger's
//! upscaler and More like this's picture-to-picture when asked.

use hearth_oss::http::{self, Req};
use serde_json::{json, Value};

/// The choices that make one picture's graph.
#[derive(Debug, Clone, PartialEq)]
pub struct Ask {
    pub prompt: String,
    pub leave_out: String,
    pub look: String,
    pub w: u32,
    pub h: u32,
    pub model: String,
    pub steps: u32,
    pub seed: u64,
    pub cfg: f32,
    pub upscale: String,
    /// picture to picture: the uploaded picture's name in ComfyUI's input folder, and how much changes (0..1)
    pub from: Option<(String, f32)>,
}

/// The looks (Type and make › Look): the words each one adds to what you asked for, and what its graph node says.
pub const LOOKS: &[(&str, &str, &str)] = &[
    ("Film", "soft film grain, warm tones, cinematic light", "soft grain · warm"),
    ("Painting", "oil painting, visible brush strokes, rich colour", "brush strokes · rich"),
    ("Game art", "stylised game concept art, clean shapes, dramatic light", "concept art · bold"),
    ("Pixel", "pixel art, 16-bit, crisp pixels, limited palette", "16-bit · crisp"),
    ("Wallpaper", "wallpaper, wide composition, clean, high detail", "clean · detailed"),
    ("Sketch", "pencil sketch, hand drawn lines, paper texture", "pencil · paper"),
];
pub fn look_words(l: &str) -> (&'static str, &'static str) {
    LOOKS.iter().find(|x| x.0 == l).map(|x| (x.1, x.2)).unwrap_or(("", ""))
}

pub const SIZES: &[(&str, u32, u32)] = &[("16:9", 1344, 768), ("9:16", 768, 1344), ("1:1", 1024, 1024), ("4:3", 1152, 896)];

/// ComfyUI's API graph for one picture. Node ids are fixed (3 Make, 4 Load model, 5 Empty picture, 6 Describe it,
/// 7 Leave out, 8 Turn into pictures, 9 Save, 10/11 Bigger, 12/13 the picture to start from).
pub fn graph(a: &Ask, prefix: &str) -> Value {
    let (lw, _) = look_words(&a.look);
    let positive = if lw.is_empty() { a.prompt.trim().to_string() } else { format!("{}, {lw}", a.prompt.trim().trim_end_matches(',')) };
    let mut g = json!({
        "4": {"class_type": "CheckpointLoaderSimple", "inputs": {"ckpt_name": a.model}},
        "6": {"class_type": "CLIPTextEncode", "inputs": {"text": positive, "clip": ["4", 1]}},
        "7": {"class_type": "CLIPTextEncode", "inputs": {"text": a.leave_out, "clip": ["4", 1]}},
        "5": {"class_type": "EmptyLatentImage", "inputs": {"width": a.w, "height": a.h, "batch_size": 1}},
        "3": {"class_type": "KSampler", "inputs": {"seed": a.seed, "steps": a.steps, "cfg": a.cfg, "sampler_name": "euler", "scheduler": "normal", "denoise": 1.0,
              "model": ["4", 0], "positive": ["6", 0], "negative": ["7", 0], "latent_image": ["5", 0]}},
        "8": {"class_type": "VAEDecode", "inputs": {"samples": ["3", 0], "vae": ["4", 2]}},
        "9": {"class_type": "SaveImage", "inputs": {"filename_prefix": prefix, "images": ["8", 0]}},
    });
    if let Some((img, change)) = &a.from {
        g["12"] = json!({"class_type": "LoadImage", "inputs": {"image": img}});
        g["13"] = json!({"class_type": "VAEEncode", "inputs": {"pixels": ["12", 0], "vae": ["4", 2]}});
        g["3"]["inputs"]["latent_image"] = json!(["13", 0]);
        g["3"]["inputs"]["denoise"] = json!(change);
        if let Some(o) = g.as_object_mut() {
            o.remove("5");
        }
    }
    if !a.upscale.is_empty() {
        g["10"] = json!({"class_type": "UpscaleModelLoader", "inputs": {"model_name": a.upscale}});
        g["11"] = json!({"class_type": "ImageUpscaleWithModel", "inputs": {"upscale_model": ["10", 0], "image": ["8", 0]}});
        g["9"]["inputs"]["images"] = json!(["11", 0]);
    }
    g
}

/// Bigger: one picture already made, through the upscaler, saved again.
pub fn bigger(img: &str, upscaler: &str, prefix: &str) -> Value {
    json!({
        "12": {"class_type": "LoadImage", "inputs": {"image": img}},
        "10": {"class_type": "UpscaleModelLoader", "inputs": {"model_name": upscaler}},
        "11": {"class_type": "ImageUpscaleWithModel", "inputs": {"upscale_model": ["10", 0], "image": ["12", 0]}},
        "9": {"class_type": "SaveImage", "inputs": {"filename_prefix": prefix, "images": ["11", 0]}},
    })
}

pub fn get(base: &str, path: &str) -> Result<Value, String> {
    let r = http::send(&Req::get(&format!("{base}{path}")).timeout(8))?;
    if !r.ok() {
        return Err(format!("ComfyUI answered {}", r.status));
    }
    r.json().ok_or_else(|| "ComfyUI's answer couldn't be read".into())
}

pub fn queue_prompt(base: &str, g: &Value, client: &str, title: &str) -> Result<String, String> {
    let body = json!({"prompt": g, "client_id": client, "extra_data": {"hearth_title": title}});
    let r = http::send(&Req::new("POST", &format!("{base}/prompt")).json(&body).timeout(15))?;
    let j = r.json().unwrap_or(Value::Null);
    if !r.ok() {
        let why = j.pointer("/error/message").and_then(Value::as_str).unwrap_or("").to_string();
        let more = j.get("node_errors").and_then(Value::as_object).and_then(|o| o.values().next()).and_then(|e| e.pointer("/errors/0/details")).and_then(Value::as_str).unwrap_or("").to_string();
        return Err(format!("ComfyUI said no: {why} {more}").trim().to_string());
    }
    j.get("prompt_id").and_then(Value::as_str).map(str::to_string).ok_or_else(|| "ComfyUI gave no id".into())
}

pub fn view(base: &str, img: &Value) -> Result<Vec<u8>, String> {
    let q = format!(
        "/view?filename={}&subfolder={}&type={}",
        http::enc(img.get("filename").and_then(Value::as_str).unwrap_or("")),
        http::enc(img.get("subfolder").and_then(Value::as_str).unwrap_or("")),
        http::enc(img.get("type").and_then(Value::as_str).unwrap_or("output"))
    );
    let r = http::send(&Req::get(&format!("{base}{q}")).timeout(30))?;
    if r.ok() {
        Ok(r.body)
    } else {
        Err(format!("ComfyUI answered {}", r.status))
    }
}

pub fn upload(base: &str, name: &str, png: &[u8]) -> Result<String, String> {
    let (body, ct) = http::multipart(&[("overwrite", "true")], ("image", name, "image/png", png));
    let r = http::send(&Req::new("POST", &format!("{base}/upload/image")).body(body, &ct).timeout(30))?;
    r.json().and_then(|j| j.get("name").and_then(Value::as_str).map(str::to_string)).ok_or_else(|| format!("ComfyUI didn't take the picture ({})", r.status))
}

/// The list ComfyUI offers for one input of one node (its checkpoints, its upscalers).
pub fn choices(info: &Value, node: &str, input: &str) -> Vec<String> {
    let v = info.pointer(&format!("/{node}/input/required/{input}/0"));
    let v = match v {
        Some(Value::Array(a)) => Some(a.clone()),
        // newer ComfyUI: ["COMBO", {"options": [...]}]
        _ => info.pointer(&format!("/{node}/input/required/{input}/1/options")).and_then(Value::as_array).cloned(),
    };
    v.unwrap_or_default().iter().filter_map(|x| x.as_str().map(str::to_string)).collect()
}

/// A model's file name in words: "hearth-xl-1.0.safetensors" → "Hearth XL 1.0".
pub fn pretty(file: &str) -> String {
    let stem = file.rsplit('/').next().unwrap_or(file);
    let stem = stem.rsplit_once('.').filter(|(_, e)| matches!(*e, "safetensors" | "ckpt" | "pth" | "pt" | "gguf" | "bin")).map(|(s, _)| s).unwrap_or(stem);
    // a name with spaces is kept as it was written ("Fast 1-step"); kebab or snake case is made into words
    if stem.contains(' ') {
        let mut c = stem.chars();
        return match c.next() {
            Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
            None => String::new(),
        };
    }
    stem.split(['-', '_'])
        .filter(|w| !w.is_empty())
        .map(|w| {
            let lw = w.to_lowercase();
            if matches!(lw.as_str(), "xl" | "sd" | "sdxl" | "vae" | "gguf") {
                lw.to_uppercase()
            } else {
                let mut c = w.chars();
                match c.next() {
                    Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                    None => String::new(),
                }
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// "cuda:0 NVIDIA GeForce RTX 5070 : cudaMallocAsync" → "RTX 5070"
pub fn card(name: &str) -> String {
    let n = name.split(" : ").next().unwrap_or(name);
    let n = n.split_once(' ').filter(|(a, _)| a.contains(':')).map(|(_, b)| b).unwrap_or(n);
    n.replace("NVIDIA ", "").replace("GeForce ", "").trim().to_string()
}

/// "482 113"
pub fn seed_words(s: u64) -> String {
    let d = s.to_string();
    let mut out = String::new();
    for (i, c) in d.chars().enumerate() {
        if i > 0 && (d.len() - i) % 3 == 0 {
            out.push(' ');
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ask() -> Ask {
        Ask { prompt: "A cozy cabin in a snowy pine forest at night".into(), leave_out: "people, text, blurry".into(), look: "Film".into(), w: 1344, h: 768, model: "hearth-xl-1.0.safetensors".into(), steps: 28, seed: 482113, cfg: 6.5, upscale: String::new(), from: None }
    }

    #[test]
    fn the_graph_is_comfyuis_own_and_carries_every_choice() {
        let g = graph(&ask(), "Imagine/cabin");
        assert_eq!(g["3"]["inputs"]["steps"], 28);
        assert_eq!(g["3"]["inputs"]["seed"], 482113);
        assert_eq!(g["5"]["inputs"]["width"], 1344);
        assert_eq!(g["7"]["inputs"]["text"], "people, text, blurry");
        assert!(g["6"]["inputs"]["text"].as_str().unwrap().ends_with("soft film grain, warm tones, cinematic light"));
        assert_eq!(g["9"]["inputs"]["images"], json!(["8", 0]));
        let mut a = ask();
        a.upscale = "4x.pth".into();
        a.from = Some(("in.png".into(), 0.55));
        let g = graph(&a, "x");
        assert_eq!(g["9"]["inputs"]["images"], json!(["11", 0]));
        assert_eq!(g["3"]["inputs"]["latent_image"], json!(["13", 0]));
        assert!(g.get("5").is_none());
    }

    #[test]
    fn words_for_models_cards_and_seeds() {
        assert_eq!(pretty("hearth-xl-1.0.safetensors"), "Hearth XL 1.0");
        assert_eq!(pretty("Fast 1-step.safetensors"), "Fast 1-step");
        assert_eq!(pretty("Upscaler 4×.pth"), "Upscaler 4×");
        assert_eq!(card("cuda:0 NVIDIA GeForce RTX 5070 : cudaMallocAsync"), "RTX 5070");
        assert_eq!(seed_words(482113), "482 113");
        let info = json!({"CheckpointLoaderSimple": {"input": {"required": {"ckpt_name": [["a.safetensors", "b.ckpt"]]}}}, "UpscaleModelLoader": {"input": {"required": {"model_name": ["COMBO", {"options": ["4x.pth"]}]}}}});
        assert_eq!(choices(&info, "CheckpointLoaderSimple", "ckpt_name"), ["a.safetensors", "b.ckpt"]);
        assert_eq!(choices(&info, "UpscaleModelLoader", "model_name"), ["4x.pth"]);
    }
}
