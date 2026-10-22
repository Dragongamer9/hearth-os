//! Hearth's mark (ui/hearth-logo.svg: a fireplace arch with a flame inside) as terminal art. The picture is 30 x 30
//! pixels, drawn two pixels to a character with half blocks, in true colour, in the 256-colour set, or as plain
//! characters. Made from the SVG's shapes by a small script (each pixel: the shape covering most of it, coloured
//! from the SVG's gradients); one letter per colour, "." where nothing is drawn. The fourth field of each colour
//! says which part it belongs to (s stone, f flame, c the flame's core), for the plain version.

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Colors { True, X256, Plain }

/// How many columns the art takes.
pub const WIDTH: usize = 30;

const PALETTE: [(u8, u8, u8, char); 47] = [
    (248, 240, 224, 's'),
    (248, 232, 224, 's'),
    (240, 232, 224, 's'),
    (240, 232, 216, 's'),
    (240, 224, 216, 's'),
    (255, 200, 96, 'f'),
    (240, 224, 208, 's'),
    (255, 192, 96, 'f'),
    (232, 224, 208, 's'),
    (255, 192, 88, 'f'),
    (255, 184, 88, 'f'),
    (255, 176, 80, 'f'),
    (232, 216, 208, 's'),
    (255, 168, 80, 'f'),
    (232, 216, 200, 's'),
    (255, 160, 80, 'f'),
    (224, 216, 200, 's'),
    (255, 160, 72, 'f'),
    (255, 152, 64, 'f'),
    (255, 152, 72, 'f'),
    (224, 208, 192, 's'),
    (255, 144, 64, 'f'),
    (255, 240, 200, 'c'),
    (255, 136, 64, 'f'),
    (255, 240, 192, 'c'),
    (255, 128, 64, 'f'),
    (255, 232, 176, 'c'),
    (248, 128, 64, 'f'),
    (255, 224, 160, 'c'),
    (248, 112, 64, 'f'),
    (255, 224, 144, 'c'),
    (255, 224, 152, 'c'),
    (216, 200, 184, 's'),
    (240, 112, 64, 'f'),
    (255, 216, 136, 'c'),
    (240, 104, 64, 'f'),
    (255, 208, 120, 'c'),
    (240, 96, 64, 'f'),
    (255, 208, 112, 'c'),
    (216, 200, 176, 's'),
    (232, 88, 56, 'f'),
    (255, 200, 96, 'c'),
    (208, 192, 176, 's'),
    (232, 80, 56, 'f'),
    (255, 192, 80, 'c'),
    (232, 72, 56, 'f'),
    (208, 192, 168, 's'),
];
const PIXELS: [&str; 30] = [
    "...........AAAAAAAA...........",
    ".........BBBBBBBBBBBB.........",
    ".......CCCCCC....CCCCCC.......",
    "......DDDD..........DDDD......",
    "......DDD............DDD......",
    ".....EEE..............EEE.....",
    "....EEE........F.......EEE....",
    "....GGG.......FFH......GGG....",
    "....II........JHH.......II....",
    "...III........KKKK......III...",
    "...III........LLLLL.....III...",
    "...MM........NNNNNN......MM...",
    "...OO......P.PPPPPPP.....OO...",
    "...QQ.....RR.RRRRRRR.....QQ...",
    "...QQ.....SS..STSSSSS....QQ...",
    "...UU....VVVV.VWVVVVV....UU...",
    "...UU....XXXXXYYXXXXX....UU...",
    "...UU....ZZZZaaaaZZZZ....UU...",
    "...UU....bbbbcccccbbb....UU...",
    "...UU....dddefffffddd....UU...",
    "...gg....hhhiiiiiihhh....gg...",
    "...gg....jjjkkkkkkjjj....gg...",
    "...gg.....llmmmmmmll.....gg...",
    "...nn......oopppppo......nn...",
    "...qq.......rrssrr.......qq...",
    "...qq.........tt.........qq...",
    "...qq....................qq...",
    ".uqqqqqqqqqqqqqqqqqqqqqqqqqqu.",
    "uuuuuuuuuuuuuuuuuuuuuuuuuuuuuu",
    ".uuuuuuuuuuuuuuuuuuuuuuuuuuuu.",
];

fn index(c: char) -> Option<usize> {
    match c {
        'A'..='Z' => Some(c as usize - 'A' as usize),
        'a'..='z' => Some(26 + c as usize - 'a' as usize),
        '0'..='9' => Some(52 + c as usize - '0' as usize),
        _ => None,
    }
}

/// The nearest colour of the 256-colour set: its 6x6x6 cube or its grey ramp.
pub fn x256(r: u8, g: u8, b: u8) -> u8 {
    let level = |v: u8| -> usize { if v < 48 { 0 } else if v < 115 { 1 } else { ((v as usize - 35) / 40).min(5) } };
    const STEPS: [i32; 6] = [0, 95, 135, 175, 215, 255];
    let (ri, gi, bi) = (level(r), level(g), level(b));
    let cube = (STEPS[ri], STEPS[gi], STEPS[bi]);
    let avg = (r as i32 + g as i32 + b as i32) / 3;
    let gi2 = if avg > 238 { 23 } else { ((avg - 3).max(0) / 10).min(23) };
    let grey = 8 + gi2 * 10;
    let d = |a: (i32, i32, i32)| (a.0 - r as i32).pow(2) + (a.1 - g as i32).pow(2) + (a.2 - b as i32).pow(2);
    if d((grey, grey, grey)) < d(cube) { (232 + gi2) as u8 } else { (16 + 36 * ri + 6 * gi + bi) as u8 }
}

fn fg(mode: Colors, c: (u8, u8, u8, char)) -> String {
    match mode { Colors::True => format!("\x1b[38;2;{};{};{}m", c.0, c.1, c.2), Colors::X256 => format!("\x1b[38;5;{}m", x256(c.0, c.1, c.2)), Colors::Plain => String::new() }
}
fn bg(mode: Colors, c: (u8, u8, u8, char)) -> String {
    match mode { Colors::True => format!("\x1b[48;2;{};{};{}m", c.0, c.1, c.2), Colors::X256 => format!("\x1b[48;5;{}m", x256(c.0, c.1, c.2)), Colors::Plain => String::new() }
}

/// The art, one string per terminal line, each WIDTH columns wide.
pub fn lines(mode: Colors) -> Vec<String> {
    let rows: Vec<Vec<Option<usize>>> = PIXELS.iter().map(|r| r.chars().map(index).collect()).collect();
    let mut out = vec![];
    for pair in rows.chunks(2) {
        let (top, bottom) = (&pair[0], pair.get(1));
        let mut line = String::new();
        for x in 0..WIDTH {
            let t = top.get(x).copied().flatten().map(|i| PALETTE[i]);
            let b = bottom.and_then(|r| r.get(x).copied().flatten()).map(|i| PALETTE[i]);
            if mode == Colors::Plain {
                let part = t.or(b).map(|c| c.3);
                line.push(match part { Some('c') => '*', Some('f') => '#', Some(_) => '%', None => ' ' });
                continue;
            }
            match (t, b) {
                (None, None) => line.push(' '),
                (Some(t), None) => { line.push_str(&fg(mode, t)); line.push('\u{2580}'); line.push_str("\x1b[0m"); }
                (None, Some(b)) => { line.push_str(&fg(mode, b)); line.push('\u{2584}'); line.push_str("\x1b[0m"); }
                (Some(t), Some(b)) if (t.0, t.1, t.2) == (b.0, b.1, b.2) => { line.push_str(&fg(mode, t)); line.push('\u{2588}'); line.push_str("\x1b[0m"); }
                (Some(t), Some(b)) => { line.push_str(&fg(mode, t)); line.push_str(&bg(mode, b)); line.push('\u{2580}'); line.push_str("\x1b[0m"); }
            }
        }
        out.push(line);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_row_fits_and_every_colour_exists() {
        for r in PIXELS { assert_eq!(r.chars().count(), WIDTH); for c in r.chars() { if let Some(i) = index(c) { assert!(i < PALETTE.len(), "{c}"); } else { assert_eq!(c, '.'); } } }
        assert_eq!(PIXELS.len() % 2, 0);
    }

    #[test]
    fn plain_art_has_no_escapes() {
        let l = lines(Colors::Plain);
        assert_eq!(l.len(), PIXELS.len() / 2);
        for s in &l { assert_eq!(s.chars().count(), WIDTH); assert!(!s.contains('\x1b')); }
        assert!(l.iter().any(|s| s.contains('#')) && l.iter().any(|s| s.contains('*')) && l.iter().any(|s| s.contains('%')));
        assert!(lines(Colors::True)[0].contains("\x1b[38;2;"));
        assert!(lines(Colors::X256)[0].contains("\x1b[38;5;"));
    }

    #[test]
    fn nearest_256_colours() {
        assert_eq!(x256(0, 0, 0), 16);
        assert_eq!(x256(255, 255, 255), 231);
        assert_eq!(x256(255, 0, 0), 196);
        assert_eq!(x256(128, 128, 128), 244);
        assert_eq!(x256(255, 138, 61), 209);
    }
}
