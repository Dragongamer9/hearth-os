//! CaptureEditor.qml in Rust: the markup editor. Tools: arrow, box, pen, highlighter, text, blur, pixelate, numbered
//! steps, crop; seven colours, three widths, undo and redo. Save and copy (Pictures › Screenshots, and copied),
//! Copy, Save as, Show in folder, Read text (tesseract) and QR code (zbarimg), New screenshot, and Share (copy, save,
//! open with an app for pictures, send to your phone when Phone link is on; Open with and Send use the saved file,
//! saved first when it isn't). The shapes are kept in the picture's own pixels (draw.rs): the view draws them scaled
//! to fit, and saving draws them 1:1 on the full-size (cropped) picture (export.rs). The picture taken is never
//! changed: saving always makes a new file (Save as never replaces one without asking).

use crate::btn::{b, label};
use crate::draw::{self, Shape, R, T};
use crate::export::{self, Kind};
use crate::helper::{self, a};
use crate::picture::Pic;
use crate::Msg;
use hearth_ui::ui::{self, boxed, gap};
use hearth_ui::Pal;
use iced::widget::canvas::{self, Frame, Geometry, LineCap, LineJoin, Path, Stroke};
use iced::widget::image::FilterMethod;
use iced::widget::{column, container, pin, responsive, row, scrollable, stack, text_editor, text_input, Column, Space};
use iced::{mouse, Background, Border, Color, Element, Font, Length, Padding, Pixels, Point, Rectangle, Renderer, Size, Task, Theme};
use serde_json::Value;

pub const SAVE_FIELD: &str = "capture-saveas";
pub const TEXT_FIELD: &str = "capture-text";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Panel {
    None,
    Text,
    Qr,
    SaveAs,
    Share,
}

#[derive(Debug, Clone)]
pub struct ShareApp {
    pub id: String,
    pub name: String,
    pub default: bool,
}

/// What to do once the picture as it is now has been made.
#[derive(Debug, Clone)]
pub enum Job {
    SaveCopy,
    CopyOnly,
    SaveAs { to: String, replace: bool },
    Ocr,
    Qr,
    OpenWith(usize),
    SendPhone(usize),
}

pub struct Ed {
    pub pic: Option<Pic>,
    pub load_error: String,
    pub crop: Option<R>,
    pub shapes: Vec<Shape>,
    pub live: Option<Shape>,
    pub undo: Vec<(Vec<Shape>, Option<R>)>,
    pub redo: Vec<(Vec<Shape>, Option<R>)>,
    pub tool: T,
    pub color: u32,
    pub width: char,
    pub panel: Panel,
    pub share_apps: Vec<ShareApp>,
    pub share_phones: Option<Value>,
    pub edits: u64,
    pub saved_edits: Option<u64>,
    pub ocr_state: &'static str,
    pub ocr_text: String,
    pub ocr: text_editor::Content,
    pub qr_codes: Vec<String>,
    pub qr_state: &'static str,
    pub qr_error: String,
    pub save_as_path: String,
    pub save_as_exists: bool,
    pub toast: String,
    pub toast_seq: u64,
    pub last_saved: String,
    pub busy: bool,
    pub editing_text: bool,
    pub text_x: f32,
    pub text_y: f32,
    pub text_in: String,
    /// The first ones CI asks for (--sample, --save, --show) once the picture is in.
    pub after_load: Vec<EMsg>,
    /// The crop shown (the view of a cropped picture is a copy of that part: iced's own image crop doesn't stay in
    /// its place).
    pub view_img: Option<(R, iced::widget::image::Handle)>,
}

#[derive(Debug, Clone)]
pub enum EMsg {
    Tool(T),
    Color(u32),
    Width(char),
    Undo,
    Redo,
    Press(f32, f32),
    Move(f32, f32),
    Release,
    TextIn(String),
    TextCommit,
    CropAll,
    CropDone,
    SaveCopy,
    Copy,
    SaveAsOpen,
    SaveAsPath(String),
    SaveAs(bool),
    PanelClose,
    ShowFolder,
    Share,
    ReadText,
    ReadQr,
    NewShot,
    OpenWith(usize),
    SendPhone(usize),
    CopyText,
    QrCopy(usize),
    QrOpen(usize),
    OcrAction(text_editor::Action),
    Kbd,
    Sample,
    Esc,
    // answers
    Exported(Job, Result<String, String>),
    Saved(Job, Value),
    SavedCopied(String, Value),
    Said(Value, &'static str, &'static str),
    OcrDone(Value),
    QrDone(Value),
    Apps(Value),
    Phones(Value),
    ToastEnd(u64),
}

/// What the editor asks of the rest of Capture.
pub enum Fx {
    /// A picture was saved (the recent list changes).
    Saved(String),
    /// New screenshot (the mode chosen on Take).
    NewShot,
}

impl Ed {
    pub fn new() -> Ed {
        Ed {
            pic: None,
            load_error: String::new(),
            crop: None,
            shapes: Vec::new(),
            live: None,
            undo: Vec::new(),
            redo: Vec::new(),
            tool: T::Arrow,
            color: draw::COLORS[0],
            width: 'm',
            panel: Panel::None,
            share_apps: Vec::new(),
            share_phones: None,
            edits: 0,
            saved_edits: None,
            ocr_state: "",
            ocr_text: String::new(),
            ocr: text_editor::Content::new(),
            qr_codes: Vec::new(),
            qr_state: "",
            qr_error: String::new(),
            save_as_path: String::new(),
            save_as_exists: false,
            toast: String::new(),
            toast_seq: 0,
            last_saved: String::new(),
            busy: false,
            editing_text: false,
            text_x: 0.0,
            text_y: 0.0,
            text_in: String::new(),
            after_load: Vec::new(),
            view_img: None,
        }
    }

    /// Keeps the view's picture in step with the crop (a copy of the cropped part, made only when it changes).
    pub fn sync_view(&mut self) {
        let Some(pic) = self.pic.as_ref() else {
            self.view_img = None;
            return;
        };
        let v = self.view_rect();
        if v == self.full() {
            self.view_img = None;
            return;
        }
        if self.view_img.as_ref().map(|x| x.0) == Some(v) {
            return;
        }
        let c = draw::clamp_rect(v, pic.w as f32, pic.h as f32);
        let (x0, y0, w, h) = (c.x as usize, c.y as usize, c.w as usize, c.h as usize);
        let src = pic.rgba();
        let pw = pic.w as usize;
        let mut out = Vec::with_capacity(w * h * 4);
        for y in y0..y0 + h {
            let i = (y * pw + x0) * 4;
            if i + w * 4 <= src.len() {
                out.extend_from_slice(&src[i..i + w * 4]);
            }
        }
        if out.len() == w * h * 4 {
            self.view_img = Some((v, iced::widget::image::Handle::from_rgba(w as u32, h as u32, out)));
        }
    }

    /// A new picture: everything starts again (the colour and width stay, as in Mantel).
    pub fn start(&mut self, crop: Option<R>) {
        let (color, width) = (self.color, self.width);
        let after = std::mem::take(&mut self.after_load);
        *self = Ed::new();
        self.color = color;
        self.width = width;
        self.crop = crop;
        self.after_load = after;
    }

    pub fn img_w(&self) -> f32 {
        self.pic.as_ref().map(|p| p.w as f32).unwrap_or(0.0)
    }
    pub fn img_h(&self) -> f32 {
        self.pic.as_ref().map(|p| p.h as f32).unwrap_or(0.0)
    }
    pub fn full(&self) -> R {
        R::new(0.0, 0.0, self.img_w().max(1.0), self.img_h().max(1.0))
    }
    /// What the view shows: all of it while cropping (or with no crop), else the crop.
    pub fn view_rect(&self) -> R {
        match self.crop {
            Some(c) if self.tool != T::Crop => c,
            _ => self.full(),
        }
    }
    pub fn lw(&self) -> f32 {
        draw::width_of(self.width)
    }
    pub fn text_size(&self) -> f32 {
        draw::text_size_of(self.width)
    }

    fn snapshot(&self) -> (Vec<Shape>, Option<R>) {
        (self.shapes.clone(), self.crop)
    }
    fn change(&mut self, f: impl FnOnce(&mut Ed)) {
        let s = self.snapshot();
        self.undo.push(s);
        if self.undo.len() > 100 {
            self.undo.remove(0);
        }
        self.redo.clear();
        f(self);
        self.edits += 1;
    }
    fn add(&mut self, sh: Shape) {
        if draw::worth_keeping(&sh) {
            self.change(|e| e.shapes.push(sh));
        }
    }
    fn commit_text(&mut self) {
        if self.editing_text && !self.text_in.trim().is_empty() {
            let sh = Shape::text(self.color, self.lw(), self.text_x, self.text_y, &self.text_in.clone(), self.text_size());
            self.add(sh);
        }
        self.editing_text = false;
        self.text_in.clear();
    }

    /// A sample of every tool (the pictures in CI show the editor with them; Mantel's sample()).
    pub fn sample(&mut self) {
        let v = self.view_rect();
        let (w, h, x, y) = (v.w, v.h, v.x, v.y);
        self.change(|e| {
            e.shapes.extend([
                Shape::rect(T::Box, 0xff4d4d, 6.0, x + w * 0.08, y + h * 0.12, x + w * 0.42, y + h * 0.42),
                Shape::rect(T::Arrow, 0xff8a3d, 6.0, x + w * 0.62, y + h * 0.70, x + w * 0.44, y + h * 0.44),
                Shape::line(T::Hl, 0xffd166, 6.0, vec![x + w * 0.5, y + h * 0.16, x + w * 0.9, y + h * 0.16]),
                Shape::line(T::Pen, 0x4cd97b, 4.0, vec![x + w * 0.1, y + h * 0.8, x + w * 0.16, y + h * 0.72, x + w * 0.22, y + h * 0.82, x + w * 0.3, y + h * 0.7]),
                Shape::rect(T::Pix, 0xffffff, 6.0, x + w * 0.55, y + h * 0.25, x + w * 0.92, y + h * 0.42),
                Shape::rect(T::Blur, 0xffffff, 6.0, x + w * 0.68, y + h * 0.55, x + w * 0.94, y + h * 0.9),
                Shape::text(0xffffff, 6.0, x + w * 0.1, y + h * 0.46, "Click here first", 34.0),
                Shape::step(0x4aa8ff, 6.0, x + w * 0.07, y + h * 0.1, 1),
                Shape::step(0x4aa8ff, 6.0, x + w * 0.64, y + h * 0.72, 2),
            ]);
        });
    }

    /// The picture as it is now (a new file of its own), then JOB.
    fn finished(&mut self, job: Job, kind: Kind) -> Task<Msg> {
        let Some(pic) = self.pic.clone() else { return self.say("Couldn't make the picture") };
        self.busy = true;
        let (shapes, crop) = (self.shapes.clone(), self.crop);
        Task::perform(export::finished_async(pic, shapes, crop, kind), move |r| Msg::Ed(EMsg::Exported(job.clone(), r)))
    }

    pub fn say(&mut self, t: impl Into<String>) -> Task<Msg> {
        self.toast = t.into();
        self.toast_seq += 1;
        let seq = self.toast_seq;
        Task::perform(hearth_ui::run::after(5000), move |_| Msg::Ed(EMsg::ToastEnd(seq)))
    }

    fn open_save_as(&mut self, folder: &str) -> Task<Msg> {
        let home = std::env::var("HOME").unwrap_or_default();
        let folder = if folder.is_empty() { format!("{home}/Pictures/Screenshots") } else { folder.to_string() };
        self.save_as_path = format!("{folder}/{}", draw::default_name(hearth_ui::time::now()));
        self.save_as_exists = false;
        self.panel = Panel::SaveAs;
        Task::batch([iced::widget::operation::focus(SAVE_FIELD), iced::widget::operation::select_all(SAVE_FIELD)])
    }

    /// Share's Open with / Send: the saved file as it is now (the one saved already when nothing changed since),
    /// else saved now (Pictures › Screenshots, as Save does).
    fn with_saved(&mut self, job: Job) -> Task<Msg> {
        if !self.last_saved.is_empty() && self.saved_edits == Some(self.edits) {
            let v = serde_json::json!({"ok": true, "path": self.last_saved, "name": self.last_saved.rsplit('/').next().unwrap_or("")});
            return Task::done(Msg::Ed(EMsg::Saved(job, v)));
        }
        self.finished(job, Kind::Png)
    }

    pub fn state(&self) -> Value {
        serde_json::json!({"tool": format!("{:?}", self.tool), "shapes": self.shapes.len(), "ready": self.pic.is_some(), "panel": format!("{:?}", self.panel),
            "ocr": self.ocr_state, "qr": self.qr_codes, "toast": self.toast, "saved": self.last_saved, "busy": self.busy})
    }
}

fn call(args: Vec<String>, then: impl Fn(Value) -> EMsg + Send + 'static) -> Task<Msg> {
    Task::perform(helper::call(args, None, 60), move |v| Msg::Ed(then(v)))
}

fn kind_name(k: Kind) -> &'static str {
    match k {
        Kind::Png => "PNG",
        Kind::Jpeg => "JPEG",
        Kind::Webp => "WEBP",
    }
}

/// The editor's own messages. `tools` is the helper's `tools` (tesseract, zbarimg, the folder).
pub fn update(ed: &mut Ed, m: EMsg, tools: &Value, fx: &mut Vec<Fx>) -> Task<Msg> {
    let t = update_inner(ed, m, tools, fx);
    ed.sync_view();
    t
}

fn update_inner(ed: &mut Ed, m: EMsg, tools: &Value, fx: &mut Vec<Fx>) -> Task<Msg> {
    match m {
        EMsg::Tool(t) => {
            ed.commit_text();
            ed.tool = t;
        }
        EMsg::Color(c) => ed.color = c,
        EMsg::Width(w) => ed.width = w,
        EMsg::Undo => {
            if let Some(s) = ed.undo.pop() {
                ed.redo.push(ed.snapshot());
                ed.shapes = s.0;
                ed.crop = s.1;
                ed.edits += 1;
            }
        }
        EMsg::Redo => {
            if let Some(s) = ed.redo.pop() {
                ed.undo.push(ed.snapshot());
                ed.shapes = s.0;
                ed.crop = s.1;
                ed.edits += 1;
            }
        }
        EMsg::Press(x, y) => {
            if ed.pic.is_none() {
                return Task::none();
            }
            match ed.tool {
                T::Text => {
                    ed.commit_text();
                    ed.text_x = x;
                    ed.text_y = y;
                    ed.editing_text = true;
                    return iced::widget::operation::focus(TEXT_FIELD);
                }
                T::Step => {
                    let n = draw::next_step(&ed.shapes);
                    let sh = Shape::step(ed.color, ed.lw(), x, y, n);
                    ed.add(sh);
                }
                T::Pen | T::Hl => {
                    let c = if ed.tool == T::Hl && ed.color == draw::COLORS[0] { 0xffd166 } else { ed.color };
                    ed.live = Some(Shape::line(ed.tool, c, ed.lw(), vec![x, y]));
                }
                t => ed.live = Some(Shape::rect(t, ed.color, ed.lw(), x, y, x, y)),
            }
        }
        EMsg::Move(x, y) => {
            let v = ed.view_rect();
            let (x, y) = (x.clamp(v.x, v.x + v.w), y.clamp(v.y, v.y + v.h));
            if let Some(l) = ed.live.as_mut() {
                if matches!(l.t, T::Pen | T::Hl) {
                    l.pts.extend([x, y]);
                } else {
                    l.x2 = x;
                    l.y2 = y;
                }
            }
        }
        EMsg::Release => {
            if let Some(l) = ed.live.take() {
                if l.t == T::Crop {
                    let r = draw::clamp_rect(draw::norm_rect(l.x1, l.y1, l.x2, l.y2), ed.img_w(), ed.img_h());
                    if r.w >= 8.0 && r.h >= 8.0 {
                        ed.change(|e| e.crop = Some(r));
                    }
                } else {
                    ed.add(l);
                }
            }
        }
        EMsg::TextIn(s) => ed.text_in = s,
        EMsg::TextCommit => ed.commit_text(),
        EMsg::CropAll => ed.change(|e| e.crop = None),
        EMsg::CropDone => ed.tool = T::Arrow,
        EMsg::Sample => ed.sample(),
        EMsg::Esc => {
            if ed.editing_text {
                ed.editing_text = false;
                ed.text_in.clear();
            } else if ed.panel != Panel::None {
                ed.panel = Panel::None;
            }
        }
        EMsg::SaveCopy => {
            ed.commit_text();
            return ed.finished(Job::SaveCopy, Kind::Png);
        }
        EMsg::Copy => {
            ed.commit_text();
            return ed.finished(Job::CopyOnly, Kind::Png);
        }
        EMsg::SaveAsOpen => return ed.open_save_as(helper::s(tools, "folder")),
        EMsg::SaveAsPath(s) => {
            ed.save_as_path = s;
            ed.save_as_exists = false;
        }
        EMsg::SaveAs(replace) => {
            let to = ed.save_as_path.trim().to_string();
            if to.is_empty() {
                return Task::none();
            }
            ed.commit_text();
            let kind = Kind::for_path(&to);
            return ed.finished(Job::SaveAs { to, replace }, kind);
        }
        EMsg::PanelClose => ed.panel = Panel::None,
        EMsg::ShowFolder => {
            let p = if ed.last_saved.is_empty() { helper::s(tools, "folder").to_string() } else { ed.last_saved.clone() };
            return call(a(&["folder", &p]), |_| EMsg::ToastEnd(u64::MAX));
        }
        EMsg::Share => {
            ed.panel = Panel::Share;
            ed.share_phones = None;
            return Task::batch([call(a(&["apps"]), EMsg::Apps), call(a(&["phones"]), EMsg::Phones)]);
        }
        EMsg::Apps(v) => {
            ed.share_apps = v
                .get("apps")
                .and_then(Value::as_array)
                .map(|l| l.iter().map(|x| ShareApp { id: helper::s(x, "id").into(), name: helper::s(x, "name").into(), default: helper::b(x, "default") }).collect())
                .unwrap_or_default();
        }
        EMsg::Phones(v) => ed.share_phones = Some(v),
        EMsg::ReadText => {
            ed.panel = Panel::Text;
            ed.ocr_state = "busy";
            ed.ocr_text.clear();
            ed.ocr = text_editor::Content::with_text("Reading…");
            return ed.finished(Job::Ocr, Kind::Png);
        }
        EMsg::ReadQr => {
            ed.panel = Panel::Qr;
            ed.qr_state = "busy";
            ed.qr_codes.clear();
            return ed.finished(Job::Qr, Kind::Png);
        }
        EMsg::NewShot => fx.push(Fx::NewShot),
        EMsg::OpenWith(i) => return ed.with_saved(Job::OpenWith(i)),
        EMsg::SendPhone(i) => return ed.with_saved(Job::SendPhone(i)),
        EMsg::CopyText => {
            let t = ed.ocr_text.clone().into_bytes();
            return Task::perform(helper::call(a(&["copy-text"]), Some(t), 20), |v| Msg::Ed(EMsg::Said(v, "Text copied", "")));
        }
        EMsg::QrCopy(i) => {
            let t = ed.qr_codes.get(i).cloned().unwrap_or_default().into_bytes();
            return Task::perform(helper::call(a(&["copy-text"]), Some(t), 20), |v| Msg::Ed(EMsg::Said(v, "Copied", "")));
        }
        EMsg::QrOpen(i) => {
            if let Some(c) = ed.qr_codes.get(i) {
                helper::detached("xdg-open", &[c.clone()]);
            }
        }
        EMsg::OcrAction(act) => {
            if !act.is_edit() {
                ed.ocr.perform(act);
            }
        }
        EMsg::Kbd => {
            // The on-screen keyboard comes up for the field that has the keyboard (Plasma's, or Hearth's).
            return iced::widget::operation::focus(SAVE_FIELD);
        }
        EMsg::Exported(job, r) => {
            ed.busy = false;
            let p = match r {
                Ok(p) => p,
                Err(_) => {
                    return match job {
                        Job::Ocr => {
                            ed.ocr_state = "error";
                            ed.ocr = text_editor::Content::with_text("Couldn't read it. ");
                            Task::none()
                        }
                        Job::Qr => {
                            ed.qr_state = "error";
                            Task::none()
                        }
                        Job::SaveAs { to, .. } if Kind::for_path(&to) != Kind::Png => {
                            ed.say(format!("This PC can't save {} pictures here: try .png or .jpg", kind_name(Kind::for_path(&to))))
                        }
                        _ => ed.say("Couldn't make the picture"),
                    };
                }
            };
            return match job {
                Job::SaveCopy | Job::OpenWith(_) | Job::SendPhone(_) => call(a(&["save", &p]), move |v| EMsg::Saved(job.clone(), v)),
                Job::SaveAs { ref to, replace } => {
                    let mut args = a(&["save", &p, "--to", to]);
                    if replace {
                        args.push("--replace".into());
                    }
                    let j = job.clone();
                    call(args, move |v| EMsg::Saved(j.clone(), v))
                }
                Job::CopyOnly => call(a(&["copy", &p]), |v| EMsg::Said(v, "Copied: paste it into any app", "Couldn't copy it")),
                Job::Ocr => call(a(&["ocr", &p]), EMsg::OcrDone),
                Job::Qr => call(a(&["qr", &p]), EMsg::QrDone),
            };
        }
        EMsg::Saved(job, v) => {
            let ok = helper::b(&v, "ok");
            let path = helper::s(&v, "path").to_string();
            let name = helper::s(&v, "name").to_string();
            match job {
                Job::SaveAs { .. } => {
                    if helper::b(&v, "exists") {
                        ed.save_as_exists = true;
                        return ed.say(helper::s(&v, "error").to_string());
                    }
                    if !ok && !helper::s(&v, "kind").is_empty() {
                        return ed.say(format!("This PC can't save {} pictures here: try .png or .jpg", helper::s(&v, "kind").to_uppercase()));
                    }
                    if !ok {
                        let e = helper::s(&v, "error");
                        return ed.say(if e.is_empty() { "Couldn't save it".to_string() } else { e.to_string() });
                    }
                    ed.last_saved = path.clone();
                    ed.saved_edits = Some(ed.edits);
                    fx.push(Fx::Saved(path));
                    ed.panel = Panel::None;
                    return ed.say(format!("Saved as {name}"));
                }
                _ if !ok => {
                    let e = helper::s(&v, "error");
                    return ed.say(if e.is_empty() { "Couldn't save it".to_string() } else { e.to_string() });
                }
                Job::SaveCopy => {
                    ed.last_saved = path.clone();
                    ed.saved_edits = Some(ed.edits);
                    fx.push(Fx::Saved(path.clone()));
                    return call(a(&["copy", &path]), move |c| EMsg::SavedCopied(name.clone(), c));
                }
                Job::OpenWith(i) => {
                    if !path.is_empty() && path != ed.last_saved {
                        ed.last_saved = path.clone();
                        ed.saved_edits = Some(ed.edits);
                        fx.push(Fx::Saved(path.clone()));
                    }
                    let Some(app) = ed.share_apps.get(i).cloned() else { return Task::none() };
                    let msg = format!("Opening it in {}", app.name);
                    return Task::perform(helper::call(a(&["open-with", &app.id, &path]), None, 30), move |v| {
                        let ok = helper::b(&v, "ok");
                        let e = helper::s(&v, "error").to_string();
                        Msg::Ed(EMsg::Said(serde_json::json!({"ok": ok, "error": e, "say": msg.clone()}), "", "Couldn't open it"))
                    });
                }
                Job::SendPhone(i) => {
                    if !path.is_empty() && path != ed.last_saved {
                        ed.last_saved = path.clone();
                        ed.saved_edits = Some(ed.edits);
                        fx.push(Fx::Saved(path.clone()));
                    }
                    let phones = ed.share_phones.as_ref().and_then(|p| p.get("phones")).and_then(Value::as_array).cloned().unwrap_or_default();
                    let Some(ph) = phones.get(i) else { return Task::none() };
                    let (pid, pname) = (helper::s(ph, "id").to_string(), helper::s(ph, "name").to_string());
                    let t1 = ed.say(format!("Sending it to {pname}…"));
                    let sent = format!("Sent to {pname}");
                    let t2 = Task::perform(helper::call(a(&["send-phone", &path, &pid]), None, 45), move |v| {
                        let ok = helper::b(&v, "ok");
                        let e = helper::s(&v, "error").to_string();
                        Msg::Ed(EMsg::Said(serde_json::json!({"ok": ok, "error": e, "say": sent.clone()}), "", "Couldn't send it"))
                    });
                    return Task::batch([t1, t2]);
                }
                _ => {}
            }
        }
        EMsg::SavedCopied(name, c) => {
            return ed.say(if helper::b(&c, "ok") { format!("Saved as {name} and copied") } else { format!("Saved as {name}") });
        }
        EMsg::Said(v, good, bad) => {
            if helper::b(&v, "ok") {
                let s = helper::s(&v, "say");
                return ed.say(if s.is_empty() { good.to_string() } else { s.to_string() });
            }
            let e = helper::s(&v, "error");
            return ed.say(if e.is_empty() { bad.to_string() } else { e.to_string() });
        }
        EMsg::OcrDone(v) => {
            if !helper::s(&v, "missing").is_empty() {
                ed.ocr_state = "missing";
                ed.ocr = text_editor::Content::with_text("Reading text needs tesseract (tesseract-ocr), which isn't installed on this PC.");
            } else if !helper::b(&v, "ok") {
                ed.ocr_state = "error";
                ed.ocr_text = helper::s(&v, "error").to_string();
                ed.ocr = text_editor::Content::with_text(&format!("Couldn't read it. {}", ed.ocr_text));
            } else {
                ed.ocr_state = "done";
                ed.ocr_text = helper::s(&v, "text").to_string();
                ed.ocr = text_editor::Content::with_text(if ed.ocr_text.is_empty() { "No text found." } else { &ed.ocr_text });
            }
        }
        EMsg::QrDone(v) => {
            if !helper::s(&v, "missing").is_empty() {
                ed.qr_state = "missing";
            } else if !helper::b(&v, "ok") {
                ed.qr_state = "error";
                ed.qr_error = helper::s(&v, "error").to_string();
            } else {
                ed.qr_state = "done";
                ed.qr_codes = v.get("codes").and_then(Value::as_array).map(|l| l.iter().filter_map(|c| c.as_str().map(String::from)).collect()).unwrap_or_default();
            }
        }
        EMsg::ToastEnd(seq) => {
            if seq == ed.toast_seq {
                ed.toast.clear();
            }
        }
    }
    Task::none()
}

// ------------------------------------------------------------------ the view
fn sep<'a>(p: &Pal) -> Element<'a, Msg> {
    container(Space::new()).width(Length::Fixed(1.0)).height(Length::Fixed(30.0)).style(boxed(p.line, 0.0, None)).into()
}

fn em(m: EMsg) -> Msg {
    Msg::Ed(m)
}

fn c8(c: u32) -> Color {
    let (r, g, b) = draw::rgb(c);
    Color::from_rgb8(r, g, b)
}

pub fn toolbar<'a>(ed: &'a Ed, p: &Pal) -> Element<'a, Msg> {
    let mut r = row![].spacing(6.0).align_y(iced::Center);
    for (t, g, name) in draw::TOOLS {
        let on = ed.tool == t;
        r = r.push(b(if on { name } else { "" }).glyph(g).selected(on).view(p, em(EMsg::Tool(t))));
    }
    r = r.push(sep(p));
    for c in draw::COLORS {
        r = r.push(b("").swatch(c8(c)).selected(ed.color == c).height(30.0).view(p, em(EMsg::Color(c))));
    }
    r = r.push(sep(p));
    for (k, t) in [('s', "S"), ('m', "M"), ('l', "L")] {
        r = r.push(b(t).selected(ed.width == k).view(p, em(EMsg::Width(k))));
    }
    r = r.push(sep(p));
    r = r.push(b("").glyph("undo").dim(!ed.undo.is_empty()).view(p, em(EMsg::Undo)));
    r = r.push(b("").glyph("redo").dim(!ed.redo.is_empty()).view(p, em(EMsg::Redo)));
    let bar = container(r).padding(Padding { top: 0.0, right: 16.0, bottom: 0.0, left: 16.0 }).center_y(Length::Fixed(59.0)).width(Length::Fill).style(boxed(p.bg2, 0.0, None));
    column![bar, ui::hline(p.line)].into()
}

/// The picture with its shapes, sized to the paper (the canvas is exactly the picture's place, so nothing is drawn
/// outside it).
struct Stage<'a> {
    ed: &'a Ed,
    pic: &'a Pic,
    view: R,
    sc: f32,
    accent: Color,
}

#[derive(Default)]
struct StageState {
    pressed: bool,
}

fn rel(cursor: mouse::Cursor, b: Rectangle) -> Option<Point> {
    cursor.position().map(|p| Point::new(p.x - b.x, p.y - b.y))
}

impl<'a> canvas::Program<Msg> for Stage<'a> {
    type State = StageState;

    fn update(&self, st: &mut StageState, event: &canvas::Event, b: Rectangle, cursor: mouse::Cursor) -> Option<canvas::Action<Msg>> {
        let to_pic = |q: Point| (self.view.x + q.x / self.sc, self.view.y + q.y / self.sc);
        match event {
            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let q = cursor.position_in(b)?;
                st.pressed = true;
                let (x, y) = to_pic(q);
                Some(canvas::Action::publish(em(EMsg::Press(x, y))).and_capture())
            }
            canvas::Event::Mouse(mouse::Event::CursorMoved { .. }) if st.pressed => {
                let q = rel(cursor, b)?;
                let (x, y) = to_pic(q);
                Some(canvas::Action::publish(em(EMsg::Move(x, y))).and_capture())
            }
            canvas::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) if st.pressed => {
                st.pressed = false;
                Some(canvas::Action::publish(em(EMsg::Release)).and_capture())
            }
            _ => None,
        }
    }

    fn draw(&self, _st: &StageState, renderer: &Renderer, _t: &Theme, b: Rectangle, _c: mouse::Cursor) -> Vec<Geometry> {
        let mut f = Frame::new(renderer, b.size());
        let (sc, v) = (self.sc, self.view);
        // The picture itself is the image under this canvas (a canvas draws its images over its shapes).
        let mut list: Vec<&Shape> = self.ed.shapes.iter().collect();
        if let Some(l) = self.ed.live.as_ref() {
            list.push(l);
        }
        // Blur and Pixelate: squares coloured from the picture's small copies over each area (Pixelate one grid of
        // 1/14-size squares; Blur bigger squares in three grids a third of a square apart, the later ones
        // see-through), as Mantel's saving canvas draws them, so the view shows what's saved.
        for sh in list.iter().filter(|s| s.hides()) {
            let r = draw::norm_rect(sh.x1, sh.y1, sh.x2, sh.y2);
            if r.w <= 1.0 || r.h <= 1.0 {
                continue;
            }
            if sh.t == T::Pix {
                squares(&mut f, &self.pic.pix, draw::PIX, r, v, sc, 0.0, 1.0);
            } else {
                squares(&mut f, &self.pic.blur, draw::BLUR, r, v, sc, 0.0, 1.0);
                squares(&mut f, &self.pic.blur, draw::BLUR, r, v, sc, 1.0 / 3.0, 0.5);
                squares(&mut f, &self.pic.blur, draw::BLUR, r, v, sc, 2.0 / 3.0, 0.34);
            }
        }
        for sh in list.iter().filter(|s| !s.hides() && s.t != T::Crop) {
            paint_shape(&mut f, sh, sc, v.x, v.y);
        }
        // Crop: outside the crop dimmed, while the crop tool is on.
        if self.ed.tool == T::Crop {
            let r = match self.ed.live.as_ref() {
                Some(l) if l.t == T::Crop => Some(draw::norm_rect(l.x1, l.y1, l.x2, l.y2)),
                _ => self.ed.crop,
            };
            let (rx, ry, rw, rh) = match r {
                Some(r) => (r.x * sc, r.y * sc, r.w * sc, r.h * sc),
                None => (0.0, 0.0, b.width, b.height),
            };
            let dim = Color { r: 0.0, g: 0.0, b: 0.0, a: 0.6 };
            f.fill_rectangle(Point::new(0.0, 0.0), Size::new(b.width, ry), dim);
            f.fill_rectangle(Point::new(0.0, ry + rh), Size::new(b.width, (b.height - ry - rh).max(0.0)), dim);
            f.fill_rectangle(Point::new(0.0, ry), Size::new(rx, rh), dim);
            f.fill_rectangle(Point::new(rx + rw, ry), Size::new((b.width - rx - rw).max(0.0), rh), dim);
            f.stroke(&Path::rectangle(Point::new(rx, ry), Size::new(rw, rh)), Stroke::default().with_color(self.accent).with_width(2.0));
        }
        vec![f.into_geometry()]
    }

    fn mouse_interaction(&self, _st: &StageState, b: Rectangle, cursor: mouse::Cursor) -> mouse::Interaction {
        if cursor.is_over(b) {
            if self.ed.tool == T::Text {
                mouse::Interaction::Text
            } else {
                mouse::Interaction::Crosshair
            }
        } else {
            mouse::Interaction::default()
        }
    }
}

/// capture-draw.js's pixelSquares for the view: the squares of picture area R (its pixels) on a grid of 1/K picture
/// pixels moved by SHIFT squares, each the colour of the small copy's pixel there, clipped to R.
fn squares(f: &mut Frame, small: &iced::widget::image::Handle, k: f32, r: R, v: R, sc: f32, shift: f32, alpha: f32) {
    let iced::widget::image::Handle::Rgba { width, height, pixels, .. } = small else { return };
    let (sw, sh) = (*width as i64, *height as i64);
    if sw == 0 || sh == 0 {
        return;
    }
    let side = 1.0 / k;
    let (x0, y0, x1, y1) = (r.x, r.y, r.x + r.w, r.y + r.h);
    let mut ty = (y0 * k - shift).floor() as i64;
    while (ty as f32) < y1 * k - shift {
        let mut tx = (x0 * k - shift).floor() as i64;
        while (tx as f32) < x1 * k - shift {
            let gx = (tx as f32 + shift) * side;
            let gy = (ty as f32 + shift) * side;
            let (cx0, cy0) = (gx.max(x0), gy.max(y0));
            let (cx1, cy1) = ((gx + side).min(x1), (gy + side).min(y1));
            if cx1 > cx0 && cy1 > cy0 {
                let px = ((gx + side / 2.0) * k).floor().clamp(0.0, (sw - 1) as f32) as i64;
                let py = ((gy + side / 2.0) * k).floor().clamp(0.0, (sh - 1) as f32) as i64;
                let i = ((py * sw + px) * 4) as usize;
                if i + 3 < pixels.len() {
                    let c = Color::from_rgba8(pixels[i], pixels[i + 1], pixels[i + 2], alpha * pixels[i + 3] as f32 / 255.0);
                    f.fill_rectangle(Point::new((cx0 - v.x) * sc, (cy0 - v.y) * sc), Size::new((cx1 - cx0) * sc + 0.6, (cy1 - cy0) * sc + 0.6), c);
                }
            }
            tx += 1;
        }
        ty += 1;
    }
}

/// capture-draw.js's paint() for the view: SH at scale S with picture pixel (OX, OY) at the top left.
pub fn paint_shape(f: &mut Frame, sh: &Shape, s: f32, ox: f32, oy: f32) {
    let x = |v: f32| (v - ox) * s;
    let y = |v: f32| (v - oy) * s;
    let c = c8(sh.c);
    let lw = (sh.w * s).max(1.0);
    let st = |w: f32, col: Color, cap: LineCap| Stroke::default().with_color(col).with_width(w).with_line_cap(cap).with_line_join(LineJoin::Round);
    match sh.t {
        T::Arrow => {
            let hd = draw::arrow_head(x(sh.x1), y(sh.y1), x(sh.x2), y(sh.y2), (lw * 3.2).max(10.0));
            let (bx, by) = ((hd[0] + hd[2]) / 2.0, (hd[1] + hd[3]) / 2.0);
            f.stroke(&Path::line(Point::new(x(sh.x1), y(sh.y1)), Point::new(bx, by)), st(lw, c, LineCap::Round));
            let head = Path::new(|p| {
                p.move_to(Point::new(x(sh.x2), y(sh.y2)));
                p.line_to(Point::new(hd[0], hd[1]));
                p.line_to(Point::new(hd[2], hd[3]));
                p.close();
            });
            f.fill(&head, c);
        }
        T::Box => {
            let r = draw::norm_rect(x(sh.x1), y(sh.y1), x(sh.x2), y(sh.y2));
            f.stroke(&Path::rectangle(Point::new(r.x, r.y), Size::new(r.w, r.h)), st(lw, c, LineCap::Round));
        }
        T::Pen | T::Hl => {
            let hl = sh.t == T::Hl;
            let p = &sh.pts;
            if p.len() >= 2 {
                let path = Path::new(|b| {
                    b.move_to(Point::new(x(p[0]), y(p[1])));
                    let mut i = 2;
                    while i + 1 < p.len() {
                        b.line_to(Point::new(x(p[i]), y(p[i + 1])));
                        i += 2;
                    }
                    if p.len() == 2 {
                        b.line_to(Point::new(x(p[0]) + 0.1, y(p[1])));
                    }
                });
                if hl {
                    f.stroke(&path, st(lw * 3.5, Color { a: 0.38, ..c }, LineCap::Square));
                } else {
                    f.stroke(&path, st(lw, c, LineCap::Round));
                }
            }
        }
        T::Text => {
            let size = (sh.size * s).max(6.0);
            let ow = (size / 7.0).max(2.0) / 2.0;
            let outline = if sh.c == 0x111111 { Color::WHITE } else { Color { r: 0.0, g: 0.0, b: 0.0, a: 0.65 } };
            let font = Font { weight: iced::font::Weight::Semibold, ..Font::DEFAULT };
            for (li, line) in sh.text.split('\n').enumerate() {
                let pos = Point::new(x(sh.x1), y(sh.y1) + li as f32 * size * 1.2);
                let txt = |at: Point, col: Color| canvas::Text { content: line.to_string(), position: at, color: col, size: Pixels(size), font, ..canvas::Text::default() };
                for (dx, dy) in [(-1.0, 0.0), (1.0, 0.0), (0.0, -1.0), (0.0, 1.0), (-0.7, -0.7), (0.7, -0.7), (-0.7, 0.7), (0.7, 0.7)] {
                    f.fill_text(txt(Point::new(pos.x + dx * ow, pos.y + dy * ow), outline));
                }
                f.fill_text(txt(pos, c));
            }
        }
        T::Step => {
            let rad = (16.0 * s * ((sh.w / 6.0) + 0.6) / 1.6).max(8.0);
            let center = Point::new(x(sh.x1), y(sh.y1));
            let disc = Path::circle(center, rad);
            f.fill(&disc, c);
            f.stroke(&disc, st((rad / 7.0).max(1.5), Color::WHITE, LineCap::Round));
            let ink = if sh.c == 0xffffff || sh.c == 0xffd166 { c8(0x111111) } else { Color::WHITE };
            f.fill_text(canvas::Text {
                content: sh.n.to_string(),
                position: Point::new(center.x, center.y + rad * 0.05),
                color: ink,
                size: Pixels((rad * 1.15).round()),
                font: Font { weight: iced::font::Weight::Bold, ..Font::DEFAULT },
                align_x: iced::widget::text::Alignment::Center,
                align_y: iced::alignment::Vertical::Center,
                ..canvas::Text::default()
            });
        }
        _ => {}
    }
}

fn pill<'a>(p: &Pal, inner: Element<'a, Msg>, h: f32) -> Element<'a, Msg> {
    container(inner)
        .padding(Padding { top: 0.0, right: 16.0, bottom: 0.0, left: 16.0 })
        .center_y(Length::Fixed(h))
        .style(boxed(Pal::a(p.panel, 0.96), h / 2.0, Some((p.line, 1.0))))
        .into()
}

pub fn stage<'a>(ed: &'a Ed, p: &Pal) -> Element<'a, Msg> {
    let p = *p;
    let body: Element<'a, Msg> = match ed.pic.as_ref() {
        None => container(label(if ed.load_error.is_empty() { "Opening the picture…".to_string() } else { format!("Couldn't open the picture: {}", ed.load_error) }, 14.0, p.muted, false))
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into(),
        Some(pic) => responsive(move |size: Size| {
            let v = ed.view_rect();
            let sc = draw::fit(size.width - 32.0, size.height - 32.0, v.w, v.h);
            let (pw, ph) = ((v.w * sc).max(1.0), (v.h * sc).max(1.0));
            let (ox, oy) = (((size.width - pw) / 2.0).max(0.0), ((size.height - ph) / 2.0).max(0.0));
            let cv = canvas::Canvas::new(Stage { ed, pic, view: v, sc, accent: p.accent }).width(Length::Fixed(pw)).height(Length::Fixed(ph));
            let shown = match ed.view_img.as_ref() {
                Some((r, h)) if *r == v => h.clone(),
                _ => pic.handle.clone(),
            };
            let base = iced::widget::image(shown).content_fit(iced::ContentFit::Fill).filter_method(FilterMethod::Linear).width(Length::Fixed(pw)).height(Length::Fixed(ph));
            let cv = stack![base, cv];
            let paper = container(container(cv).padding(1.0).style(boxed(Color::TRANSPARENT, 0.0, Some((p.line, 1.0))))).center_x(Length::Fill).center_y(Length::Fill);
            let mut layers = stack![paper].width(Length::Fill).height(Length::Fill);
            // Typing a text: right where you clicked.
            if ed.editing_text {
                let ts = (ed.text_size() * sc).max(10.0);
                let field = text_input("", &ed.text_in)
                    .id(TEXT_FIELD)
                    .on_input(|s| em(EMsg::TextIn(s)))
                    .on_submit(em(EMsg::TextCommit))
                    .size(ts)
                    .font(Font { weight: iced::font::Weight::Semibold, ..Font::DEFAULT })
                    .padding(4.0)
                    .width(Length::Fixed(((ed.text_in.chars().count() as f32 + 2.0) * ts * 0.6).max(160.0)))
                    .style(move |_: &Theme, _s: text_input::Status| text_input::Style {
                        background: Background::Color(Color { r: 0.0, g: 0.0, b: 0.0, a: 0.4 }),
                        border: Border { color: p.accent, width: 2.0, radius: 4.0.into() },
                        icon: p.text,
                        placeholder: p.muted,
                        value: c8(ed.color),
                        selection: Pal::a(p.accent, 0.4),
                    });
                let tx = ox + 1.0 + (ed.text_x - v.x) * sc - 4.0;
                let ty = oy + 1.0 + (ed.text_y - v.y) * sc - 4.0;
                layers = layers.push(pin(field).x(tx.max(0.0)).y(ty.max(0.0)));
            }
            if ed.tool == T::Crop {
                let tip = format!("Drag the part to keep{}", ed.crop.map(|c| format!("  ·  {} × {}", c.w, c.h)).unwrap_or_default());
                let mut r = row![pill(&p, label(tip, 13.0, p.text, false).into(), 38.0)].spacing(8.0).align_y(iced::Center);
                if ed.crop.is_some() {
                    r = r.push(b("All of it").view(&p, em(EMsg::CropAll)));
                }
                r = r.push(b("Done").primary().view(&p, em(EMsg::CropDone)));
                layers = layers.push(column![Space::new().height(Length::Fill), container(r).center_x(Length::Fill), gap(1.0, 14.0)]);
            }
            if !ed.toast.is_empty() {
                let tt = pill(&p, crate::btn::label(ed.toast.clone(), 14.0, p.text, false).font(ui::weight(iced::font::Weight::Medium)).into(), 40.0);
                layers = layers.push(column![gap(1.0, 14.0), container(tt).center_x(Length::Fill)]);
            }
            layers.into()
        })
        .into(),
    };
    container(body).width(Length::Fill).height(Length::Fill).style(boxed(p.bg, 0.0, None)).into()
}

pub fn side<'a>(ed: &'a Ed, p: &Pal, tools: &'a Value, note: &'a str, shot: &'a str) -> Element<'a, Msg> {
    let p = *p;
    let muted = |s: String| -> Element<'a, Msg> { label(s, 12.0, p.muted, false).into() };
    let mut col = Column::new().spacing(8.0).width(Length::Fill);
    col = col.push(b(if ed.busy { "Working…" } else { "Save and copy" }).glyph("check").big().primary().fill().view(&p, em(EMsg::SaveCopy)));
    col = col.push(row![b("Copy").glyph("copy").fill().view(&p, em(EMsg::Copy)), b("Save as…").glyph("doc").fill().view(&p, em(EMsg::SaveAsOpen))].spacing(8.0));
    col = col.push(row![
        b("Show in folder").glyph("folder").fill().view(&p, em(EMsg::ShowFolder)),
        b("Share…").glyph("share").selected(ed.panel == Panel::Share).fill().view(&p, em(EMsg::Share))
    ]
    .spacing(8.0));
    col = col.push(column![gap(1.0, 4.0), ui::hline(p.line)]);
    col = col.push(row![
        b("Read text").glyph("scan").selected(ed.panel == Panel::Text).dim(helper::b(tools, "tesseract")).fill().view(&p, em(EMsg::ReadText)),
        b("QR code").glyph("qr").selected(ed.panel == Panel::Qr).dim(helper::b(tools, "zbar")).fill().view(&p, em(EMsg::ReadQr))
    ]
    .spacing(8.0));
    col = col.push(b("New screenshot").glyph("area").fill().view(&p, em(EMsg::NewShot)));
    let head = |t: &'static str| -> Element<'a, Msg> { label(t, 15.0, p.text, true).into() };
    match ed.panel {
        Panel::Share => {
            let mut s = Column::new().spacing(8.0).width(Length::Fill);
            s = s.push(row![container(head("Share")).width(Length::Fill), b("").glyph("close").view(&p, em(EMsg::PanelClose))].align_y(iced::Center));
            s = s.push(row![b("Copy").glyph("copy").fill().view(&p, em(EMsg::Copy)), b("Save").glyph("check").fill().view(&p, em(EMsg::SaveCopy))].spacing(8.0));
            s = s.push(muted("Open with".into()));
            let mut apps = Column::new().spacing(6.0).width(Length::Fill);
            for (i, app) in ed.share_apps.iter().enumerate() {
                let t = format!("{}{}", app.name, if app.default { "  ·  default" } else { "" });
                apps = apps.push(b(t).fill().view(&p, em(EMsg::OpenWith(i))));
            }
            if ed.share_apps.is_empty() {
                apps = apps.push(muted("No app for pictures found.".into()));
            }
            s = s.push(scrollable(apps).height(Length::Shrink).height(Length::Fixed((ed.share_apps.len().max(1) as f32 * 44.0).min(150.0))));
            s = s.push(muted("Send to your phone".into()));
            let on = ed.share_phones.as_ref().map(|v| helper::b(v, "on")).unwrap_or(false);
            let phones: Vec<Value> = ed.share_phones.as_ref().and_then(|v| v.get("phones")).and_then(Value::as_array).cloned().unwrap_or_default();
            if on {
                for (i, ph) in phones.iter().enumerate() {
                    s = s.push(b(format!("Send to {}", helper::s(ph, "name"))).glyph("phone").fill().view(&p, em(EMsg::SendPhone(i))));
                }
            }
            if !on || phones.is_empty() {
                let t = if ed.share_phones.is_none() {
                    "Looking for your phone…"
                } else if !on {
                    "Phone link is off (Settings › Phone)."
                } else {
                    "No phone connected right now."
                };
                s = s.push(muted(t.into()));
            }
            col = col.push(column![gap(1.0, 8.0), s]);
        }
        Panel::SaveAs => {
            let field = text_input("", &ed.save_as_path)
                .id(SAVE_FIELD)
                .on_input(|s| em(EMsg::SaveAsPath(s)))
                .on_submit(em(EMsg::SaveAs(false)))
                .size(13.0)
                .padding(8.0)
                .font(hearth_ui::app::INTER)
                .style(move |_: &Theme, st: text_input::Status| text_input::Style {
                    background: Background::Color(p.bg2),
                    border: Border { color: if matches!(st, text_input::Status::Focused { .. }) { p.accent } else { p.line }, width: if matches!(st, text_input::Status::Focused { .. }) { 2.0 } else { 1.0 }, radius: 8.0.into() },
                    icon: p.text,
                    placeholder: p.muted,
                    value: p.text,
                    selection: Pal::a(p.accent, 0.35),
                });
            let mut s = Column::new().spacing(8.0).width(Length::Fill);
            s = s.push(head("Save as"));
            s = s.push(field);
            if ed.save_as_exists {
                s = s.push(label("There's a file with that name. Replace it?", 12.0, p.danger, false));
            }
            s = s.push(row![
                b("").glyph("kbd").view(&p, em(EMsg::Kbd)),
                Space::new().width(Length::Fill),
                b("Cancel").view(&p, em(EMsg::PanelClose)),
                b(if ed.save_as_exists { "Replace" } else { "Save" }).primary().view(&p, em(EMsg::SaveAs(ed.save_as_exists)))
            ]
            .spacing(8.0)
            .align_y(iced::Center));
            col = col.push(column![gap(1.0, 8.0), s]);
        }
        Panel::Text => {
            let color = if ed.ocr_state == "done" && !ed.ocr_text.is_empty() { p.text } else { p.muted };
            let box_ = container(
                text_editor(&ed.ocr)
                    .on_action(|act| em(EMsg::OcrAction(act)))
                    .size(13.0)
                    .font(hearth_ui::app::INTER)
                    .height(Length::Fill)
                    .style(move |_: &Theme, _s: text_editor::Status| text_editor::Style {
                        background: Background::Color(Color::TRANSPARENT),
                        border: Border::default(),
                        placeholder: p.muted,
                        value: color,
                        selection: Pal::a(p.accent, 0.35),
                    }),
            )
            .padding(4.0)
            .height(Length::Fill)
            .style(boxed(p.bg2, 8.0, Some((p.line, 1.0))));
            let mut s = Column::new().spacing(8.0).width(Length::Fill).height(Length::Fill);
            s = s.push(head("Text in the picture"));
            s = s.push(box_);
            if ed.ocr_state == "done" && !ed.ocr_text.is_empty() {
                s = s.push(b("Copy the text").glyph("copy").fill().view(&p, em(EMsg::CopyText)));
            }
            col = col.push(column![gap(1.0, 8.0), s].height(Length::Fill));
        }
        Panel::Qr => {
            let mut s = Column::new().spacing(8.0).width(Length::Fill);
            s = s.push(head("QR codes"));
            if ed.qr_state != "done" || ed.qr_codes.is_empty() {
                let t = match ed.qr_state {
                    "busy" => "Looking…".to_string(),
                    "missing" => "Reading QR codes needs zbarimg (zbar-tools), which isn't installed on this PC.".to_string(),
                    "error" => format!("Couldn't read it. {}", ed.qr_error),
                    _ => "No QR code in the picture.".to_string(),
                };
                s = s.push(label(t, 13.0, p.muted, false));
            }
            if ed.qr_state == "done" {
                for (i, code) in ed.qr_codes.iter().enumerate() {
                    let mut btns = row![b("Copy").glyph("copy").view(&p, em(EMsg::QrCopy(i)))].spacing(8.0);
                    let l = code.to_lowercase();
                    if l.starts_with("http://") || l.starts_with("https://") || l.starts_with("mailto:") || l.starts_with("tel:") {
                        btns = btns.push(b("Open").glyph("open").view(&p, em(EMsg::QrOpen(i))));
                    }
                    s = s.push(container(column![label(code.clone(), 13.0, p.text, false), btns].spacing(8.0)).padding(10.0).width(Length::Fill).style(boxed(p.bg2, 8.0, Some((p.line, 1.0)))));
                }
            }
            col = col.push(column![gap(1.0, 8.0), s]);
        }
        Panel::None => {}
    }
    if ed.panel != Panel::Text {
        col = col.push(Space::new().height(Length::Fill));
    }
    let mut info = String::new();
    if ed.img_w() > 0.0 {
        info = match ed.crop {
            Some(c) => format!("{} × {} pixels", c.w, c.h),
            None => format!("{} × {} pixels", ed.img_w(), ed.img_h()),
        };
    }
    if !ed.last_saved.is_empty() {
        info.push_str(&format!("\nSaved: {}", ed.last_saved.rsplit('/').next().unwrap_or("")));
    }
    if !note.is_empty() && shot.contains("/scroll-") {
        info.push_str(&format!("\n{note}"));
    }
    col = col.push(muted(info));
    let panel = container(col.height(Length::Fill)).padding(16.0).width(Length::Fixed(299.0)).height(Length::Fill).style(boxed(p.panel, 0.0, None));
    row![ui::vline(p.line), panel].height(Length::Fill).into()
}

pub fn view<'a>(ed: &'a Ed, p: &Pal, tools: &'a Value, note: &'a str, shot: &'a str) -> Element<'a, Msg> {
    column![toolbar(ed, p), row![stage(ed, p), side(ed, p, tools, note, shot)].height(Length::Fill)].into()
}
