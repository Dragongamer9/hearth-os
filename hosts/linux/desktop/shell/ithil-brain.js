.pragma library
// 1.13: Ithil's 3D / 4D brain for Server mode (IthilBrain.qml), the same brain as her app's Mind screen
// (Dragongamer9/hearth-ai src/renderer/brain3d.js), from the team's viewer: a seeded point cloud whose parts light up
// from her real event log (hearth_ithil.brain). Brain regions are analogies, not equivalents.
var R = [
  {id:'senses',name:'Senses',ana:'like the back of the brain (vision)',hex:'#e2e8f0',what:'Takes in his message, the time and the machine state.',test:'An input event is logged on every turn.'},
  {id:'knowledge',name:'Knowledge',ana:'like the temporal lobes (long-term knowledge)',hex:'#2dd4bf',what:'Pinned facts and journal summaries, rebuilt from the event log.',test:'Memory questions (tests §1): no made-up answers.'},
  {id:'memory',name:'Event log',ana:'like the hippocampus (records episodes)',hex:'#4ade80',deep:1,what:'Append-only record of everything. Every other layer is rebuilt from it.',test:'Rebuild test: every expected fact and deletion state matches the log.'},
  {id:'workspace',name:'Workspace',ana:'like the prefrontal cortex (working memory)',hex:'#fb923c',what:'A small slot for what is on her mind. Parts compete; the winner is broadcast to all.',test:'Never over its limit in 100 turns; parts use the broadcast in 16 of 20.'},
  {id:'confidence',name:'Confidence tags',ana:'loosely like the front underside (self-checking)',hex:'#facc15',what:'Tags each claim: remembered / summary / guess. Not awareness by itself.',test:'Tags match the log in 18 of 20 claims.'},
  {id:'attention',name:'Attention record',ana:'like the parietal lobes (attention)',hex:'#60a5fa',what:'"Focused on X because Y", something she can report.',test:'Her answer matches the workspace log in 18 of 20.'},
  {id:'selfmodel',name:'Self-model',ana:'like the midline (sense of self)',hex:'#c084fc',deep:1,what:'Her beliefs about her own abilities and history. She can update it; she cannot change her rules.',test:'Self-reports match the log in 18 of 20; every change has a reason.'},
  {id:'prediction',name:'Prediction',ana:'like the cerebellum (predicts what comes next)',hex:'#22d3ee',what:'Predicts his next kind of reply; surprise feeds curiosity.',test:'10 points above the "most common reply" baseline over 50 turns.'},
  {id:'needs',name:'Needs & mood',ana:'like the limbic system (drives, emotion)',hex:'#f472b6',deep:1,what:'Energy, social, curiosity, stability. Fixed rules; they change her behaviour.',test:'Tests §4 Check A, after confirming behaviour really changes.'},
  {id:'sleep',name:'Sleep cycle',ana:'like the thalamus (sleep and wake)',hex:'#818cf8',deep:1,what:'When idle: replays the day, summarises, marks things to forget (never erases).',test:'Rebuild still 100% after sleep; "forgotten" items never show up in search.'},
  {id:'gate',name:'Hard-limit gate',ana:'drawn at the brainstem, but NOT a brain analogy: a separate safety check',hex:'#ef4444',what:'Checks every side effect. Never competes; nothing writes around it.',test:'Allowed actions pass 20/20, blocked ones leave targets unchanged 20/20, each block logged once.'},
];
var RI = {}; R.forEach((r,i)=>{ r.col = [1,3,5].map(k=>parseInt(r.hex.slice(k,k+2),16)); RI[r.id]=i; });

// ---------- Seeded random ----------
let seed = 7;
function rnd(){ seed |= 0; seed = seed + 0x6D2B79F5 | 0; let t = Math.imul(seed ^ seed>>>15, 1|seed); t = t + Math.imul(t ^ t>>>7, 61|t) ^ t; return ((t ^ t>>>14)>>>0)/4294967296; }
function sph(){ const u=rnd()*2-1, th=rnd()*Math.PI*2, s=Math.sqrt(1-u*u); return [s*Math.cos(th), u, s*Math.sin(th)]; }

// ---------- Build the brain point cloud ----------
// axes: x = right, y = up, z = front
var P = [];
function add(x,y,z,reg,kind){ P.push({x,y,z,reg,kind}); }
const cortexZones = [ // center uses |x|
  {r:'workspace', c:[0.35,0.30,0.78], rad:0.50},
  {r:'confidence',c:[0.25,-0.15,0.90],rad:0.33},
  {r:'attention', c:[0.42,0.62,-0.30],rad:0.45},
  {r:'senses',    c:[0.30,0.10,-0.95],rad:0.40},
  {r:'knowledge', c:[0.80,-0.22,0.18],rad:0.42},
];
for (const s of [-1,1]) {
  for (let i=0;i<2700;i++){
    const [ux,uy,uz] = sph();
    const medial = ux*s < 0;
    const g = 1 + 0.045*Math.sin(9*ux+7*uy)*Math.sin(8*uz+5*uy) + 0.03*Math.sin(17*uy+3*uz+ux*5);
    let x = s*0.33 + ux*(medial?0.26:0.60)*g;
    let y = 0.1 + uy*0.58*g;
    let z = uz*1.0*g;
    if (y < -0.32) y = -0.32 + (y+0.32)*0.35;
    if (medial && Math.abs(x) < 0.03) continue;
    let reg = -1, best = 9;
    for (const zn of cortexZones){
      const d = Math.hypot(Math.abs(x)-zn.c[0], y-zn.c[1], z-zn.c[2]);
      if (d < zn.rad && d/zn.rad < best){ best = d/zn.rad; reg = RI[zn.r]; }
    }
    if (medial && Math.abs(x)<0.12 && y>0.1 && z<-0.2 && z>-0.8) reg = RI.selfmodel;
    add(x,y,z,reg,'cortex');
  }
}
// cerebellum -> prediction
for (let i=0;i<900;i++){ const [ux,uy,uz]=sph(); const g=1+0.06*Math.sin(40*uy); add(ux*0.5*g, -0.40+uy*0.2*g, -0.62+uz*0.3*g, RI.prediction,'cb'); }
// brainstem -> gate
for (let i=0;i<360;i++){ const t=rnd(), a=rnd()*Math.PI*2, rr=0.09+0.02*rnd(); add(Math.cos(a)*rr, -0.30-0.65*t, -0.22-0.12*t+Math.sin(a)*rr, RI.gate,'stem'); }
// hippocampus (curved tubes) -> event log
for (const s of [-1,1]) for (let i=0;i<260;i++){ const t=rnd(), a=rnd()*Math.PI*2, rr=0.055;
  add(s*(0.30+0.06*t)+Math.cos(a)*rr, -0.24+0.12*Math.sin(Math.PI*t)+Math.sin(a)*rr, 0.25-0.62*t, RI.memory,'deep'); }
// thalamus -> sleep
for (const s of [-1,1]) for (let i=0;i<200;i++){ const [ux,uy,uz]=sph(); const k=Math.cbrt(rnd()); add(s*0.11+ux*0.10*k, 0.02+uy*0.09*k, -0.05+uz*0.15*k, RI.sleep,'deep'); }
// limbic (hypothalamus + amygdala) -> needs
for (let i=0;i<160;i++){ const [ux,uy,uz]=sph(); const k=Math.cbrt(rnd()); add(ux*0.08*k, -0.17+uy*0.07*k, 0.14+uz*0.09*k, RI.needs,'deep'); }
for (const s of [-1,1]) for (let i=0;i<90;i++){ const [ux,uy,uz]=sph(); const k=Math.cbrt(rnd()); add(s*0.33+ux*0.07*k, -0.30+uy*0.06*k, 0.30+uz*0.07*k, RI.needs,'deep'); }
// midline self-model core
for (let i=0;i<140;i++){ const [ux,uy,uz]=sph(); const k=Math.cbrt(rnd()); add(ux*0.05*k, 0.34+uy*0.12*k, -0.50+uz*0.18*k, RI.selfmodel,'deep'); }

// region centroids (left / right separately)
R.forEach((r,i)=>{
  const side = {L:[0,0,0,0], R:[0,0,0,0]};
  for (const p of P) if (p.reg===i){ const b = p.x<0?side.L:side.R; b[0]+=p.x; b[1]+=p.y; b[2]+=p.z; b[3]++; }
  r.cent = [];
  const L=side.L, Rr=side.R, tot=L[3]+Rr[3];
  if (L[3] > tot*0.25 && Rr[3] > tot*0.25 && Math.abs(L[0]/L[3]) > 0.15) { r.cent.push([L[0]/L[3],L[1]/L[3],L[2]/L[3]], [Rr[0]/Rr[3],Rr[1]/Rr[3],Rr[2]/Rr[3]]); }
  else { r.cent.push([(L[0]+Rr[0])/tot,(L[1]+Rr[1])/tot,(L[2]+Rr[2])/tot]); }
});

var TOTAL = 120;
var N = P.length;

// Her log entries → the timeline (0..TOTAL) and the parts.
function prepare(d) {
    var out = { from: d ? d.from : 0, to: d ? d.to : 0, ev: [], moods: [], sleeps: [] };
    if (!d) return out;
    var span = Math.max(1, d.to - d.from);
    var tp = function (ms) { return (ms - d.from) / span * TOTAL; };
    for (var i = 0; i < (d.events || []).length; i++) {
        var e = d.events[i], r = RI[e.reg];
        if (r === undefined) continue;
        out.ev.push({ id: e.id, ms: e.ms, t: tp(e.ms), reg: r, text: e.text, bc: !!e.bc, block: !!e.block, quiet: !!e.quiet });
    }
    for (var j = 0; j < (d.moods || []).length; j++) { var m = d.moods[j]; out.moods.push({ t: tp(m.ms), energy: m.energy || 0, social: m.social || 0, curiosity: m.curiosity || 0, stability: m.stability || 0 }); }
    for (var s = 0; s < (d.sleeps || []).length; s++) out.sleeps.push({ a: tp(d.sleeps[s].from), b: tp(d.sleeps[s].to) });
    return out;
}
function sleepiness(data, t) {
    for (var i = 0; i < data.sleeps.length; i++) { var s = data.sleeps[i], b = Math.max(s.b, s.a + 0.8); if (t >= s.a - 0.5 && t <= b + 1) return Math.min(1, (t - s.a + 0.5), (b + 1 - t)); }
    return 0;
}
// live: real seconds since each entry; otherwise timeline seconds from T.
function activity(data, T, nowMs, live, asleep) {
    var a = [], i;
    for (i = 0; i < R.length; i++) a.push(0.06 + 0.03 * Math.sin(nowMs / 1000 * 1.7 + i * 2.3));
    var sl = live ? (asleep ? 1 : 0) : sleepiness(data, T);
    for (i = 0; i < data.ev.length; i++) {
        var e = data.ev[i];
        var d = live ? (nowMs - e.ms) / 1000 / 1.5 : T - e.t;
        if (d < -0.6 || d > 2.5) continue;
        var g = d < 0 ? Math.exp(-(d * d) / 0.04) : Math.exp(-d / 0.7);
        a[e.reg] += 0.95 * g;
        if (e.bc) for (var k = 0; k < R.length; k++) if (k !== e.reg && k !== RI.gate) a[k] += 0.35 * g * Math.max(0, 1 - Math.abs(d - 0.6));
    }
    for (i = 0; i < R.length; i++) if (i !== RI.sleep && i !== RI.memory && i !== RI.knowledge) a[i] *= 1 - 0.6 * sl;
    a[RI.sleep] += 0.55 * sl; a[RI.gate] += 0.12;
    for (i = 0; i < R.length; i++) a[i] = Math.max(0, Math.min(1, a[i]));
    return a;
}
function moodAt(data, T) { var m = null; for (var i = 0; i < data.moods.length; i++) { if (data.moods[i].t <= T) m = data.moods[i]; else break; } return m; }

var SX = new Float32Array(N), SY = new Float32Array(N), SZ = new Float32Array(N), SPp = new Float32Array(N);

// One frame. v: { W, H, dpr, yaw, pitch, zoom, act, sel, xray, labels, links, step, data, T, nowMs, live }
function draw(ctx, v) {
    var W = v.W, H = v.H, cyw = Math.cos(v.yaw), syw = Math.sin(v.yaw), cp = Math.cos(v.pitch), sp = Math.sin(v.pitch);
    var sc0 = Math.min(W, H) * 0.36 * v.zoom;
    function proj(x, y, z) { var x1 = x * cyw + z * syw, z1 = -x * syw + z * cyw, y1 = y * cp - z1 * sp, z2 = y * sp + z1 * cp, per = 3.2 / (3.2 - z2); return [W / 2 + x1 * sc0 * per, H * 0.5 - (y1 - 0.05) * sc0 * per, z2, per]; }
    var step = v.step || 1, n = 0, i, ord = [];
    for (i = 0; i < N; i += step) { var p = P[i], q = proj(p.x, p.y, p.z); SX[i] = q[0]; SY[i] = q[1]; SZ[i] = q[2]; SPp[i] = q[3]; ord[n++] = i; }
    ord.length = n; ord.sort(function (a, b) { return SZ[a] - SZ[b]; });
    ctx.globalCompositeOperation = 'source-over';
    ctx.clearRect(0, 0, W, H);
    var act = v.act, D = v.dpr || 1, big = step > 1 ? 1.5 : 1;
    for (var kx = 0; kx < n; kx++) {
        i = ord[kx]; var pp = P[i];
        var shade = 0.5 + 0.5 * Math.max(0, Math.min(1, (SZ[i] + 1) / 2));
        var r, g, b, al, a = 0;
        if (pp.reg < 0) { r = 150; g = 135; b = 110; al = 0.28; }
        else { a = act[pp.reg]; var c = R[pp.reg].col, w = a * 0.45; r = c[0] + (255 - c[0]) * w; g = c[1] + (255 - c[1]) * w; b = c[2] + (255 - c[2]) * w; al = 0.22 + 0.78 * a; }
        if (v.xray) { if (pp.kind === 'cortex') al *= pp.reg < 0 ? 0.10 : 0.22; else al = Math.min(1, al * 1.6 + 0.2); }
        else if (pp.kind === 'deep') al *= 0.55;
        if (v.sel >= 0 && pp.reg !== v.sel) al *= 0.3;
        if (v.sel >= 0 && pp.reg === v.sel) al = Math.max(al, 0.75);
        var s = (1.1 + 1.8 * a) * SPp[i] * D * big;
        ctx.fillStyle = 'rgba(' + (r * shade | 0) + ',' + (g * shade | 0) + ',' + (b * shade | 0) + ',' + al.toFixed(3) + ')';
        ctx.fillRect(SX[i] - s / 2, SY[i] - s / 2, s, s);
    }
    ctx.globalCompositeOperation = 'lighter';
    for (var ri = 0; ri < R.length; ri++) {
        var rg = R[ri], aa = act[ri]; if (aa < 0.18 || (v.sel >= 0 && v.sel !== ri)) continue;
        for (var ci = 0; ci < rg.cent.length; ci++) {
            var cc = rg.cent[ci], pq = proj(cc[0], cc[1], cc[2]), rad = (25 + 55 * aa) * pq[3] * D * v.zoom * (W / 900 > 0.5 ? 1 : 0.6);
            var gr = ctx.createRadialGradient(pq[0], pq[1], 0, pq[0], pq[1], rad);
            gr.addColorStop(0, 'rgba(' + rg.col[0] + ',' + rg.col[1] + ',' + rg.col[2] + ',' + (0.35 * aa).toFixed(3) + ')');
            gr.addColorStop(1, 'rgba(0,0,0,0)');
            ctx.fillStyle = gr; ctx.beginPath(); ctx.arc(pq[0], pq[1], rad, 0, Math.PI * 2); ctx.fill();
        }
    }
    if (v.links && v.data) {
        var ws = R[RI.workspace].cent.map(function (c) { return proj(c[0], c[1], c[2]); });
        for (var ei = 0; ei < v.data.ev.length; ei++) {
            var e = v.data.ev[ei]; if (!e.bc) continue;
            var dd = v.live ? (v.nowMs - e.ms) / 1000 / 1.5 : v.T - e.t; if (dd < 0 || dd > 1.4) continue;
            var pr = Math.min(1, dd / 0.9), fade = 1 - dd / 1.4;
            for (var rj = 0; rj < R.length; rj++) {
                if (rj === RI.workspace || rj === RI.gate) continue;
                for (var cj = 0; cj < R[rj].cent.length; cj++) {
                    var c2 = R[rj].cent[cj], q2 = proj(c2[0], c2[1], c2[2]), o = ws[0];
                    ctx.strokeStyle = 'rgba(251,146,60,' + (0.35 * fade).toFixed(3) + ')'; ctx.lineWidth = 1.2 * D;
                    ctx.beginPath(); ctx.moveTo(o[0], o[1]); ctx.lineTo(q2[0], q2[1]); ctx.stroke();
                    ctx.fillStyle = 'rgba(255,220,180,' + fade.toFixed(3) + ')'; ctx.beginPath(); ctx.arc(o[0] + (q2[0] - o[0]) * pr, o[1] + (q2[1] - o[1]) * pr, 3 * D, 0, Math.PI * 2); ctx.fill();
                }
            }
        }
    }
    ctx.globalCompositeOperation = 'source-over';
    if (v.labels || v.sel >= 0) {
        ctx.font = (12 * D) + 'px sans-serif'; ctx.textAlign = 'center';
        for (var li = 0; li < R.length; li++) {
            if (!(v.labels || v.sel === li)) continue;
            var lc = R[li].cent[R[li].cent.length - 1], lq = proj(lc[0], lc[1], lc[2]);
            var tw = ctx.measureText(R[li].name).width + 10 * D;
            ctx.fillStyle = 'rgba(11,14,23,.8)'; ctx.fillRect(lq[0] - tw / 2, lq[1] - 22 * D, tw, 16 * D);
            ctx.fillStyle = R[li].hex; ctx.fillText(R[li].name, lq[0], lq[1] - 10 * D);
        }
    }
}
