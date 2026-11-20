//! The vault as a graph: a node for each note, a link for each `[[link]]`, embed or written-out path. All counts are
//! the vault's own (a project with no notes is "no notes yet"; nothing is made up). The layout is worked out once
//! when the vault is read (off the window's thread) and kept; the modes place the same nodes their own way.

use crate::vault::{EdgeKind, Vault};

#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    pub note: usize,
    pub label: String,
    pub project: String,
    pub kind: String,
    pub tags: Vec<String>,
    pub links: usize,
    pub words: usize,
    pub mtime: i64,
    pub log: Option<crate::day::Ymd>,
    /// Has a picture of its own (its first embedded picture), for "a note's own picture as its node".
    pub picture: Option<String>,
    /// Classic's place, -1..1.
    pub x: f32,
    pub y: f32,
    /// Depth for the 3D modes, -1..1.
    pub z: f32,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Graph {
    pub nodes: Vec<Node>,
    pub edges: Vec<(usize, usize, EdgeKind)>,
}

/// A small steady random number from a string (the same note lands in the same place every time).
pub fn hash(s: &str) -> u32 {
    let mut h: u32 = 0x811c_9dc5;
    for b in s.bytes() {
        h ^= b as u32;
        h = h.wrapping_mul(0x0100_0193);
    }
    h
}

pub fn unit(s: &str, salt: u32) -> f32 {
    let mut h = hash(s) ^ salt.wrapping_mul(0x9e37_79b9);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2c1b_3c6d);
    h ^= h >> 12;
    (h % 100_000) as f32 / 100_000.0
}

pub fn build(v: &Vault) -> Graph {
    let mut g = Graph::default();
    for (i, n) in v.notes.iter().enumerate() {
        let pic = n.embeds.iter().find(|e| crate::md::is_picture(e)).cloned();
        g.nodes.push(Node {
            note: i,
            label: n.label.clone(),
            project: n.project.clone(),
            kind: n.kind.clone(),
            tags: n.tags.clone(),
            links: 0,
            words: n.words,
            mtime: n.mtime,
            log: n.log,
            picture: pic,
            x: unit(&n.rel, 1) * 2.0 - 1.0,
            y: unit(&n.rel, 2) * 2.0 - 1.0,
            z: unit(&n.rel, 3) * 2.0 - 1.0,
        });
    }
    for e in &v.edges {
        if e.from < g.nodes.len() && e.to < g.nodes.len() && e.from != e.to {
            g.edges.push((e.from, e.to, e.kind));
        }
    }
    for (a, b, _) in g.edges.clone() {
        g.nodes[a].links += 1;
        g.nodes[b].links += 1;
    }
    layout(&mut g);
    g
}

/// A force layout (Fruchterman–Reingold), the same every time for the same notes. Fewer steps for big vaults.
pub fn layout(g: &mut Graph) {
    let n = g.nodes.len();
    if n == 0 {
        return;
    }
    let steps = if n > 3000 { 25 } else if n > 800 { 60 } else { 220 };
    let k = (4.0 / n as f32).sqrt().clamp(0.02, 0.5);
    let mut px: Vec<f32> = g.nodes.iter().map(|x| x.x).collect();
    let mut py: Vec<f32> = g.nodes.iter().map(|x| x.y).collect();
    // A pull toward each project's own corner keeps projects apart (the logs in the middle).
    let anchor = |p: &str| -> (f32, f32) {
        match p {
            "log" => (0.05, 0.05),
            "home" | "tools" | "" => (-0.1, -0.4),
            other => {
                let a = unit(other, 9) * std::f32::consts::TAU;
                (0.55 * a.cos(), 0.55 * a.sin())
            }
        }
    };
    let anchors: Vec<(f32, f32)> = g.nodes.iter().map(|x| anchor(&x.project)).collect();
    let mut t = 0.12f32;
    let big = n > 1500;
    for _ in 0..steps {
        let mut dx = vec![0f32; n];
        let mut dy = vec![0f32; n];
        if big {
            // Big vaults: repulsion only between nodes in the same cell of a grid.
            let cells = 24usize;
            let cell = |x: f32, y: f32| -> usize { let cx = (((x + 1.5) / 3.0) * cells as f32).clamp(0.0, cells as f32 - 1.0) as usize; let cy = (((y + 1.5) / 3.0) * cells as f32).clamp(0.0, cells as f32 - 1.0) as usize; cy * cells + cx };
            let mut grid: Vec<Vec<usize>> = vec![Vec::new(); cells * cells];
            for i in 0..n {
                grid[cell(px[i], py[i])].push(i);
            }
            for bucket in &grid {
                for (ai, &i) in bucket.iter().enumerate() {
                    for &j in &bucket[ai + 1..] {
                        let (ddx, ddy) = (px[i] - px[j], py[i] - py[j]);
                        let d2 = (ddx * ddx + ddy * ddy).max(1e-6);
                        let f = k * k / d2;
                        dx[i] += ddx * f;
                        dy[i] += ddy * f;
                        dx[j] -= ddx * f;
                        dy[j] -= ddy * f;
                    }
                }
            }
        } else {
            for i in 0..n {
                for j in i + 1..n {
                    let (ddx, ddy) = (px[i] - px[j], py[i] - py[j]);
                    let d2 = (ddx * ddx + ddy * ddy).max(1e-6);
                    let f = k * k / d2;
                    dx[i] += ddx * f;
                    dy[i] += ddy * f;
                    dx[j] -= ddx * f;
                    dy[j] -= ddy * f;
                }
            }
        }
        for &(a, b, _) in &g.edges {
            let (ddx, ddy) = (px[a] - px[b], py[a] - py[b]);
            let d = (ddx * ddx + ddy * ddy).sqrt().max(1e-4);
            let f = d / k;
            dx[a] -= ddx * f;
            dy[a] -= ddy * f;
            dx[b] += ddx * f;
            dy[b] += ddy * f;
        }
        for i in 0..n {
            dx[i] += (anchors[i].0 - px[i]) * 0.6;
            dy[i] += (anchors[i].1 - py[i]) * 0.6;
            let d = (dx[i] * dx[i] + dy[i] * dy[i]).sqrt().max(1e-6);
            let m = d.min(t);
            px[i] += dx[i] / d * m;
            py[i] += dy[i] / d * m;
        }
        t *= 0.985;
    }
    // Fit to -1..1.
    let (mut x0, mut x1, mut y0, mut y1) = (f32::MAX, f32::MIN, f32::MAX, f32::MIN);
    for i in 0..n {
        x0 = x0.min(px[i]);
        x1 = x1.max(px[i]);
        y0 = y0.min(py[i]);
        y1 = y1.max(py[i]);
    }
    let (sx, sy) = ((x1 - x0).max(1e-3), (y1 - y0).max(1e-3));
    for (i, nd) in g.nodes.iter_mut().enumerate() {
        nd.x = if n == 1 { 0.0 } else { (px[i] - x0) / sx * 2.0 - 1.0 };
        nd.y = if n == 1 { 0.0 } else { (py[i] - y0) / sy * 2.0 - 1.0 };
    }
}

impl Graph {
    pub fn neighbours(&self, i: usize) -> Vec<usize> {
        let mut v: Vec<usize> = self.edges.iter().filter_map(|&(a, b, _)| if a == i { Some(b) } else if b == i { Some(a) } else { None }).collect();
        v.sort();
        v.dedup();
        v
    }

    /// The most linked notes: (node, links).
    pub fn most_linked(&self, n: usize) -> Vec<(usize, usize)> {
        let mut v: Vec<(usize, usize)> = self.nodes.iter().enumerate().filter(|(_, x)| x.project != "log" && x.project != "home").map(|(i, x)| (i, x.links)).filter(|x| x.1 > 0).collect();
        v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        v.truncate(n);
        v
    }

    pub fn orphans(&self) -> Vec<usize> {
        self.nodes.iter().enumerate().filter(|(_, x)| x.links == 0).map(|(i, _)| i).collect()
    }

    /// Groups of linked notes (connected parts with 2 or more notes, without the logs and Home, which join
    /// everything), each named by the word most of its notes start with: (name, size).
    pub fn clusters(&self) -> Vec<(String, usize)> {
        let n = self.nodes.len();
        let hub = |i: usize| self.nodes[i].project == "log" || self.nodes[i].project == "home";
        let mut parent: Vec<usize> = (0..n).collect();
        fn find(p: &mut Vec<usize>, x: usize) -> usize {
            let mut r = x;
            while p[r] != r {
                r = p[r];
            }
            let mut c = x;
            while p[c] != r {
                let nx = p[c];
                p[c] = r;
                c = nx;
            }
            r
        }
        for &(a, b, _) in &self.edges {
            if hub(a) || hub(b) {
                continue;
            }
            let (ra, rb) = (find(&mut parent, a), find(&mut parent, b));
            if ra != rb {
                parent[ra] = rb;
            }
        }
        let mut groups: std::collections::BTreeMap<usize, Vec<usize>> = Default::default();
        for i in 0..n {
            if !hub(i) {
                let r = find(&mut parent, i);
                groups.entry(r).or_default().push(i);
            }
        }
        let mut out: Vec<(String, usize)> = Vec::new();
        for (_, members) in groups {
            if members.len() < 2 {
                continue;
            }
            let mut words: std::collections::BTreeMap<String, usize> = Default::default();
            for &m in &members {
                let w = self.nodes[m].label.split('-').next().unwrap_or("").to_string();
                *words.entry(w).or_default() += 1;
            }
            let name = words.into_iter().max_by(|a, b| a.1.cmp(&b.1).then(b.0.cmp(&a.0))).map(|x| x.0).unwrap_or_default();
            out.push((name, members.len()));
        }
        out.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        out
    }

    /// The open note's neighbourhood: notes linking to it (parents), notes it links to (children), and notes in the
    /// same folder (siblings).
    pub fn neighbourhood(&self, v: &Vault, i: usize) -> (Vec<usize>, Vec<usize>, Vec<usize>) {
        let mut parents: Vec<usize> = self.edges.iter().filter(|e| e.1 == i).map(|e| e.0).collect();
        let mut children: Vec<usize> = self.edges.iter().filter(|e| e.0 == i).map(|e| e.1).collect();
        parents.sort();
        parents.dedup();
        children.sort();
        children.dedup();
        let dir = |r: &str| -> String {
            let p: Vec<&str> = r.split('/').collect();
            if p.len() >= 3 { p[..2].join("/") } else { String::new() }
        };
        let mine = v.notes.get(self.nodes.get(i).map(|n| n.note).unwrap_or(0)).map(|n| dir(&n.rel)).unwrap_or_default();
        let siblings: Vec<usize> = if mine.is_empty() {
            Vec::new()
        } else {
            self.nodes.iter().enumerate().filter(|(j, nd)| *j != i && !parents.contains(j) && !children.contains(j) && dir(&v.notes[nd.note].rel) == mine).map(|(j, _)| j).take(6).collect()
        };
        (parents, children, siblings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn real_counts_and_a_steady_layout() {
        let d = tempfile::tempdir().unwrap();
        crate::vault::sample(d.path());
        let v = Vault::scan(d.path());
        let g = build(&v);
        assert_eq!(g.nodes.len(), v.notes.len());
        let g2 = build(&v);
        assert_eq!(g.nodes, g2.nodes, "the same place every time");
        assert!(g.nodes.iter().all(|n| (-1.0..=1.0).contains(&n.x) && (-1.0..=1.0).contains(&n.y)));
        let store = v.find("hearth/mockups/hearth-store/README.md").unwrap();
        assert!(g.most_linked(3).iter().any(|x| x.0 == store));
        assert_eq!(g.nodes[store].picture.as_deref(), Some("store-1-home.png"));
        assert!(g.orphans().contains(&v.find("README.md").unwrap()), "the top README links nowhere");
        let (parents, children, _) = g.neighbourhood(&v, store);
        assert!(parents.contains(&v.find("log/2026-11-19.md").unwrap()));
        assert!(children.contains(&v.find("hearth/assets/1.16-store-shots/README.md").unwrap()));
        assert!(g.clusters().iter().any(|c| c.1 == 2), "{:?}", g.clusters());
    }
}
