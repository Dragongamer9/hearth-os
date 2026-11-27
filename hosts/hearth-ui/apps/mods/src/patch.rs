//! Make a patch with the winners (board 1's conflict lens): where two or more of your plugins change the same record,
//! the one lowest in the load order wins whole, and the others' changes are lost. The patch (a small light plugin,
//! "Hearth Patch - <mod>.esp", placed last) keeps the winner and brings back each earlier plugin's changes to the
//! parts (subrecords) the winner left as the original had them, as Mator Smash and zMerge's patchers do. Written anew
//! in Rust from the plugin format (UESP's Mod File Format pages); no xEdit or zEdit is used.
//!
//! Safe by construction: a plugin's own references (form IDs) are renumbered for the patch's master list; a part
//! whose references can't be told apart for sure is left as the winner has it (and said). Records inside cells and
//! worldspaces (placed objects, navmeshes) stay as they are.

use std::collections::{BTreeMap, BTreeSet};

pub type Sub = ([u8; 4], Vec<u8>);

#[derive(Debug, Clone)]
pub struct Rec {
    pub sig: [u8; 4],
    pub flags: u32,
    pub fid: u32,
    pub rest: [u8; 8],
    pub subs: Vec<Sub>,
}

#[derive(Debug, Clone, Default)]
pub struct Plugin {
    pub name: String,
    pub masters: Vec<String>,
    pub version: f32,
    /// the records in top groups (not inside cells, worldspaces or topics' children)
    pub recs: Vec<Rec>,
    pub nested: usize,
}

impl Plugin {
    /// Its masters, then itself: what a form ID's top byte points at.
    pub fn list(&self) -> Vec<String> {
        let mut v = self.masters.clone();
        v.push(self.name.clone());
        v
    }
    pub fn key(&self, fid: u32) -> (String, u32) {
        let i = (fid >> 24) as usize;
        let l = self.list();
        (l.get(i).or(l.last()).cloned().unwrap_or_default().to_lowercase(), fid & 0x00FF_FFFF)
    }
}

fn u32_at(b: &[u8], i: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(i..i + 4)?.try_into().ok()?))
}

fn subs_of(data: &[u8]) -> Vec<Sub> {
    let mut v = Vec::new();
    let mut i = 0;
    let mut big: Option<usize> = None;
    while i + 6 <= data.len() {
        let sig: [u8; 4] = data[i..i + 4].try_into().unwrap_or([0; 4]);
        let mut len = u16::from_le_bytes([data[i + 4], data[i + 5]]) as usize;
        i += 6;
        if let Some(b) = big.take() {
            len = b;
        }
        let Some(d) = data.get(i..i + len) else { break };
        if &sig == b"XXXX" && len == 4 {
            big = u32_at(d, 0).map(|x| x as usize);
        } else {
            v.push((sig, d.to_vec()));
        }
        i += len;
    }
    v
}

fn inflate(b: &[u8]) -> Option<Vec<u8>> {
    use std::io::Read;
    let n = u32_at(b, 0)? as usize;
    let mut out = Vec::with_capacity(n);
    flate2::read::ZlibDecoder::new(b.get(4..)?).read_to_end(&mut out).ok()?;
    Some(out)
}

const COMPRESSED: u32 = 0x0004_0000;

/// Reads a plugin (.esp / .esm / .esl) of a game with 24-byte record headers (Skyrim SE, Fallout 4, Starfield's kind).
pub fn read(name: &str, b: &[u8]) -> Result<Plugin, String> {
    if b.get(0..4) != Some(b"TES4") {
        return Err(format!("{name} isn't a plugin this patcher reads."));
    }
    let hsize = u32_at(b, 4).ok_or("short")? as usize;
    let head = subs_of(b.get(24..24 + hsize).ok_or("short header")?);
    let mut p = Plugin { name: name.to_string(), ..Default::default() };
    for (s, d) in &head {
        if s == b"MAST" {
            p.masters.push(String::from_utf8_lossy(d).trim_end_matches('\0').to_string());
        } else if s == b"HEDR" {
            p.version = f32::from_le_bytes(d.get(0..4).and_then(|x| x.try_into().ok()).unwrap_or([0; 4]));
        }
    }
    let mut i = 24 + hsize;
    while i + 24 <= b.len() {
        if &b[i..i + 4] != b"GRUP" {
            break;
        }
        let gsize = u32_at(b, i + 4).unwrap_or(0) as usize;
        let gtype = u32_at(b, i + 12).unwrap_or(0);
        let end = (i + gsize).min(b.len());
        if gtype == 0 {
            let mut j = i + 24;
            while j + 24 <= end {
                let sig: [u8; 4] = b[j..j + 4].try_into().unwrap_or([0; 4]);
                let size = u32_at(b, j + 4).unwrap_or(0) as usize;
                if &sig == b"GRUP" {
                    p.nested += 1;
                    j += size.max(24);
                    continue;
                }
                let flags = u32_at(b, j + 8).unwrap_or(0);
                let fid = u32_at(b, j + 12).unwrap_or(0);
                let rest: [u8; 8] = b[j + 16..j + 24].try_into().unwrap_or([0; 8]);
                let raw = b.get(j + 24..j + 24 + size).ok_or("a record runs past the end")?;
                let data = if flags & COMPRESSED != 0 { inflate(raw).ok_or("a compressed record couldn't be read")? } else { raw.to_vec() };
                p.recs.push(Rec { sig, flags: flags & !COMPRESSED, fid, rest, subs: subs_of(&data) });
                j += 24 + size;
            }
        }
        i = end.max(i + 24);
    }
    Ok(p)
}

/// What the patch did, in words, and its bytes.
#[derive(Debug, Default)]
pub struct Made {
    pub bytes: Vec<u8>,
    pub records: usize,
    pub parts: usize,
    pub kept: usize,
    pub masters: Vec<String>,
}

fn renumber(d: &[u8], from: &[String], to: &[String], known: &BTreeSet<(String, u32)>) -> Option<Vec<u8>> {
    let mut out = d.to_vec();
    let same = |i: usize| from.get(i).zip(to.get(i)).map(|(a, b)| a.eq_ignore_ascii_case(b)).unwrap_or(false);
    let mut i = 0;
    while i + 4 <= d.len() {
        let v = u32_at(d, i).unwrap_or(0);
        let idx = (v >> 24) as usize;
        let low = v & 0x00FF_FFFF;
        if idx < from.len() && low >= 0x800 && !same(idx) {
            let k = (from[idx].to_lowercase(), low);
            let new = to.iter().position(|t| t.eq_ignore_ascii_case(&from[idx]));
            let looks = known.contains(&k);
            if i % 4 == 0 && looks {
                let n = new?;
                out[i..i + 4].copy_from_slice(&(((n as u32) << 24) | low).to_le_bytes());
            } else if looks || i % 4 == 0 {
                // a reference not on a field's edge, or a value that may be one: not taken
                return None;
            }
        }
        i += 1;
    }
    Some(out)
}

fn of(subs: &[Sub], t: [u8; 4]) -> Vec<&Vec<u8>> {
    subs.iter().filter(|s| s.0 == t).map(|s| &s.1).collect()
}

/// The patch: `plugins` in load order (already read); `only` limits it to records one of those plugins (by name)
/// touches (empty: every record). `light` is the game's light-plugin flag (0: none).
pub fn make(plugins: &[Plugin], only: &[String], light: u32) -> Result<Made, String> {
    let pos: BTreeMap<String, usize> = plugins.iter().enumerate().map(|(i, p)| (p.name.to_lowercase(), i)).collect();
    let mut by: BTreeMap<(String, u32), Vec<(usize, usize)>> = BTreeMap::new();
    for (pi, p) in plugins.iter().enumerate() {
        for (ri, r) in p.recs.iter().enumerate() {
            by.entry(p.key(r.fid)).or_default().push((pi, ri));
        }
    }
    let known: BTreeSet<(String, u32)> = by.keys().cloned().collect();
    let only: BTreeSet<String> = only.iter().map(|s| s.to_lowercase()).collect();
    // pass 1: what changes
    let mut picks: Vec<(usize, Vec<(usize, Sub)>, (String, u32))> = Vec::new();
    let mut kept = 0usize;
    let mut used: BTreeSet<usize> = BTreeSet::new();
    for (k, es) in &by {
        if es.len() < 3 || (!only.is_empty() && !es.iter().any(|(pi, _)| only.contains(&plugins[*pi].name.to_lowercase()))) {
            continue;
        }
        let (bp, br) = es[0];
        let (wp, wr) = *es.last().unwrap_or(&es[0]);
        let base = &plugins[bp].recs[br];
        let win = &plugins[wp].recs[wr];
        if base.sig != win.sig {
            continue;
        }
        // (from which plugin, the part)
        let mut result: Vec<(usize, Sub)> = win.subs.iter().map(|s| (wp, s.clone())).collect();
        let mut changed = false;
        for &(op, or) in &es[1..es.len() - 1] {
            let o = &plugins[op].recs[or];
            let types: BTreeSet<[u8; 4]> = o.subs.iter().chain(base.subs.iter()).map(|s| s.0).collect();
            for t in types {
                let (vo, vb, vw) = (of(&o.subs, t), of(&base.subs, t), of(&win.subs, t));
                if vo == vb || vw != vb {
                    continue;
                }
                if vo.is_empty() {
                    // it took that part out
                    result.retain(|s| s.1 .0 != t);
                    changed = true;
                    continue;
                }
                let at = result.iter().position(|s| s.1 .0 == t);
                let Some(at) = at else {
                    kept += 1;
                    continue;
                };
                result.retain(|s| s.1 .0 != t);
                let at = at.min(result.len());
                for (n, d) in vo.into_iter().enumerate() {
                    result.insert(at + n, (op, (t, d.clone())));
                }
                changed = true;
            }
        }
        if changed {
            for (p, _) in &result {
                used.insert(*p);
            }
            used.insert(wp);
            picks.push((wp, result, k.clone()));
        }
    }
    if picks.is_empty() {
        return Err("Nothing to patch: where your plugins change the same records, the winner already has every change.".into());
    }
    // the patch's masters: every plugin the parts come from and theirs, in load order
    let mut ms: BTreeSet<String> = BTreeSet::new();
    for u in &used {
        for n in plugins[*u].list() {
            ms.insert(n);
        }
    }
    let mut masters: Vec<String> = ms.into_iter().collect();
    masters.sort_by_key(|m| pos.get(&m.to_lowercase()).copied().unwrap_or(usize::MAX));
    masters.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
    if masters.len() > 254 {
        return Err(format!("The patch would need {} masters (254 at most).", masters.len()));
    }
    // pass 2: renumber and write, by top group
    let mut groups: BTreeMap<[u8; 4], Vec<u8>> = BTreeMap::new();
    let mut order: Vec<[u8; 4]> = Vec::new();
    let (mut records, mut parts) = (0usize, 0usize);
    for (wp, result, k) in picks {
        let win_list = plugins[wp].list();
        let w = plugins[wp].recs.iter().find(|r| plugins[wp].key(r.fid) == k).cloned();
        let Some(w) = w else { continue };
        let mut data: Vec<u8> = Vec::new();
        let mut ok = true;
        for (from, (t, d)) in &result {
            let fl = plugins[*from].list();
            let d2 = match renumber(d, &fl, &masters, &known) {
                Some(x) => x,
                None if *from != wp => {
                    // left as the winner has it
                    kept += 1;
                    match w.subs.iter().find(|s| s.0 == *t).and_then(|s| renumber(&s.1, &win_list, &masters, &known)) {
                        Some(x) => x,
                        None => continue,
                    }
                }
                None => {
                    ok = false;
                    break;
                }
            };
            if *from != wp {
                parts += 1;
            }
            if d2.len() > 0xFFFF {
                data.extend_from_slice(b"XXXX");
                data.extend_from_slice(&4u16.to_le_bytes());
                data.extend_from_slice(&(d2.len() as u32).to_le_bytes());
                data.extend_from_slice(t);
                data.extend_from_slice(&0u16.to_le_bytes());
            } else {
                data.extend_from_slice(t);
                data.extend_from_slice(&(d2.len() as u16).to_le_bytes());
            }
            data.extend_from_slice(&d2);
        }
        if !ok {
            kept += 1;
            continue;
        }
        let idx = masters.iter().position(|m| m.eq_ignore_ascii_case(&win_list[((w.fid >> 24) as usize).min(win_list.len() - 1)])).ok_or("a master went missing")?;
        let fid = ((idx as u32) << 24) | (w.fid & 0x00FF_FFFF);
        let g = groups.entry(w.sig).or_default();
        if !order.contains(&w.sig) {
            order.push(w.sig);
        }
        g.extend_from_slice(&w.sig);
        g.extend_from_slice(&(data.len() as u32).to_le_bytes());
        g.extend_from_slice(&w.flags.to_le_bytes());
        g.extend_from_slice(&fid.to_le_bytes());
        g.extend_from_slice(&w.rest);
        g.extend_from_slice(&data);
        records += 1;
    }
    if records == 0 {
        return Err("Nothing could be patched safely (the parts that differ refer to things the patch can't renumber for sure).".into());
    }
    // the header: HEDR (the winners' format version, the record count, the next id), the author, the masters
    let version = plugins.iter().rev().map(|p| p.version).find(|v| *v > 0.0).unwrap_or(1.71);
    let mut h: Vec<u8> = Vec::new();
    let mut sub = |t: &[u8; 4], d: &[u8]| {
        h.extend_from_slice(t);
        h.extend_from_slice(&(d.len() as u16).to_le_bytes());
        h.extend_from_slice(d);
    };
    let mut hedr = version.to_le_bytes().to_vec();
    hedr.extend_from_slice(&(records as u32).to_le_bytes());
    hedr.extend_from_slice(&0x800u32.to_le_bytes());
    sub(b"HEDR", &hedr);
    sub(b"CNAM", b"Hearth Mod Manager\0");
    sub(b"SNAM", b"Made with the winners: each plugin's own changes kept\0");
    for m in &masters {
        let mut z = m.as_bytes().to_vec();
        z.push(0);
        sub(b"MAST", &z);
        sub(b"DATA", &0u64.to_le_bytes());
    }
    let mut out: Vec<u8> = Vec::new();
    out.extend_from_slice(b"TES4");
    out.extend_from_slice(&(h.len() as u32).to_le_bytes());
    out.extend_from_slice(&light.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&[0, 0, 0, 0, 44, 0, 0, 0]);
    out.extend_from_slice(&h);
    for sig in order {
        let g = &groups[&sig];
        out.extend_from_slice(b"GRUP");
        out.extend_from_slice(&((g.len() + 24) as u32).to_le_bytes());
        out.extend_from_slice(&sig);
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&[0u8; 8]);
        out.extend_from_slice(g);
    }
    Ok(Made { bytes: out, records, parts, kept, masters })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sub(t: &[u8; 4], d: &[u8]) -> Vec<u8> {
        let mut v = t.to_vec();
        v.extend_from_slice(&(d.len() as u16).to_le_bytes());
        v.extend_from_slice(d);
        v
    }
    fn plugin(masters: &[&str], recs: &[(u32, Vec<Vec<u8>>)]) -> Vec<u8> {
        let mut h = sub(b"HEDR", &[0xae, 0x47, 0xd9, 0x3f, 1, 0, 0, 0, 0, 8, 0, 0]);
        for m in masters {
            let mut z = m.as_bytes().to_vec();
            z.push(0);
            h.extend(sub(b"MAST", &z));
            h.extend(sub(b"DATA", &[0; 8]));
        }
        let mut b = b"TES4".to_vec();
        b.extend((h.len() as u32).to_le_bytes());
        b.extend([0u8; 16]);
        b.extend(h);
        let mut g = Vec::new();
        for (fid, subs) in recs {
            let d: Vec<u8> = subs.concat();
            g.extend(b"WEAP");
            g.extend((d.len() as u32).to_le_bytes());
            g.extend(0u32.to_le_bytes());
            g.extend(fid.to_le_bytes());
            g.extend([0u8; 8]);
            g.extend(d);
        }
        b.extend(b"GRUP");
        b.extend(((g.len() + 24) as u32).to_le_bytes());
        b.extend(b"WEAP");
        b.extend([0u8; 12]);
        b.extend(g);
        b
    }

    #[test]
    fn both_changes_kept() {
        // Skyrim.esm's sword: name, damage. A changes the damage, B (the winner) the name.
        let base = plugin(&[], &[(0x0001_2EB7, vec![sub(b"EDID", b"IronSword\0"), sub(b"FULL", b"Iron Sword\0"), sub(b"DATA", &[10, 0, 0, 0])])]);
        let a = plugin(&["Skyrim.esm"], &[(0x0001_2EB7, vec![sub(b"EDID", b"IronSword\0"), sub(b"FULL", b"Iron Sword\0"), sub(b"DATA", &[14, 0, 0, 0])])]);
        let b = plugin(&["Skyrim.esm"], &[(0x0001_2EB7, vec![sub(b"EDID", b"IronSword\0"), sub(b"FULL", b"Old Iron Sword\0"), sub(b"DATA", &[10, 0, 0, 0])])]);
        let ps = vec![read("Skyrim.esm", &base).unwrap(), read("Damage.esp", &a).unwrap(), read("Names.esp", &b).unwrap()];
        let m = make(&ps, &[], 0x200).unwrap();
        assert_eq!(m.records, 1);
        assert_eq!(m.masters, vec!["Skyrim.esm", "Damage.esp", "Names.esp"]);
        let p = read("Hearth Patch.esp", &m.bytes).unwrap();
        let r = &p.recs[0];
        assert_eq!(r.fid, 0x0001_2EB7);
        assert_eq!(of(&r.subs, *b"FULL")[0].as_slice(), b"Old Iron Sword\0");
        assert_eq!(of(&r.subs, *b"DATA")[0].as_slice(), &[14, 0, 0, 0]);
    }

    #[test]
    fn references_renumbered() {
        // A adds a keyword (its own form 0x01000801) to the base's sword; B wins with another change.
        let base = plugin(&[], &[(0x0000_0900, vec![sub(b"EDID", b"S\0"), sub(b"KWDA", &0x0000_0901u32.to_le_bytes()), sub(b"DATA", &[1, 0, 0, 0])]), (0x0000_0901, vec![sub(b"EDID", b"K\0")])]);
        let a = plugin(&["Skyrim.esm"], &[(0x0000_0900, vec![sub(b"EDID", b"S\0"), sub(b"KWDA", &0x0100_0801u32.to_le_bytes()), sub(b"DATA", &[1, 0, 0, 0])]), (0x0100_0801, vec![sub(b"EDID", b"NewKeyword\0")])]);
        let b = plugin(&["Skyrim.esm", "Other.esm"], &[(0x0000_0900, vec![sub(b"EDID", b"S\0"), sub(b"KWDA", &0x0000_0901u32.to_le_bytes()), sub(b"DATA", &[9, 0, 0, 0])])]);
        let other = plugin(&["Skyrim.esm"], &[]);
        let ps = vec![read("Skyrim.esm", &base).unwrap(), read("Other.esm", &other).unwrap(), read("Keywords.esp", &a).unwrap(), read("Tweaks.esp", &b).unwrap()];
        let m = make(&ps, &[], 0x200).unwrap();
        let p = read("Hearth Patch.esp", &m.bytes).unwrap();
        let at = p.masters.iter().position(|x| x == "Keywords.esp").unwrap() as u32;
        let r = p.recs.iter().find(|r| r.fid & 0xFFFFFF == 0x900).unwrap();
        assert_eq!(u32_at(of(&r.subs, *b"KWDA")[0], 0).unwrap(), (at << 24) | 0x801);
        assert_eq!(of(&r.subs, *b"DATA")[0].as_slice(), &[9, 0, 0, 0]);
    }
}
