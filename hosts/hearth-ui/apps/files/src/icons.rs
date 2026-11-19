//! Hearth Files' icons, drawn (no icon theme, no emoji font needed): the coloured ones the approved mockups show for
//! places and files (a yellow folder, a page, the house, the drive, the plug, the globe, the penguin, the red pin…)
//! and the thin line ones of the toolbar and the menus (cut, copy, paste, rename, share, delete, sort, view…).
//! Each is drawn in a 24 × 24 box and scaled; a canvas draws only when it changes.

use iced::widget::canvas::{self, Frame, Geometry, LineCap, LineJoin, Path, Stroke};
use iced::{mouse, Color, Element, Length, Point, Rectangle, Renderer, Size, Theme};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Ic {
    // coloured
    Folder,
    FolderOpen,
    File,
    FileText,
    FileImage,
    FileVideo,
    FileAudio,
    FileArchive,
    FilePdf,
    FileApp,
    FileDisc,
    Home,
    Gallery,
    Cloud,
    Desktop,
    Download,
    Document,
    Music,
    Video,
    Games,
    Laptop,
    Disc,
    Usb,
    Globe,
    Penguin,
    Pin,
    TrashBin,
    Server,
    Phone,
    Star,
    Lock,
    Clock,
    Link,
    Info,
    Keyboard,
    Zip,
    OpenWith,
    NewTab,
    Apps,
    Briefcase,
    Image,
    // line (drawn in the colour given)
    Back,
    Forward,
    Up,
    Refresh,
    Plus,
    Cut,
    Copy,
    Paste,
    Rename,
    Share,
    Delete,
    Sort,
    ViewList,
    Pane,
    Dots,
    ChevDown,
    ChevRight,
    Check,
    Close,
    Search,
    Grid,
    ListRows,
    Compact,
    Minimize,
    Maximize,
    Undo,
    Eye,
    Terminal,
    Split,
}

struct Draw {
    ic: Ic,
    c: Color,
}

fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::from_rgb8(r, g, b)
}

const YELLOW: (u8, u8, u8) = (0xe8, 0xb3, 0x3e);
const YELLOW_LT: (u8, u8, u8) = (0xf6, 0xcd, 0x5f);
const PAGE: (u8, u8, u8) = (0xe6, 0xe8, 0xee);
const PAGE_DK: (u8, u8, u8) = (0xb9, 0xbe, 0xca);

fn c3(t: (u8, u8, u8)) -> Color {
    rgb(t.0, t.1, t.2)
}

struct P<'a> {
    f: &'a mut Frame,
    k: f32,
}

impl P<'_> {
    fn pt(&self, x: f32, y: f32) -> Point {
        Point::new(x * self.k, y * self.k)
    }
    fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, r: f32, c: Color) {
        let k = self.k;
        let path = Path::rounded_rectangle(Point::new(x * k, y * k), Size::new(w * k, h * k), (r * k).into());
        self.f.fill(&path, c);
    }
    fn circle(&mut self, x: f32, y: f32, r: f32, c: Color) {
        let path = Path::circle(self.pt(x, y), r * self.k);
        self.f.fill(&path, c);
    }
    fn ring(&mut self, x: f32, y: f32, r: f32, w: f32, c: Color) {
        let path = Path::circle(self.pt(x, y), r * self.k);
        self.f.stroke(&path, Stroke::default().with_color(c).with_width(w * self.k));
    }
    fn poly(&mut self, pts: &[(f32, f32)], c: Color) {
        if pts.len() < 3 {
            return;
        }
        let k = self.k;
        let path = Path::new(|b| {
            b.move_to(Point::new(pts[0].0 * k, pts[0].1 * k));
            for p in &pts[1..] {
                b.line_to(Point::new(p.0 * k, p.1 * k));
            }
            b.close();
        });
        self.f.fill(&path, c);
    }
    fn line(&mut self, pts: &[(f32, f32)], w: f32, c: Color) {
        if pts.len() < 2 {
            return;
        }
        let k = self.k;
        let path = Path::new(|b| {
            b.move_to(Point::new(pts[0].0 * k, pts[0].1 * k));
            for p in &pts[1..] {
                b.line_to(Point::new(p.0 * k, p.1 * k));
            }
        });
        self.f.stroke(&path, Stroke::default().with_color(c).with_width((w * self.k).max(0.8)).with_line_cap(LineCap::Round).with_line_join(LineJoin::Round));
    }
    fn arc(&mut self, cx: f32, cy: f32, r: f32, a0: f32, a1: f32, w: f32, c: Color) {
        let n = 18;
        let pts: Vec<(f32, f32)> = (0..=n)
            .map(|i| {
                let a = (a0 + (a1 - a0) * i as f32 / n as f32).to_radians();
                (cx + r * a.cos(), cy + r * a.sin())
            })
            .collect();
        self.line(&pts, w, c);
    }
    fn page(&mut self, band: Option<Color>) {
        self.poly(&[(5.0, 2.5), (14.5, 2.5), (19.5, 7.5), (19.5, 21.5), (5.0, 21.5)], c3(PAGE));
        self.poly(&[(14.5, 2.5), (19.5, 7.5), (14.5, 7.5)], c3(PAGE_DK));
        if let Some(b) = band {
            self.rect(5.0, 15.0, 14.5, 4.0, 0.0, b);
        }
    }
}

/// An icon's drawing kept between frames (scrolling a long folder redraws every frame); drawn again only when the
/// icon, its colour or its size changes.
#[derive(Default)]
pub struct DrawCache {
    cache: canvas::Cache,
    key: std::cell::Cell<Option<(Ic, Color, Size)>>,
}

impl<M> canvas::Program<M> for Draw {
    type State = DrawCache;
    fn draw(&self, st: &DrawCache, renderer: &Renderer, _t: &Theme, bounds: Rectangle, _c: mouse::Cursor) -> Vec<Geometry> {
        let key = Some((self.ic, self.c, bounds.size()));
        if st.key.get() != key {
            st.cache.clear();
            st.key.set(key);
        }
        vec![st.cache.draw(renderer, bounds.size(), |f| self.paint(f, bounds))]
    }
}

impl Draw {
    fn paint(&self, f: &mut Frame, bounds: Rectangle) {
        let k = bounds.width.min(bounds.height) / 24.0;
        let c = self.c;
        let mut p = P { f, k };
        let lw = 1.7;
        match self.ic {
            Ic::Folder => {
                p.rect(2.0, 5.0, 8.5, 4.0, 1.2, c3(YELLOW));
                p.rect(2.0, 7.0, 20.0, 13.0, 1.6, c3(YELLOW));
                p.rect(2.0, 9.0, 20.0, 11.0, 1.6, c3(YELLOW_LT));
            }
            Ic::FolderOpen => {
                p.rect(2.0, 5.0, 8.5, 4.0, 1.2, c3(YELLOW));
                p.rect(2.0, 7.0, 18.0, 13.0, 1.6, c3(YELLOW));
                p.poly(&[(5.0, 11.0), (22.5, 11.0), (19.5, 20.0), (2.0, 20.0)], c3(YELLOW_LT));
            }
            Ic::File | Ic::FileApp => {
                p.page(None);
                if self.ic == Ic::FileApp {
                    p.rect(8.0, 11.0, 8.5, 7.0, 1.0, rgb(0x6d, 0x8c, 0xff));
                }
            }
            Ic::FileText | Ic::Document => {
                p.page(None);
                for y in [10.5f32, 13.5, 16.5] {
                    p.line(&[(8.0, y), (16.5, y)], 1.3, rgb(0x6d, 0x8c, 0xd8));
                }
            }
            Ic::FilePdf => p.page(Some(rgb(0xd9, 0x48, 0x3b))),
            Ic::FileArchive | Ic::Zip => {
                p.page(None);
                for (i, y) in [4.0f32, 7.0, 10.0, 13.0].iter().enumerate() {
                    p.rect(if i % 2 == 0 { 11.0 } else { 12.5 }, *y, 1.5, 2.0, 0.0, rgb(0x7a, 0x80, 0x90));
                }
                p.rect(10.5, 15.0, 4.5, 4.0, 1.0, rgb(0x7a, 0x80, 0x90));
            }
            Ic::FileDisc => {
                p.page(None);
                p.circle(12.2, 14.0, 4.5, rgb(0x9a, 0xa4, 0xb8));
                p.circle(12.2, 14.0, 1.3, c3(PAGE));
            }
            Ic::FileImage | Ic::Gallery | Ic::Image => {
                p.rect(3.0, 4.5, 18.0, 15.0, 2.0, rgb(0xd5, 0xd9, 0xe2));
                p.rect(4.5, 6.0, 15.0, 12.0, 1.0, rgb(0x5b, 0x88, 0xc9));
                p.poly(&[(4.5, 18.0), (9.5, 11.0), (13.0, 15.0), (15.5, 12.5), (19.5, 18.0)], rgb(0x4c, 0xa0, 0x6a));
                p.circle(15.5, 9.0, 1.5, rgb(0xf6, 0xd3, 0x6b));
            }
            Ic::FileVideo | Ic::Video => {
                p.rect(3.0, 8.0, 18.0, 12.0, 1.5, rgb(0x52, 0x58, 0x68));
                p.poly(&[(3.0, 4.5), (20.5, 3.0), (21.0, 6.5), (3.5, 8.0)], rgb(0xd5, 0xd9, 0xe2));
                for x in [6.5f32, 11.0, 15.5] {
                    p.poly(&[(x, 4.3), (x + 2.0, 4.1), (x + 0.8, 7.3), (x - 1.2, 7.5)], rgb(0x52, 0x58, 0x68));
                }
                p.poly(&[(10.0, 11.0), (15.5, 14.0), (10.0, 17.0)], rgb(0xe6, 0xe8, 0xee));
            }
            Ic::FileAudio | Ic::Music => {
                let col = rgb(0x8f, 0xb4, 0xff);
                p.line(&[(9.5, 17.5), (9.5, 5.0), (19.0, 3.0), (19.0, 15.5)], 2.0, col);
                p.circle(7.3, 17.8, 2.8, col);
                p.circle(16.8, 15.8, 2.8, col);
            }
            Ic::Home => {
                p.poly(&[(2.5, 12.0), (12.0, 3.0), (21.5, 12.0)], rgb(0xe2, 0x5d, 0x3c));
                p.rect(5.0, 11.0, 14.0, 10.0, 0.5, rgb(0xf2, 0xb2, 0x56));
                p.rect(10.0, 14.5, 4.0, 6.5, 0.5, rgb(0x8a, 0x4b, 0x2a));
            }
            Ic::Cloud => {
                let col = rgb(0xe8, 0xec, 0xf4);
                p.circle(8.5, 14.0, 4.5, col);
                p.circle(13.5, 11.0, 5.5, col);
                p.circle(17.5, 14.5, 4.0, col);
                p.rect(8.5, 13.5, 9.0, 5.0, 0.0, col);
            }
            Ic::Desktop => {
                p.rect(2.5, 4.0, 19.0, 13.0, 1.5, rgb(0xb8, 0xbe, 0xcc));
                p.rect(4.0, 5.5, 16.0, 10.0, 0.5, rgb(0x3c, 0x4a, 0x66));
                p.rect(10.0, 17.0, 4.0, 2.5, 0.0, rgb(0xb8, 0xbe, 0xcc));
                p.rect(7.0, 19.5, 10.0, 1.8, 0.8, rgb(0xb8, 0xbe, 0xcc));
            }
            Ic::Download => {
                let col = rgb(0xe8, 0xec, 0xf4);
                p.rect(10.0, 3.0, 4.0, 10.0, 0.5, col);
                p.poly(&[(5.5, 11.5), (18.5, 11.5), (12.0, 18.0)], col);
                p.rect(4.0, 19.5, 16.0, 2.0, 1.0, col);
            }
            Ic::Games => {
                let col = rgb(0xb8, 0xbe, 0xcc);
                p.rect(2.5, 7.5, 19.0, 10.0, 5.0, col);
                p.rect(5.5, 11.5, 5.0, 1.8, 0.4, rgb(0x2a, 0x2e, 0x38));
                p.rect(7.1, 9.9, 1.8, 5.0, 0.4, rgb(0x2a, 0x2e, 0x38));
                p.circle(16.0, 11.0, 1.2, rgb(0xd9, 0x48, 0x3b));
                p.circle(18.0, 13.3, 1.2, rgb(0x4c, 0xa0, 0x6a));
            }
            Ic::Laptop => {
                p.rect(4.5, 5.0, 15.0, 10.5, 1.2, rgb(0xb8, 0xbe, 0xcc));
                p.rect(5.8, 6.3, 12.4, 8.0, 0.4, rgb(0x4d, 0x7c, 0xd6));
                p.poly(&[(2.0, 17.0), (22.0, 17.0), (20.5, 19.5), (3.5, 19.5)], rgb(0x9a, 0xa1, 0xb2));
            }
            Ic::Disc => {
                p.circle(12.0, 12.0, 9.5, rgb(0xd4, 0xda, 0xe4));
                p.circle(12.0, 12.0, 7.0, rgb(0xb4, 0xd8, 0xf0));
                p.poly(&[(12.0, 12.0), (12.0, 2.5), (19.0, 5.5)], rgb(0xe8, 0xd0, 0xf2));
                p.circle(12.0, 12.0, 3.2, rgb(0xd4, 0xda, 0xe4));
                p.circle(12.0, 12.0, 1.4, rgb(0x55, 0x5b, 0x6a));
            }
            Ic::Usb => {
                let col = rgb(0xb8, 0xbe, 0xcc);
                p.poly(&[(10.0, 7.0), (17.0, 14.0), (12.0, 19.0), (5.0, 12.0)], col);
                p.line(&[(13.5, 7.5), (16.5, 4.5)], 2.2, col);
                p.line(&[(16.5, 10.5), (19.5, 7.5)], 2.2, col);
                p.line(&[(7.0, 17.0), (3.5, 20.5)], 2.6, rgb(0x70, 0x76, 0x86));
            }
            Ic::Globe => {
                let col = rgb(0x4d, 0x9a, 0xe8);
                p.circle(12.0, 12.0, 9.5, col);
                let ln = rgb(0xc8, 0xe4, 0xff);
                p.ring(12.0, 12.0, 9.5, 1.0, ln);
                p.line(&[(2.5, 12.0), (21.5, 12.0)], 1.0, ln);
                p.line(&[(12.0, 2.5), (12.0, 21.5)], 1.0, ln);
                p.arc(12.0, 12.0, 9.5, -60.0, 60.0, 1.0, ln);
                p.line(&[(4.0, 7.0), (20.0, 7.0)], 0.9, ln);
                p.line(&[(4.0, 17.0), (20.0, 17.0)], 0.9, ln);
            }
            Ic::Penguin => {
                p.poly(&[(12.0, 2.5), (17.0, 7.0), (19.0, 16.0), (16.0, 20.0), (8.0, 20.0), (5.0, 16.0), (7.0, 7.0)], rgb(0x22, 0x24, 0x2a));
                p.poly(&[(12.0, 8.5), (15.5, 11.0), (16.0, 17.0), (12.0, 19.5), (8.0, 17.0), (8.5, 11.0)], rgb(0xf2, 0xf2, 0xf2));
                p.circle(10.3, 6.5, 0.9, rgb(0xf2, 0xf2, 0xf2));
                p.circle(13.7, 6.5, 0.9, rgb(0xf2, 0xf2, 0xf2));
                p.poly(&[(10.8, 8.0), (13.2, 8.0), (12.0, 9.6)], rgb(0xf0, 0xa8, 0x30));
                p.poly(&[(6.0, 19.0), (10.5, 19.0), (9.5, 21.5), (5.0, 21.5)], rgb(0xf0, 0xa8, 0x30));
                p.poly(&[(13.5, 19.0), (18.0, 19.0), (19.0, 21.5), (14.5, 21.5)], rgb(0xf0, 0xa8, 0x30));
            }
            Ic::Pin => {
                let red = rgb(0xe0, 0x45, 0x3c);
                p.poly(&[(13.0, 3.0), (21.0, 11.0), (18.5, 12.0), (14.0, 16.5), (14.5, 19.5), (12.5, 19.5), (4.5, 11.5), (4.5, 9.5), (7.5, 10.0), (12.0, 5.5)], red);
                p.line(&[(8.5, 15.5), (3.5, 20.5)], 1.6, rgb(0xb0, 0xb6, 0xc2));
            }
            Ic::TrashBin => {
                let col = rgb(0xb8, 0xbe, 0xcc);
                p.rect(4.0, 5.0, 16.0, 2.2, 1.0, col);
                p.rect(9.5, 3.0, 5.0, 2.5, 0.8, col);
                p.poly(&[(5.5, 8.0), (18.5, 8.0), (17.2, 21.0), (6.8, 21.0)], col);
                for x in [9.5f32, 12.0, 14.5] {
                    p.line(&[(x, 10.5), (x, 18.5)], 1.1, rgb(0x70, 0x76, 0x86));
                }
            }
            Ic::Server => {
                let col = rgb(0xd4, 0xda, 0xe4);
                p.rect(9.0, 3.0, 6.0, 5.0, 1.0, col);
                p.rect(2.5, 15.0, 6.0, 5.0, 1.0, col);
                p.rect(15.5, 15.0, 6.0, 5.0, 1.0, col);
                p.line(&[(12.0, 8.0), (12.0, 12.0), (5.5, 12.0), (5.5, 15.0)], 1.5, col);
                p.line(&[(12.0, 12.0), (18.5, 12.0), (18.5, 15.0)], 1.5, col);
            }
            Ic::Phone => {
                p.rect(6.5, 2.0, 11.0, 20.0, 2.0, rgb(0x2a, 0x2e, 0x38));
                p.rect(7.7, 4.0, 8.6, 15.5, 0.5, rgb(0x4d, 0x7c, 0xd6));
                for (i, cc) in [rgb(0xe0, 0x45, 0x3c), rgb(0xf6, 0xd3, 0x6b), rgb(0x4c, 0xa0, 0x6a), rgb(0x8f, 0xb4, 0xff)].iter().enumerate() {
                    p.rect(8.8 + (i % 2) as f32 * 3.6, 5.5 + (i / 2) as f32 * 3.3, 2.6, 2.4, 0.6, *cc);
                }
            }
            Ic::Star => {
                let mut pts = Vec::new();
                for i in 0..10 {
                    let a = (-90.0 + 36.0 * i as f32).to_radians();
                    let r = if i % 2 == 0 { 9.5 } else { 4.2 };
                    pts.push((12.0 + r * a.cos(), 12.5 + r * a.sin()));
                }
                p.poly(&pts, rgb(0xf6, 0xc3, 0x3e));
            }
            Ic::Lock => {
                p.arc(12.0, 9.5, 4.5, 180.0, 360.0, 2.2, rgb(0xb8, 0xbe, 0xcc));
                p.line(&[(7.5, 9.5), (7.5, 11.0)], 2.2, rgb(0xb8, 0xbe, 0xcc));
                p.line(&[(16.5, 9.5), (16.5, 11.0)], 2.2, rgb(0xb8, 0xbe, 0xcc));
                p.rect(5.0, 10.5, 14.0, 10.5, 2.0, rgb(0xf0, 0xb0, 0x30));
                p.circle(12.0, 15.5, 1.5, rgb(0x7a, 0x50, 0x10));
            }
            Ic::Clock => {
                p.circle(12.0, 12.0, 9.5, rgb(0xe8, 0xec, 0xf4));
                p.line(&[(12.0, 6.5), (12.0, 12.0), (16.0, 14.0)], 1.8, rgb(0x3a, 0x40, 0x50));
            }
            Ic::Link => {
                let col = rgb(0x6d, 0x9c, 0xf8);
                p.line(&[(10.0, 14.0), (14.0, 10.0)], 2.0, col);
                p.line(&[(11.0, 6.5), (13.5, 4.0), (20.0, 10.5), (17.5, 13.0)], 2.0, col);
                p.line(&[(13.0, 17.5), (10.5, 20.0), (4.0, 13.5), (6.5, 11.0)], 2.0, col);
            }
            Ic::Info => {
                p.circle(12.0, 12.0, 9.5, c);
                p.rect(11.0, 10.5, 2.0, 7.0, 0.6, rgb(0x14, 0x15, 0x1b));
                p.circle(12.0, 7.5, 1.3, rgb(0x14, 0x15, 0x1b));
            }
            Ic::Keyboard | Ic::Terminal => {
                p.rect(2.0, 6.0, 20.0, 12.0, 2.0, rgb(0xd4, 0xda, 0xe4));
                for r in 0..2 {
                    for i in 0..6 {
                        p.rect(4.0 + i as f32 * 2.8, 8.0 + r as f32 * 3.0, 2.0, 2.0, 0.4, rgb(0x52, 0x58, 0x68));
                    }
                }
                p.rect(6.5, 14.2, 11.0, 2.0, 0.4, rgb(0x52, 0x58, 0x68));
            }
            Ic::OpenWith | Ic::Apps => {
                let cs = [rgb(0xe0, 0x5a, 0x3c), rgb(0x6c, 0xb8, 0x4c), rgb(0x3c, 0x9c, 0xe8), rgb(0xf6, 0xc3, 0x3e)];
                for (i, cc) in cs.iter().enumerate() {
                    p.rect(3.0 + (i % 2) as f32 * 9.5, 3.0 + (i / 2) as f32 * 9.5, 8.5, 8.5, 1.0, *cc);
                }
            }
            Ic::NewTab => {
                p.rect(2.5, 6.0, 19.0, 14.0, 1.5, rgb(0xb8, 0xbe, 0xcc));
                p.rect(2.5, 4.0, 8.0, 4.0, 1.0, rgb(0xb8, 0xbe, 0xcc));
                p.rect(4.0, 9.0, 16.0, 9.5, 0.5, rgb(0x3c, 0x4a, 0x66));
            }
            Ic::Briefcase => {
                p.rect(8.5, 4.0, 7.0, 4.0, 1.2, rgb(0x8a, 0x5a, 0x30));
                p.rect(2.5, 7.0, 19.0, 13.0, 2.0, rgb(0xb0, 0x74, 0x3e));
                p.rect(2.5, 12.0, 19.0, 1.5, 0.0, rgb(0x8a, 0x5a, 0x30));
            }
            // ------------------------------------------------------------ line icons
            Ic::Back => p.line(&[(20.0, 12.0), (5.0, 12.0), (11.0, 6.0), (5.0, 12.0), (11.0, 18.0)], lw, c),
            Ic::Forward => p.line(&[(4.0, 12.0), (19.0, 12.0), (13.0, 6.0), (19.0, 12.0), (13.0, 18.0)], lw, c),
            Ic::Up => p.line(&[(12.0, 20.0), (12.0, 5.0), (6.0, 11.0), (12.0, 5.0), (18.0, 11.0)], lw, c),
            Ic::Refresh => {
                p.arc(12.0, 12.0, 7.5, -40.0, 270.0, lw, c);
                p.line(&[(13.0, 2.0), (17.8, 7.2), (12.0, 8.3)], lw, c);
            }
            Ic::Plus => {
                p.line(&[(12.0, 5.0), (12.0, 19.0)], lw, c);
                p.line(&[(5.0, 12.0), (19.0, 12.0)], lw, c);
            }
            Ic::Cut => {
                p.ring(7.0, 17.0, 3.0, lw, c);
                p.ring(17.0, 17.0, 3.0, lw, c);
                p.line(&[(9.0, 14.8), (17.0, 3.5)], lw, c);
                p.line(&[(15.0, 14.8), (7.0, 3.5)], lw, c);
            }
            Ic::Copy => {
                p.line(&[(8.0, 8.0), (20.0, 8.0), (20.0, 20.0), (8.0, 20.0), (8.0, 8.0)], lw, c);
                p.line(&[(5.0, 16.0), (4.0, 16.0), (4.0, 4.0), (16.0, 4.0), (16.0, 5.0)], lw, c);
            }
            Ic::Paste => {
                p.line(&[(8.0, 5.0), (5.0, 5.0), (5.0, 21.0), (19.0, 21.0), (19.0, 5.0), (16.0, 5.0)], lw, c);
                p.line(&[(8.5, 3.0), (15.5, 3.0), (15.5, 7.0), (8.5, 7.0), (8.5, 3.0)], lw, c);
            }
            Ic::Rename => {
                p.line(&[(4.0, 20.0), (5.0, 15.5), (15.5, 5.0), (19.0, 8.5), (8.5, 19.0), (4.0, 20.0)], lw, c);
                p.line(&[(13.0, 7.5), (16.5, 11.0)], lw, c);
            }
            Ic::Share => {
                p.line(&[(12.0, 15.0), (12.0, 3.5), (7.5, 8.0), (12.0, 3.5), (16.5, 8.0)], lw, c);
                p.line(&[(7.0, 11.0), (5.0, 11.0), (5.0, 20.5), (19.0, 20.5), (19.0, 11.0), (17.0, 11.0)], lw, c);
            }
            Ic::Delete => {
                p.line(&[(4.0, 6.0), (20.0, 6.0)], lw, c);
                p.line(&[(9.0, 6.0), (9.5, 3.5), (14.5, 3.5), (15.0, 6.0)], lw, c);
                p.line(&[(6.0, 6.0), (7.0, 20.5), (17.0, 20.5), (18.0, 6.0)], lw, c);
                p.line(&[(10.0, 10.0), (10.0, 17.0)], lw * 0.8, c);
                p.line(&[(14.0, 10.0), (14.0, 17.0)], lw * 0.8, c);
            }
            Ic::Sort => {
                p.line(&[(8.0, 19.0), (8.0, 5.0), (4.5, 8.5), (8.0, 5.0), (11.5, 8.5)], lw, c);
                p.line(&[(16.0, 5.0), (16.0, 19.0), (12.5, 15.5), (16.0, 19.0), (19.5, 15.5)], lw, c);
            }
            Ic::ViewList => {
                for y in [6.0f32, 12.0, 18.0] {
                    p.line(&[(4.0, y), (20.0, y)], lw, c);
                }
            }
            Ic::Pane => {
                p.line(&[(3.0, 6.0), (21.0, 6.0), (21.0, 18.0), (3.0, 18.0), (3.0, 6.0)], lw, c);
                p.line(&[(15.0, 6.0), (15.0, 18.0)], lw, c);
            }
            Ic::Dots => {
                for x in [5.0f32, 12.0, 19.0] {
                    p.circle(x, 12.0, 1.5, c);
                }
            }
            Ic::ChevDown => p.poly(&[(6.0, 9.0), (18.0, 9.0), (12.0, 16.0)], c),
            Ic::ChevRight => p.poly(&[(9.0, 6.0), (16.0, 12.0), (9.0, 18.0)], c),
            Ic::Check => p.line(&[(5.0, 12.5), (10.0, 17.5), (19.0, 7.0)], 2.0, c),
            Ic::Close => {
                p.line(&[(6.0, 6.0), (18.0, 18.0)], lw, c);
                p.line(&[(18.0, 6.0), (6.0, 18.0)], lw, c);
            }
            Ic::Search => {
                p.ring(10.5, 10.5, 6.5, lw, c);
                p.line(&[(15.5, 15.5), (20.5, 20.5)], lw, c);
            }
            Ic::Grid => {
                for (x, y) in [(4.0f32, 4.0f32), (13.0, 4.0), (4.0, 13.0), (13.0, 13.0)] {
                    p.rect(x, y, 7.0, 7.0, 0.8, c);
                }
            }
            Ic::ListRows => {
                for y in [5.0f32, 10.5, 16.0] {
                    p.rect(4.0, y, 16.0, 3.5, 0.6, c);
                }
            }
            Ic::Compact => {
                p.line(&[(3.0, 8.0), (21.0, 8.0), (21.0, 16.0), (3.0, 16.0), (3.0, 8.0)], lw, c);
            }
            Ic::Minimize => p.line(&[(4.0, 12.0), (20.0, 12.0)], 1.4, c),
            Ic::Maximize => p.line(&[(5.0, 5.0), (19.0, 5.0), (19.0, 19.0), (5.0, 19.0), (5.0, 5.0)], 1.4, c),
            Ic::Undo => {
                p.arc(13.0, 13.0, 7.0, 180.0, 450.0, lw, c);
                p.line(&[(3.0, 9.0), (6.0, 13.0), (9.5, 9.5)], lw, c);
            }
            Ic::Eye => {
                p.arc(12.0, 18.0, 10.0, 215.0, 325.0, lw, c);
                p.arc(12.0, 6.0, 10.0, 35.0, 145.0, lw, c);
                p.ring(12.0, 12.0, 3.0, lw, c);
            }
            Ic::Split => {
                p.line(&[(3.0, 5.0), (21.0, 5.0), (21.0, 19.0), (3.0, 19.0), (3.0, 5.0)], lw, c);
                p.line(&[(12.0, 5.0), (12.0, 19.0)], lw, c);
            }
        }
    }
}

/// An icon at `size` px (`c`: the line icons' colour; the coloured ones keep their own).
pub fn ico<'a, M: 'a>(ic: Ic, c: Color, size: f32) -> Element<'a, M> {
    canvas::Canvas::new(Draw { ic, c }).width(Length::Fixed(size)).height(Length::Fixed(size)).into()
}

/// The icon for a place in the left menu or a Quick access tile (hearth-files' "kind").
pub fn place_icon(kind: &str) -> Ic {
    match kind {
        "start" => Ic::Home,
        "home" => Ic::Home,
        "pictures" | "gallery" => Ic::Gallery,
        "netroot" | "homeserver" => Ic::Cloud,
        "desktop" => Ic::Desktop,
        "downloads" => Ic::Download,
        "documents" => Ic::Document,
        "music" => Ic::Music,
        "videos" => Ic::Video,
        "games" => Ic::Games,
        "thispc" => Ic::Laptop,
        "system" | "drive" | "disk" | "internal" => Ic::Disc,
        "usb" | "removable" | "sd" | "sdcard" => Ic::Usb,
        "network" | "globe" => Ic::Globe,
        "device" | "phone" | "link" => Ic::Phone,
        "server" | "storage" | "share" => Ic::Server,
        "linux" => Ic::Penguin,
        "trash" => Ic::TrashBin,
        "apps" => Ic::Apps,
        "briefcase" => Ic::Briefcase,
        "bookmark" | "folder" => Ic::Folder,
        _ => Ic::Folder,
    }
}

/// The icon for a file by its kind (model::kind_of).
pub fn file_icon(kind: &str, name: &str) -> Ic {
    let low = name.to_lowercase();
    match kind {
        "folder" => Ic::Folder,
        "image" => Ic::FileImage,
        "video" => Ic::FileVideo,
        "audio" => Ic::FileAudio,
        "archive" => Ic::FileArchive,
        "disc" => Ic::FileDisc,
        "app" => Ic::FileApp,
        "doc" if low.ends_with(".pdf") => Ic::FilePdf,
        "doc" | "text" => Ic::FileText,
        _ => Ic::File,
    }
}
