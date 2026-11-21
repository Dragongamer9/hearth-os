//! Saving: the (cropped) picture at full size with the shapes drawn 1:1 on it (tiny-skia), as a PNG, JPEG or WEBP,
//! written to a new file of its own in the editor's folder ($XDG_RUNTIME_DIR/hearth-capture/edit-….png, where
//! Mantel's helper `put` writes them). The picture taken is never changed: saving always makes a new file.

use crate::draw::{self, Shape, R, T};
use crate::picture::Pic;
use std::sync::OnceLock;
use tiny_skia::{FillRule, LineCap, LineJoin, Paint, PathBuilder, Pixmap, Stroke, Transform};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Png,
    Jpeg,
    Webp,
}

impl Kind {
    /// The kind a Save as name asks for, by its ending (no ending: PNG, added by the helper).
    pub fn for_path(p: &str) -> Kind {
        let l = p.to_lowercase();
        if l.ends_with(".jpg") || l.ends_with(".jpeg") {
            Kind::Jpeg
        } else if l.ends_with(".webp") {
            Kind::Webp
        } else {
            Kind::Png
        }
    }
    pub fn ext(self) -> &'static str {
        match self {
            Kind::Png => "png",
            Kind::Jpeg => "jpg",
            Kind::Webp => "webp",
        }
    }
}

struct Fonts {
    semi: Option<fontdue::Font>,
    bold: Option<fontdue::Font>,
}

fn font_file(pattern: &str) -> Option<String> {
    let o = crate::helper::run_prog("fc-match", &["-f".to_string(), "%{file}".to_string(), pattern.to_string()], None, std::time::Duration::from_secs(4));
    let f = o.out.trim().to_string();
    // A variable font draws its default (regular) weight here: a fixed bold one is better.
    if o.ok && !f.is_empty() && !f.contains('[') && std::path::Path::new(&f).is_file() {
        return Some(f);
    }
    None
}

fn load_font(pattern: &str, fallbacks: &[&str]) -> Option<fontdue::Font> {
    let mut files: Vec<String> = font_file(pattern).into_iter().collect();
    files.extend(fallbacks.iter().map(|s| s.to_string()));
    for f in files {
        if let Ok(data) = std::fs::read(&f) {
            if let Ok(font) = fontdue::Font::from_bytes(data, fontdue::FontSettings::default()) {
                return Some(font);
            }
        }
    }
    None
}

fn fonts() -> &'static Fonts {
    static F: OnceLock<Fonts> = OnceLock::new();
    F.get_or_init(|| {
        let bold_files = [
            "/usr/share/fonts/dejavu-sans-fonts/DejaVuSans-Bold.ttf",
            "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf",
            "/usr/share/fonts/liberation-sans/LiberationSans-Bold.ttf",
            "/usr/share/fonts/liberation-sans-fonts/LiberationSans-Bold.ttf",
            "/usr/share/fonts/truetype/liberation/LiberationSans-Bold.ttf",
        ];
        let semi = load_font("sans-serif:style=SemiBold", &bold_files);
        let bold = load_font("sans-serif:style=Bold", &bold_files);
        Fonts { semi, bold }
    })
}

fn paint_of(c: u32, alpha: f32) -> Paint<'static> {
    let (r, g, b) = draw::rgb(c);
    let mut p = Paint::default();
    p.set_color_rgba8(r, g, b, (alpha.clamp(0.0, 1.0) * 255.0).round() as u8);
    p.anti_alias = true;
    p
}

fn stroke(w: f32, cap: LineCap) -> Stroke {
    Stroke { width: w.max(1.0), line_cap: cap, line_join: LineJoin::Round, ..Stroke::default() }
}

/// Coverage A (0..=255) of colour C at ALPHA over pixel (x, y) of the (premultiplied) pixmap.
fn blend(pm: &mut Pixmap, x: i32, y: i32, c: (u8, u8, u8), alpha: f32, a: u8) {
    if x < 0 || y < 0 || x >= pm.width() as i32 || y >= pm.height() as i32 || a == 0 {
        return;
    }
    let k = alpha * a as f32 / 255.0;
    let w = pm.width() as usize;
    let i = (y as usize * w + x as usize) * 4;
    let d = pm.data_mut();
    for (j, s) in [c.0, c.1, c.2, 255u8].iter().enumerate() {
        let v = *s as f32 * k + d[i + j] as f32 * (1.0 - k);
        d[i + j] = v.round().clamp(0.0, 255.0) as u8;
    }
}

/// One line of text as a coverage mask: (mask, width, height, left, top) for text whose top-left is (x, y).
fn text_mask(font: &fontdue::Font, s: &str, size: f32) -> (Vec<u8>, usize, usize, f32) {
    let lm = font.horizontal_line_metrics(size);
    let ascent = lm.map(|m| m.ascent).unwrap_or(size * 0.8);
    let descent = lm.map(|m| m.descent).unwrap_or(-size * 0.2);
    let height = (ascent - descent).ceil().max(1.0) as usize + 2;
    let glyphs: Vec<(fontdue::Metrics, Vec<u8>)> = s.chars().map(|ch| font.rasterize(ch, size)).collect();
    let width = glyphs.iter().map(|g| g.0.advance_width).sum::<f32>().ceil().max(1.0) as usize + 4;
    let mut mask = vec![0u8; width * height];
    let mut pen = 1.0f32;
    let baseline = ascent.round() as i32 + 1;
    for (m, bmp) in &glyphs {
        let gx = pen.round() as i32 + m.xmin;
        let gy = baseline - (m.height as i32 + m.ymin);
        for yy in 0..m.height {
            for xx in 0..m.width {
                let (px, py) = (gx + xx as i32, gy + yy as i32);
                if px < 0 || py < 0 || px as usize >= width || py as usize >= height {
                    continue;
                }
                let v = bmp[yy * m.width + xx];
                let o = py as usize * width + px as usize;
                mask[o] = mask[o].max(v);
            }
        }
        pen += m.advance_width;
    }
    (mask, width, height, ascent - descent)
}

fn dilate(mask: &[u8], w: usize, h: usize, r: f32) -> Vec<u8> {
    let ri = r.ceil() as i32;
    let mut offs = Vec::new();
    for dy in -ri..=ri {
        for dx in -ri..=ri {
            if (dx * dx + dy * dy) as f32 <= r * r + 0.5 {
                offs.push((dx, dy));
            }
        }
    }
    let mut out = vec![0u8; w * h];
    for y in 0..h as i32 {
        for x in 0..w as i32 {
            let mut m = 0u8;
            for (dx, dy) in &offs {
                let (sx, sy) = (x + dx, y + dy);
                if sx >= 0 && sy >= 0 && (sx as usize) < w && (sy as usize) < h {
                    m = m.max(mask[sy as usize * w + sx as usize]);
                }
            }
            out[y as usize * w + x as usize] = m;
        }
    }
    out
}

fn put_mask(pm: &mut Pixmap, mask: &[u8], w: usize, h: usize, left: i32, top: i32, c: (u8, u8, u8), alpha: f32) {
    for y in 0..h {
        for x in 0..w {
            let a = mask[y * w + x];
            if a > 0 {
                blend(pm, left + x as i32, top + y as i32, c, alpha, a);
            }
        }
    }
}

/// Text with its outline (as Mantel's strokeText then fillText): TOP-LEFT at (x, y).
fn draw_text(pm: &mut Pixmap, font: &fontdue::Font, s: &str, size: f32, x: f32, y: f32, fill: u32, outline: (u8, u8, u8), outline_a: f32, ow: f32) {
    let (mask, w, h, _) = text_mask(font, s, size);
    let pad = ow.ceil() as usize + 1;
    // The mask padded so the outline has room.
    let (pw, ph) = (w + pad * 2, h + pad * 2);
    let mut padded = vec![0u8; pw * ph];
    for yy in 0..h {
        padded[(yy + pad) * pw + pad..(yy + pad) * pw + pad + w].copy_from_slice(&mask[yy * w..yy * w + w]);
    }
    let left = x.round() as i32 - 1 - pad as i32;
    let top = y.round() as i32 - 1 - pad as i32;
    if ow > 0.0 {
        let out = dilate(&padded, pw, ph, ow / 2.0);
        put_mask(pm, &out, pw, ph, left, top, outline, outline_a);
    }
    put_mask(pm, &padded, pw, ph, left, top, draw::rgb(fill), 1.0);
}

fn text_width(font: &fontdue::Font, s: &str, size: f32) -> f32 {
    s.chars().map(|c| font.metrics(c, size).advance_width).sum()
}

/// The picture with its shapes, at full size: the crop (or all of it), 1:1.
pub fn render(pic: &Pic, shapes: &[Shape], crop: Option<R>) -> Result<Pixmap, String> {
    let full = R::new(0.0, 0.0, pic.w as f32, pic.h as f32);
    let v = draw::clamp_rect(crop.unwrap_or(full), pic.w as f32, pic.h as f32);
    let (vx, vy, vw, vh) = (v.x as usize, v.y as usize, v.w as usize, v.h as usize);
    let mut pm = Pixmap::new(vw as u32, vh as u32).ok_or("the picture is empty")?;
    let src = pic.rgba();
    if src.len() < (pic.w * pic.h * 4) as usize {
        return Err("the picture isn't loaded".into());
    }
    let pw = pic.w as usize;
    {
        let d = pm.data_mut();
        for y in 0..vh {
            for x in 0..vw {
                let si = ((vy + y) * pw + vx + x) * 4;
                let di = (y * vw + x) * 4;
                let a = src[si + 3] as u32;
                for c in 0..3 {
                    d[di + c] = ((src[si + c] as u32 * a + 127) / 255) as u8;
                }
                d[di + 3] = a as u8;
            }
        }
    }
    let tr = Transform::from_translate(-(vx as f32), -(vy as f32));
    for sh in shapes {
        let lw = sh.w.max(1.0);
        match sh.t {
            T::Arrow => {
                let hd = draw::arrow_head(sh.x1, sh.y1, sh.x2, sh.y2, (lw * 3.2).max(10.0));
                let (bx, by) = ((hd[0] + hd[2]) / 2.0, (hd[1] + hd[3]) / 2.0);
                let mut pb = PathBuilder::new();
                pb.move_to(sh.x1, sh.y1);
                pb.line_to(bx, by);
                if let Some(path) = pb.finish() {
                    pm.stroke_path(&path, &paint_of(sh.c, 1.0), &stroke(lw, LineCap::Round), tr, None);
                }
                let mut pb = PathBuilder::new();
                pb.move_to(sh.x2, sh.y2);
                pb.line_to(hd[0], hd[1]);
                pb.line_to(hd[2], hd[3]);
                pb.close();
                if let Some(path) = pb.finish() {
                    pm.fill_path(&path, &paint_of(sh.c, 1.0), FillRule::Winding, tr, None);
                    pm.stroke_path(&path, &paint_of(sh.c, 1.0), &stroke(lw.min(3.0), LineCap::Round), tr, None);
                }
            }
            T::Box => {
                let r = draw::norm_rect(sh.x1, sh.y1, sh.x2, sh.y2);
                let mut pb = PathBuilder::new();
                pb.move_to(r.x, r.y);
                pb.line_to(r.x + r.w, r.y);
                pb.line_to(r.x + r.w, r.y + r.h);
                pb.line_to(r.x, r.y + r.h);
                pb.close();
                if let Some(path) = pb.finish() {
                    pm.stroke_path(&path, &paint_of(sh.c, 1.0), &stroke(lw, LineCap::Round), tr, None);
                }
            }
            T::Pen | T::Hl => {
                let hl = sh.t == T::Hl;
                let p = &sh.pts;
                if p.len() >= 2 {
                    let mut pb = PathBuilder::new();
                    pb.move_to(p[0], p[1]);
                    let mut i = 2;
                    while i + 1 < p.len() {
                        pb.line_to(p[i], p[i + 1]);
                        i += 2;
                    }
                    if p.len() == 2 {
                        pb.line_to(p[0] + 0.1, p[1]);
                    }
                    if let Some(path) = pb.finish() {
                        let (w, cap, a) = if hl { (lw * 3.5, LineCap::Square, 0.38) } else { (lw, LineCap::Round, 1.0) };
                        pm.stroke_path(&path, &paint_of(sh.c, a), &stroke(w, cap), tr, None);
                    }
                }
            }
            T::Text => {
                if let Some(font) = fonts().semi.as_ref().or(fonts().bold.as_ref()) {
                    let size = sh.size.max(6.0);
                    let ow = (size / 7.0).max(2.0);
                    let (outline, oa) = if sh.c == 0x111111 { ((255, 255, 255), 1.0) } else { ((0, 0, 0), 0.65) };
                    for (li, line) in sh.text.split('\n').enumerate() {
                        let y = sh.y1 - vy as f32 + li as f32 * size * 1.2;
                        draw_text(&mut pm, font, line, size, sh.x1 - vx as f32, y, sh.c, outline, oa, ow);
                    }
                }
            }
            T::Step => {
                let rad = (16.0 * ((sh.w / 6.0) + 0.6) / 1.6).max(8.0);
                if let Some(path) = PathBuilder::from_circle(sh.x1, sh.y1, rad) {
                    pm.fill_path(&path, &paint_of(sh.c, 1.0), FillRule::Winding, tr, None);
                    pm.stroke_path(&path, &paint_of(0xffffff, 1.0), &stroke((rad / 7.0).max(1.5), LineCap::Round), tr, None);
                }
                if let Some(font) = fonts().bold.as_ref().or(fonts().semi.as_ref()) {
                    let n = sh.n.to_string();
                    let size = (rad * 1.15).round();
                    let ink = if sh.c == 0xffffff || sh.c == 0xffd166 { 0x111111 } else { 0xffffff };
                    let lm = font.horizontal_line_metrics(size);
                    let (asc, desc) = lm.map(|m| (m.ascent, m.descent)).unwrap_or((size * 0.8, -size * 0.2));
                    let w = text_width(font, &n, size);
                    let cx = sh.x1 - vx as f32;
                    let cy = sh.y1 - vy as f32 + rad * 0.05;
                    // Middle of the digits (their height is about the ascent less a fifth) on the middle of the disc.
                    let top = cy - (asc - desc) / 2.0 + (asc + desc) * 0.0;
                    draw_text(&mut pm, font, &n, size, cx - w / 2.0, top, ink, (0, 0, 0), 0.0, 0.0);
                }
            }
            T::Blur | T::Pix => {
                let q = draw::norm_rect(sh.x1 - vx as f32, sh.y1 - vy as f32, sh.x2 - vx as f32, sh.y2 - vy as f32);
                let x0 = q.x.floor().max(0.0) as usize;
                let y0 = q.y.floor().max(0.0) as usize;
                let x1 = ((q.x + q.w).ceil() as usize).min(vw);
                let y1 = ((q.y + q.h).ceil() as usize).min(vh);
                if x1 >= x0 + 2 && y1 >= y0 + 2 {
                    let (rw, rh) = (x1 - x0, y1 - y0);
                    let mut region = vec![0u8; rw * rh * 4];
                    for y in 0..rh {
                        for x in 0..rw {
                            let si = ((vy + y0 + y) * pw + vx + x0 + x) * 4;
                            region[(y * rw + x) * 4..(y * rw + x) * 4 + 4].copy_from_slice(&src[si..si + 4]);
                        }
                    }
                    if sh.t == T::Pix {
                        draw::pixelate(&mut region, rw, rh, (1.0 / draw::PIX).round() as usize);
                    } else {
                        draw::blur(&mut region, rw, rh, (1.0 / draw::BLUR).round() as usize);
                    }
                    let d = pm.data_mut();
                    for y in 0..rh {
                        for x in 0..rw {
                            let di = ((y0 + y) * vw + x0 + x) * 4;
                            let si = (y * rw + x) * 4;
                            let a = region[si + 3] as u32;
                            for c in 0..3 {
                                d[di + c] = ((region[si + c] as u32 * a + 127) / 255) as u8;
                            }
                            d[di + 3] = a as u8;
                        }
                    }
                }
            }
            T::Crop => {}
        }
    }
    Ok(pm)
}

/// The rendered picture as bytes of KIND.
pub fn encode(pm: &Pixmap, kind: Kind) -> Result<Vec<u8>, String> {
    let straight = || -> Vec<u8> {
        let mut v = Vec::with_capacity(pm.data().len());
        for px in pm.pixels() {
            let c = px.demultiply();
            v.extend_from_slice(&[c.red(), c.green(), c.blue(), c.alpha()]);
        }
        v
    };
    match kind {
        Kind::Png => pm.encode_png().map_err(|e| format!("{e}")),
        Kind::Jpeg => {
            if pm.width() > 65535 || pm.height() > 65535 {
                return Err("too big for a JPEG".into());
            }
            let rgba = straight();
            let rgb: Vec<u8> = rgba.chunks(4).flat_map(|c| [c[0], c[1], c[2]]).collect();
            let mut buf = Vec::new();
            jpeg_encoder::Encoder::new(&mut buf, 92)
                .encode(&rgb, pm.width() as u16, pm.height() as u16, jpeg_encoder::ColorType::Rgb)
                .map_err(|e| format!("{e}"))?;
            Ok(buf)
        }
        Kind::Webp => {
            let rgba = straight();
            let mut buf = Vec::new();
            image_webp::WebPEncoder::new(&mut buf)
                .encode(&rgba, pm.width(), pm.height(), image_webp::ColorType::Rgba8)
                .map_err(|e| format!("{e}"))?;
            Ok(buf)
        }
    }
}

/// The picture as it is now, written to a new file of its own in the editor's folder: its path.
pub fn finished(pic: &Pic, shapes: &[Shape], crop: Option<R>, kind: Kind) -> Result<String, String> {
    let pm = render(pic, shapes, crop)?;
    let bytes = encode(&pm, kind)?;
    let dir = crate::helper::runtime_dir();
    let path = dir.join(format!("edit-{}.{}", crate::helper::now_ms(), kind.ext()));
    let tmp = dir.join(format!(".edit-{}.part", crate::helper::now_ms()));
    std::fs::write(&tmp, &bytes).map_err(|e| format!("{e}"))?;
    std::fs::rename(&tmp, &path).map_err(|e| format!("{e}"))?;
    Ok(path.to_string_lossy().to_string())
}

pub fn finished_async(pic: Pic, shapes: Vec<Shape>, crop: Option<R>, kind: Kind) -> impl std::future::Future<Output = Result<String, String>> + Send + 'static {
    let (tx, rx) = futures::channel::oneshot::channel();
    let _ = std::thread::Builder::new().name("capture-export".into()).spawn(move || {
        let _ = tx.send(finished(&pic, &shapes, crop, kind));
    });
    async move { rx.await.unwrap_or_else(|_| Err("it stopped".into())) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::widget::image::Handle;

    fn pic(w: u32, h: u32) -> Pic {
        let mut rgba = vec![0u8; (w * h * 4) as usize];
        for (i, px) in rgba.chunks_mut(4).enumerate() {
            let v = if (i as u32 % w + i as u32 / w) % 2 == 0 { 255 } else { 0 };
            px.copy_from_slice(&[v, v, v, 255]);
        }
        Pic { path: String::new(), w, h, handle: Handle::from_rgba(w, h, rgba), pix: Handle::from_rgba(1, 1, vec![0u8; 4]), blur: Handle::from_rgba(1, 1, vec![0u8; 4]) }
    }

    #[test]
    fn saves_the_crop_at_full_size_with_its_shapes() {
        let p = pic(200, 100);
        let shapes = vec![
            Shape::rect(T::Box, 0xff4d4d, 6.0, 20.0, 20.0, 80.0, 60.0),
            Shape::rect(T::Pix, 0xffffff, 6.0, 100.0, 10.0, 180.0, 90.0),
        ];
        let pm = render(&p, &shapes, Some(R::new(10.0, 10.0, 180.0, 80.0))).unwrap();
        assert_eq!((pm.width(), pm.height()), (180, 80));
        // The box's left edge (x 20 in the picture, 10 in the crop) is red.
        let px = pm.pixel(10, 30).unwrap().demultiply();
        assert!(px.red() > 200 && px.green() < 120, "{px:?}");
        // Under Pixelate the checkerboard is grey.
        let g = pm.pixel(130, 40).unwrap().demultiply();
        assert!(g.red() > 60 && g.red() < 200, "{g:?}");
        let png = encode(&pm, Kind::Png).unwrap();
        assert_eq!(&png[..4], b"\x89PNG");
        assert_eq!(&encode(&pm, Kind::Jpeg).unwrap()[..3], b"\xff\xd8\xff");
        assert_eq!(&encode(&pm, Kind::Webp).unwrap()[8..12], b"WEBP");
        assert_eq!(Kind::for_path("/x/a.JPG"), Kind::Jpeg);
        assert_eq!(Kind::for_path("/x/a"), Kind::Png);
    }
}
