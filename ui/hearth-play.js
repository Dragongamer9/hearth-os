// Hearth's own take on Hydra Launcher's features (https://github.com/hydralauncher/hydra, MIT), written from scratch
// for Hearth's Game Mode: see docs/HYDRA-FEATURES.md. Loaded after index.html's own script; everything here works with
// a controller (A, B, the D-pad, LB/RB), touch and a mouse, like the rest of Game Mode.
//  - Discover (menu): suggestions from what you play, games you haven't started, free and open-source games.
//  - A game's page: description, genres, system requirements, how long it takes to beat (IGDB, your own key),
//    achievements from official services (Steam Web API, RetroAchievements: your own key), and Change pictures
//    (SteamGridDB, your own key).
//  - Downloads › Other downloads: Hearth's download manager for legitimate downloads (a web address, free and
//    open-source games, your itch.io games), with a queue, pause and resume, and a speed limit.
//  - Library › Add a game › Find games in a folder.
//  - Friends › Activity: your gaming profile, shared with your Hearth friends when you turn it on.
//  - Settings › Games › Integrations (your own keys, all off until you type one) and Appearance › Theme files.
// The core's side is hosts/hearth-core/src/play (calls named play.*). Not built, on purpose: download sources and
// repack feeds, warez and file-hoster sites, debrid, torrents and crack achievement files.
"use strict";

const hpOn = () => !ANDROID && (LINUX || !host);
const HP = {disc:null, discBusy:false, discErr:null, det:{}, ach:{}, keys:null, dl:null, dlTimer:null, dlLast:'', foss:null, get:'foss',
  itch:{tab:'lib', lib:null, res:null, q:'', busy:false, err:null}, find:null, findSel:new Set(), findBusy:false, art:{}, act:null, fact:null, themes:null};

// ---------- the core, or the browser preview's stand-ins ----------
function hpCall(method, args={}, timeout=60000){ return host ? call(method, args, timeout) : HP_MOCK.call(method, args); }

const HP_FOSS = [['supertuxkart','SuperTuxKart','Kart racing with Tux and friends: story mode, online races and battles.','GPL-3.0'],
  ['openra-ra','OpenRA: Red Alert','The classic Command & Conquer: Red Alert, rebuilt for today.','GPL-3.0'],['endless-sky','Endless Sky','Explore the galaxy as a trader, a mercenary or a pirate.','GPL-3.0'],
  ['shattered-pd','Shattered Pixel Dungeon','A roguelike dungeon crawler with pixel art.','GPL-3.0'],['unciv','Unciv','A turn-based empire builder like Civilization V.','MPL-2.0']];
const HP_MOCK = (()=>{
  const wait = ms => new Promise(r=>setTimeout(r,ms));
  const keys = {steamgriddb:{on:false,fields:{key:null}}, igdb:{on:true,fields:{clientId:'abc123',clientSecret:'••••9f2c'}}, steamweb:{on:false,fields:{key:null,steamId:null}},
    retro:{on:true,fields:{user:'Dragon',key:'••••k3y9'}}, itch:{on:false,fields:{key:null}}};
  let jobs = [
    {id:'d1', name:'SuperTuxKart-1.4-linux-x86_64.tar.xz', state:'downloading', done:312e6, total:700e6, pct:45, bytesPerSec:8.4e6, source:'foss', kind:'unpack'},
    {id:'d2', name:'Celeste Classic.zip', state:'queued', done:0, total:4.2e6, pct:0, source:'itch', kind:'unpack'},
    {id:'d3', name:'OpenRA-Red-Alert-x86_64.AppImage', state:'done', done:86e6, total:86e6, pct:100, source:'foss', kind:'play'}];
  const settings = {limitKBps:0, parallel:1, folder:'/home/you/Downloads/Hearth'};
  const view = () => ({settings:{...settings}, jobs:JSON.parse(JSON.stringify(jobs)), active:jobs.some(j=>j.state==='downloading'||j.state==='queued')});
  const svg = (h, w, ht) => 'data:image/svg+xml,'+encodeURIComponent(`<svg xmlns="http://www.w3.org/2000/svg" width="${w}" height="${ht}"><defs><linearGradient id="g" x2="1" y2="1"><stop offset="0" stop-color="hsl(${h},60%,40%)"/><stop offset="1" stop-color="hsl(${h+60},60%,18%)"/></linearGradient></defs><rect width="100%" height="100%" fill="url(#g)"/></svg>`);
  let share = false;
  const H = {
    'play.keys.get': async()=>JSON.parse(JSON.stringify(keys)),
    'play.keys.set': async({service,field,value})=>{ const k=keys[service]; k.fields[field]=value?(/key|Secret/.test(field)?'••••'+value.slice(-4):value):null; k.on=Object.values(k.fields).every(Boolean); return JSON.parse(JSON.stringify(keys)); },
    'play.keys.on': async({service,on})=>{ const k=keys[service]; if(on && !Object.values(k.fields).every(Boolean)) throw new Error('Type its key first.'); k.on=!!on; return JSON.parse(JSON.stringify(keys)); },
    'play.keys.clear': async({service})=>{ const k=keys[service]; Object.keys(k.fields).forEach(f=>k.fields[f]=null); k.on=false; return JSON.parse(JSON.stringify(keys)); },
    'play.details': async({id})=>{ await wait(150); const g=G(id)||{title:'Game'}; return {found:true, title:g.title, description:[`${g.title} is a game in your library. This is the browser preview's sample text: on Hearth OS it comes from the game's Steam store page.`, 'A second paragraph about what makes it worth playing.'],
      genres:[g.genre||'Adventure','Indie'], developers:['Preview Studio'], publishers:['Preview Games'], released:'12 Mar, 2024',
      requirements:{pc:{minimum:['• OS: Windows 10','• Processor: 4-core 3 GHz','• Memory: 8 GB RAM','• Graphics: GTX 1060 / RX 580','• Storage: 20 GB'], recommended:['• Memory: 16 GB RAM','• Graphics: RTX 2070 / RX 6700']}, linux:{minimum:['• OS: Ubuntu 22.04 or newer','• Memory: 8 GB RAM']}},
      timeToBeat:{rushed:6.8, normally:10, completely:22, count:420, source:'IGDB'}}; },
    'play.achievements': async({id})=>{ await wait(120); const g=G(id)||{}; if(g.src!=='emu') return {none:true, why:g.src==='gog'?'GOG doesn\'t offer achievements to other apps without each game\'s own GOG Galaxy sign-in, so Hearth can\'t show them yet.':'No official source for this game\'s achievements.'};
      return {done:2, total:4, source:'RetroAchievements', list:[{name:'First Steps',desc:'Clear the first level',done:true,at:Date.now()-864e5,icon:svg(40,64,64)},{name:'Collector',desc:'Find 50 coins',done:true,at:Date.now()-3*864e5,icon:svg(90,64,64)},{name:'Speedy',desc:'Finish in under an hour',done:false,icon:svg(200,64,64)},{name:'Completionist',desc:'Find everything',done:false,icon:svg(300,64,64)}]}; },
    'play.discover': async()=>{ await wait(250);
      const un=GAMES.filter(g=>!g.hrs&&!g.last&&!g.cloud&&!g.hidden).sort((a,b)=>(b.inst?1:0)-(a.inst?1:0)).slice(0,24).map(g=>g.id);
      const re=GAMES.filter(g=>g.hrs>=1&&g.last&&Date.now()-g.last>30*864e5).sort((a,b)=>b.hrs-a.hrs).slice(0,16).map(g=>g.id);
      const top=[...GAMES].filter(g=>g.hrs).sort((a,b)=>b.hrs-a.hrs)[0];
      const items=['Lantern Keep','Copperline','Starfall Tactics','Moss Hollow','Neon Courier','Deep Signal'].map((t,i)=>({store:'steam', id:String(900100+i), title:t, wide:svg(i*50,460,215), price:i%3?{final:'$'+(9.99+i*5).toFixed(2), base:null, discount:0}:{final:'$7.49', base:'$14.99', discount:50}}));
      const shelves=[];
      if(un.length) shelves.push({id:'unplayed', title:'Not played yet', sub:'In your library, waiting for a first go', kind:'library', items:un});
      if(re.length) shelves.push({id:'revisit', title:'Pick up again', sub:'Games you put time into, not played in a while', kind:'library', items:re});
      if(top) shelves.push({id:'like-122', title:`Because you play ${top.title}`, sub:'Best-selling RPG games on Steam you don\'t have', kind:'store', items:items.slice(0,4)});
      shelves.push({id:'popular', title:'Popular games you haven\'t played', sub:'Steam\'s top sellers, minus what you have', kind:'store', items:items.slice(2)});
      const pick=GAMES.filter(g=>g.inst&&!g.hrs); return {shelves, surprise:pick.length?pick[0].id:null}; },
    'play.dl.list': async()=>view(),
    'play.dl.add': async({url})=>{ if(!/^https?:\/\/\S+$/.test(url||'')) throw new Error('That isn\'t a web address Hearth can download (it starts with https://).'); const name=decodeURIComponent(url.split(/[?#]/)[0].split('/').pop()||'download'); const j={id:'d'+Date.now(), name, state:'queued', done:0, total:null, source:'url', kind:/\.(appimage|exe|sh)$/i.test(name)?'play':/\.(zip|tar\.\w+|7z)$/i.test(name)?'unpack':''}; jobs.push(j); return j; },
    'play.dl.pause': async({id})=>{ const j=jobs.find(x=>x.id===id); if(j&&(j.state==='downloading'||j.state==='queued')) j.state='paused'; return view(); },
    'play.dl.resume': async({id})=>{ const j=jobs.find(x=>x.id===id); if(j&&(j.state==='paused'||j.state==='failed')) j.state='queued'; return view(); },
    'play.dl.cancel': async({id})=>{ jobs=jobs.filter(x=>x.id!==id); return view(); },
    'play.dl.first': async({id})=>{ const i=jobs.findIndex(x=>x.id===id); if(i>0) jobs.unshift(...jobs.splice(i,1)); return view(); },
    'play.dl.clear': async()=>{ jobs=jobs.filter(j=>j.state!=='done'); return view(); },
    'play.dl.settings': async(a)=>{ if(a.limitKBps!=null) settings.limitKBps=a.limitKBps; if(a.parallel!=null) settings.parallel=Math.max(1,Math.min(3,a.parallel)); return view(); },
    'play.dl.addToLibrary': async({id})=>{ const j=jobs.find(x=>x.id===id); const g={id:'manual:'+id, title:j.name.replace(/\.[^.]+$/,'').replace(/[-_]/g,' '), src:'manual', inst:true, hrs:0, last:0, added:Date.now(), art:{}, size:'—', state:'installed'}; GAMES.push(g); return {id:g.id, title:g.title, source:'manual', installed:true}; },
    'play.dl.unpack': async({id})=>{ await wait(300); return {folder:'/home/you/Downloads/Hearth/'+id, found:[{path:'/home/you/Downloads/Hearth/'+id+'/run_game.sh', title:'run game', kind:'linux', size:4096, folder:id}]}; },
    'play.foss.list': async()=>HP_FOSS.map(([id,title,about,license])=>({id,title,about,license})),
    'play.foss.get': async({id})=>{ await wait(200); const f=HP_FOSS.find(x=>x[0]===id); const j={id:'d'+Date.now(), name:f[1].replace(/\W+/g,'-')+'-linux-x86_64.AppImage', state:'queued', done:0, total:9e7, pct:0, source:'foss', kind:'play'}; jobs.push(j); return j; },
    'play.itch.library': async()=>{ if(!keys.itch.on) throw new Error('Connect itch.io in Settings › Games › Integrations (with your own itch.io API key).'); return [{id:5,title:'Celeste Classic',free:true,keyId:99,by:'maddy',about:'Climb a mountain'}]; },
    'play.itch.search': async({q})=>{ if(!keys.itch.on) throw new Error('Connect itch.io in Settings › Games › Integrations.'); return [{id:7,title:q+' (free)',free:true,by:'someone'}]; },
    'play.itch.uploads': async()=>[{id:1,name:'game-linux.zip',label:'Linux',size:4.2e6,linux:true},{id:2,name:'game-win.zip',label:'Windows',size:4.6e6,windows:true}],
    'play.itch.get': async({name})=>H['play.dl.add']({url:'https://itch.zone/'+encodeURIComponent(name||'game.zip')}),
    'play.finder.scan': async()=>{ await wait(200); return {dirs:['/home/you/Games','/home/you/Downloads'], found:[{path:'/home/you/Games/Hollow Pines/HollowPines.exe',title:'HollowPines',kind:'windows',size:52e6,folder:'Hollow Pines'},{path:'/home/you/Games/stk/run_game.sh',title:'run game',kind:'linux',size:2048,folder:'stk'},{path:'/home/you/Downloads/OpenRA-Red-Alert-x86_64.AppImage',title:'OpenRA Red Alert x86 64',kind:'appimage',size:86e6,folder:'Downloads'}]}; },
    'play.finder.pick': async()=>null,
    'play.finder.add': async({paths})=>({added:(paths||[]).map((p,i)=>{ const g={id:'manual:f'+i+Date.now(), title:p.split('/').pop().replace(/\.[^.]+$/,''), src:'manual', inst:true, hrs:0, last:0, added:Date.now(), art:{}, size:'—', state:'installed'}; GAMES.push(g); return {id:g.id, title:g.title, source:'manual', installed:true}; }), errors:[]}),
    'play.art.search': async()=>{ if(!keys.steamgriddb.on) throw new Error('Turn on SteamGridDB in Settings › Games › Integrations (it needs your own free API key).'); await wait(150);
      const L=(w,h)=>[0,1,2,3,4].map(i=>({url:svg(i*70,w,h), thumb:svg(i*70,w,h)})); return {game:{id:1,name:'Preview'}, others:[], cover:L(600,900), wide:L(920,430), hero:L(1920,620), logo:L(400,160)}; },
    'play.art.fetch': async({url})=>({file:url}),
    'play.art.use': async({id,kind,file})=>{ const g=G(id); g.art=Object.assign({},g.art,{[kind]:file}); return null; },
    'play.art.clear': async({id})=>{ const g=G(id); if(g) g.art={}; return null; },
    'play.activity.mine': async()=>{ const played=GAMES.filter(g=>g.hrs||g.last); const c=g=>({id:g.id,title:g.title,source:g.src,minutes:Math.round((g.hrs||0)*60),lastPlayed:g.last||0});
      return {playing:S.running&&G(S.running)?c(G(S.running)):null, recent:played.filter(g=>g.last>Date.now()-30*864e5).sort((a,b)=>b.last-a.last).slice(0,8).map(c), top:[...played].sort((a,b)=>b.hrs-a.hrs).slice(0,5).map(c),
        totalMinutes:Math.round(played.reduce((t,g)=>t+(g.hrs||0)*60,0)), gamesPlayed:played.length, playedThisWeek:played.filter(g=>g.last>Date.now()-7*864e5).length, librarySize:GAMES.length}; },
    'play.activity.friends': async()=>[{userId:'@sarah:matrix.org', playing:{title:'Stardew Valley',minutes:3120}, recent:[{title:'Stardew Valley',minutes:3120,lastPlayed:Date.now()-2e6},{title:'Hades',minutes:900,lastPlayed:Date.now()-2*864e5}], top:[{title:'Stardew Valley',minutes:3120}], totalMinutes:5400, gamesPlayed:12, at:Date.now()-6e5}],
    'play.activity.share': async({on})=>{ share=!!on; return {settings:{shareActivity:share}}; },
    'play.activity.publish': async()=>({sent:share?1:0, off:!share}),
    'play.themes.list': async()=>({folder:'/home/you/.local/share/nimbus/themes', themes:[{id:'example-ember', name:'Ember (example)', accent:'#ff8a3d', css:':root{--accent:#ff8a3d;--accent2:#e06a1f;--accentInk:#2a1204}'},{id:'ocean', name:'Ocean', accent:'#2288ff', css:':root{--accent:#2288ff;--accent2:#1a6fd0;--accentInk:#03142a}'}]}),
  };
  return {call:(m,a)=>H[m] ? H[m](a||{}) : Promise.reject(new Error(m+' (preview)')), share:()=>share};
})();

// ---------- looks ----------
document.head.insertAdjacentHTML('beforeend', `<style id="hp-css">
.hp-top{display:flex;gap:8px;flex-wrap:wrap;align-items:center;margin:4px 0 6px}
.hp-sub{color:var(--dim);font-size:13px;margin:-6px 0 0}
.hp-tags{margin:8px 0 2px}
.hp-req{display:grid;grid-template-columns:repeat(auto-fit,minmax(220px,1fr));gap:12px;margin-top:6px}
.hp-req h4{margin:0 0 6px;font-size:13px;color:var(--accent);text-transform:uppercase;letter-spacing:.05em}
.hp-req div div{color:var(--dim);font-size:13px;line-height:1.5}
.hp-ttb{display:flex;gap:10px;flex-wrap:wrap;margin-top:6px}
.hp-ttb div{flex:1;min-width:110px;background:var(--panel2);border-radius:10px;padding:10px 12px}
.hp-ttb b{display:block;font-size:22px} .hp-ttb small{color:var(--dim)}
.hp-foss{flex:none;width:clamp(220px,22vw,300px);text-align:left;border-radius:10px;background:var(--panel);padding:14px 16px;display:flex;flex-direction:column;gap:6px;min-height:120px}
.hp-foss b{font-size:15px} .hp-foss small{color:var(--dim);font-size:12.5px;line-height:1.4} .hp-foss i{font-style:normal;font-size:11px;color:var(--accent);font-weight:700}
.hp-foss[data-f].focus{transform:scale(1.03)}
.hp-job{display:flex;align-items:center;gap:12px;padding:12px 14px;background:var(--panel);border-radius:var(--r);margin-bottom:8px;max-width:900px}
.hp-job .rt{flex:1;min-width:0} .hp-job .rt b{display:block;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
.hp-job .rt small{color:var(--dim);font-size:12.5px} .hp-job .bar{margin-top:6px}
.hp-job .hp-ic{width:38px;height:38px;border-radius:9px;background:var(--panel2);display:grid;place-items:center;flex:none}
.hp-job .hp-ic svg{width:20px;height:20px;stroke:currentColor;fill:none;stroke-width:1.8}
.hp-job.failed .rt small{color:var(--danger)}
.hp-art{flex:none;border-radius:10px;overflow:hidden;background:var(--panel2);position:relative}
.hp-art img{width:100%;height:100%;object-fit:cover;display:block}
.hp-art.cover{width:clamp(110px,11vw,150px);aspect-ratio:2/3} .hp-art.wide{width:clamp(220px,22vw,300px);aspect-ratio:460/215}
.hp-art.hero{width:clamp(280px,30vw,420px);aspect-ratio:3/1} .hp-art.logo{width:clamp(200px,20vw,280px);aspect-ratio:5/2;background:repeating-conic-gradient(var(--panel2) 0 25%,var(--panel3,#2a3140) 0 50%) 0 0/16px 16px}
.hp-art.logo img{object-fit:contain}
.hp-art[data-f].focus{transform:scale(1.04)}
.hp-prof{display:grid;grid-template-columns:repeat(auto-fit,minmax(150px,1fr));gap:10px;margin:6px 0 12px;max-width:900px}
.hp-prof div{background:var(--panel);border-radius:var(--r);padding:12px 14px} .hp-prof b{display:block;font-size:22px} .hp-prof small{color:var(--dim)}
.hp-fcard{background:var(--panel);border-radius:var(--r);padding:12px 16px;margin-bottom:8px;max-width:900px}
.hp-fcard small{color:var(--dim)}
.hp-chk{width:22px;height:22px;border-radius:6px;border:2px solid var(--dim);flex:none;display:grid;place-items:center;font-weight:800}
.hp-chk.on{background:var(--accent);border-color:var(--accent);color:var(--accentInk)}
</style>`);
I.discover = '<svg viewBox="0 0 24 24"><circle cx="12" cy="12" r="8.5"/><path d="m15.5 8.5-2 5-5 2 2-5z"/></svg>';
I.folder = I.folder || '<svg viewBox="0 0 24 24"><path d="M3.5 7.5v10a2 2 0 0 0 2 2h13a2 2 0 0 0 2-2v-8a2 2 0 0 0-2-2h-7l-2-2h-4a2 2 0 0 0-2 2z"/></svg>';
const hpBytes = b => b==null||!(b>0) ? '' : b>=1e9 ? (b/1e9).toFixed(b>=1e10?0:1)+' GB' : b>=1e6 ? (b/1e6).toFixed(b>=1e8?0:1)+' MB' : Math.max(1,Math.round(b/1e3))+' KB';
const hpHours = h => h==null ? '—' : h<1 ? Math.round(h*60)+' min' : (h<10?h.toFixed(1):Math.round(h))+' hrs';
const hpMins = m => hrs((m||0)/60);

// ---------- where it shows up ----------
if(hpOn()){
  Object.assign(VIEWS, {discover:hpViewDiscover, hpget:hpViewGet, hpfind:hpViewFind, hpart:hpViewArt});
  Object.assign(TITLES, {discover:'Discover', hpget:'Get games', hpfind:'Find games in a folder', hpart:'Pictures'});
  STABS.splice(STABS.findIndex(x=>x[0]==='import')+1, 0, ['integrations','link','Integrations']); STAB_GROUP.integrations='games';
  SORDER.splice(SORDER.indexOf('import')+1, 0, 'integrations');
  SKW.integrations = 'api key steamgriddb igdb how long to beat retroachievements itch steam web api achievements artwork pictures';
}
/** The main menu's entries (index.html renderSideMenu and the dock). */
function hpMenu(){ return hpOn() ? [['discover','discover','Discover']] : []; }
function hpDock(add, cur){ if(hpOn()) add('discover','discover','Discover','data-act="nav" data-view="discover"', {cur:cur('discover')}); }

// Library › Add a game: also Find games in a folder and Download a game.
const hpAddManualOrig = addManual;
addManual = function(){
  if(ANDROID || !hpOn()) return hpAddManualOrig();
  openSheet({title:'Add a game', sub:'Windows games run with Proton. Pictures can come from SteamGridDB (Settings › Games › Integrations).', items:[
    {label:'Choose a program, AppImage or .exe…', icon:'plus', run:()=>setTimeout(hpAddManualOrig,50)},
    {label:'Find games in a folder', icon:'search', run:()=>{ closeAll(); navigate('hpfind'); }},
    {label:'Download a free game', icon:'dl', run:()=>{ closeAll(); navigate('hpget'); }}]});
};

// When a game ends: your shared activity is sent again (only when Share my activity is on).
const hpSysEventOrig = onSysEvent;
onSysEvent = function(name, data){
  const r = hpSysEventOrig(name, data);
  if(name==='game.exited' && hpOn() && host && hpShareOn()) setTimeout(()=>hpCall('play.activity.publish',{},30000).catch(()=>{}), 3000);
  return r;
};
function hpShareOn(){ return host ? !!(MF.st && MF.st.settings && MF.st.settings.shareActivity) : HP_MOCK.share(); }

// ---------- the action dispatcher (index.html act(): data-act="hy…") ----------
function hyAct(a, d, el){
  switch(a){
    case 'hyShop': SHOP.store=d.s; closeAll(); return navigate('shop', {shopPage:{store:d.s, id:d.id, item:(HP.disc&&HP.disc.shelves||[]).flatMap(s=>s.kind==='store'?s.items:[]).find(i=>i.id===d.id)||null}});
    case 'hySurprise': { const id=HP.disc&&HP.disc.surprise; if(!id||!G(id)) return toast('Nothing new to try: every installed game has been played'); toast(`How about ${G(id).title}?`); return navigate('game',{game:id}); }
    case 'hyDiscRefresh': HP.disc=null; hpLoadDiscover(true); return render('dc-refresh',true);
    case 'hyFoss': return hpFossAsk(d.id);
    case 'hyGetTab': HP.get=d.v; if(d.v==='itch' && !HP.itch.lib) hpItchLoad(); return render('hpg-'+d.v, true);
    case 'hyItchTab': HP.itch.tab=d.v; if(d.v==='lib' && !HP.itch.lib) hpItchLoad(); return render('hpi-'+d.v, true);
    case 'hyItchSearch': return openPrompt({title:'Search itch.io for free games', value:HP.itch.q||''}).then(q=>{ if(q==null) return; HP.itch.q=q.trim(); HP.itch.res=null; HP.itch.err=null; render(null,true);
      hpCall('play.itch.search',{q:HP.itch.q},60000).then(r=>{ HP.itch.res=r||[]; }).catch(e=>{ HP.itch.err=e.message; }).finally(()=>render(null,true)); });
    case 'hyItchGame': return hpItchFiles(+d.id, d.key?+d.key:null, d.t);
    case 'hyUrl': return openPrompt({title:'Download from a web address', mode:'url', placeholder:'https://…', enterLabel:'Download'}).then(u=>{ if(!u) return;
      hpCall('play.dl.add',{url:u.trim()},30000).then(j=>{ toast(`Downloading ${j.name}`); hpDlLoad(); if(S.view!=='downloads') navigate('downloads'); }).catch(e=>toast(e.message)); });
    case 'hyDlPause': case 'hyDlResume': case 'hyDlCancel': case 'hyDlFirst':
      return hpCall('play.dl.'+a.slice(4).toLowerCase(),{id:d.id},20000).then(v=>{ HP.dl=v; render(null,true); }).catch(e=>toast(e.message));
    case 'hyDlClear': return hpCall('play.dl.clear',{},20000).then(v=>{ HP.dl=v; render(null,true); });
    case 'hyDlAdd': return hpCall('play.dl.addToLibrary',{id:d.id},60000).then(dto=>{ if(!dto) return; if(host){ const old=G(dto.id); if(old) Object.assign(old, fromHost(dto)); else GAMES.push(fromHost(dto)); } toast(`Added ${dto.title} to your library`); render(null,true); }).catch(e=>toast(e.message));
    case 'hyDlUnpack': toast('Unpacking…'); return hpCall('play.dl.unpack',{id:d.id},30*60e3).then(r=>{ HP.find={dirs:[r.folder], found:r.found||[]}; HP.findSel=new Set((r.found||[]).map(f=>f.path)); toast(r.found&&r.found.length?'Unpacked. Pick what to add to your library.':'Unpacked, but no game program was found in it.'); navigate('hpfind'); }).catch(e=>toast(e.message));
    case 'hyDlLimit': return openSheet({title:'Download speed limit', sub:'For Hearth\'s own downloads here (Steam, Epic and GOG have their own).', items:[[0,'No limit'],[1024,'1 MB/s'],[5120,'5 MB/s'],[10240,'10 MB/s'],[25600,'25 MB/s']].map(([v,l])=>({label:l, icon:(HP.dl&&HP.dl.settings.limitKBps===v)?'check':'dl', run:()=>hpCall('play.dl.settings',{limitKBps:v},20000).then(x=>{ HP.dl=x; render(null,true); toast(v?`Limited to ${l}`:'No speed limit'); })}))});
    case 'hyDlPar': return openSheet({title:'How many at once', items:[1,2,3].map(n=>({label:n===1?'One at a time':`${n} at once`, icon:(HP.dl&&HP.dl.settings.parallel===n)?'check':'dl', run:()=>hpCall('play.dl.settings',{parallel:n},20000).then(x=>{ HP.dl=x; render(null,true); })}))});
    case 'hyFindPick': return hpCall('play.finder.pick',{},600000).then(r=>{ if(!r) return; HP.find=r; HP.findSel=new Set(); render(null,true); }).catch(e=>toast(e.message));
    case 'hyFindAgain': HP.find=null; hpFindLoad(); return render('hf-again',true);
    case 'hyFindSel': { const p=HP.find&&HP.find.found[+d.i]; if(!p) return; HP.findSel.has(p.path)?HP.findSel.delete(p.path):HP.findSel.add(p.path); return render(null,true); }
    case 'hyFindAdd': { const paths=[...HP.findSel]; if(!paths.length) return toast('Pick the games to add first (A on each)');
      return hpCall('play.finder.add',{paths},120000).then(r=>{ if(host) (r.added||[]).forEach(dto=>{ if(!G(dto.id)) GAMES.push(fromHost(dto)); });
        toast(`Added ${(r.added||[]).length} game${(r.added||[]).length===1?'':'s'} to your library${(r.errors||[]).length?` (${r.errors[0]})`:''}`);
        HP.find.found=HP.find.found.filter(f=>!HP.findSel.has(f.path)); HP.findSel=new Set(); render(null,true); }).catch(e=>toast(e.message)); }
    case 'hyArt': return navigate('hpart', {game:d.id});
    case 'hyArtUse': { const gid=S.game; toast('Saving the picture…');
      return hpCall('play.art.fetch',{url:d.url},90000).then(r=>hpCall('play.art.use',{id:gid, kind:d.kind, file:r.file},20000)).then(dto=>{ if(dto&&host){ const g=G(gid); if(g) Object.assign(g, fromHost(dto)); } toast('Picture changed'); render(null,true); }).catch(e=>toast(e.message)); }
    case 'hyArtClear': return hpCall('play.art.clear',{id:S.game},20000).then(dto=>{ if(dto&&host){ const g=G(S.game); if(g) Object.assign(g, fromHost(dto)); } toast('Back to the store\'s pictures'); render(null,true); }).catch(e=>toast(e.message));
    case 'hyArtName': return openPrompt({title:'Search SteamGridDB by name', value:(HP.art[S.game]&&HP.art[S.game].q)||(G(S.game)||{}).title||''}).then(q=>{ if(q){ hpArtLoad(S.game, q.trim()); render('ar-name',true); } });
    case 'hyKeySet': return hpKeySet(d.s, d.field, d.secret==='1', d.label);
    case 'hyKeyOn': { const on=!(HP.keys&&HP.keys[d.s]&&HP.keys[d.s].on); return hpCall('play.keys.on',{service:d.s, on},15000).then(k=>{ HP.keys=k; render(null,true); }).catch(e=>toast(e.message)); }
    case 'hyKeyClear': return openSheet({title:'Forget this key?', items:[{label:'Forget it', icon:'trash', run:()=>hpCall('play.keys.clear',{service:d.s},15000).then(k=>{ HP.keys=k; render(null,true); toast('Forgotten'); })}]});
    case 'hyTheme': return hpThemeUse(d.v);
    case 'hyShare': { const on=!hpShareOn(); return hpCall('play.activity.share',{on},30000).then(st=>{ if(host && st) MF.st=st; else if(host && MF.st && MF.st.settings) MF.st.settings.shareActivity=on;
      toast(on?'Your activity goes to your friends now':'Your activity stays on this device'); if(on) hpCall('play.activity.publish',{},30000).catch(()=>{}); render(null,true); }).catch(e=>toast(e.message)); }
    case 'hyPublish': return hpCall('play.activity.publish',{},30000).then(r=>toast(r&&r.off?'Turn on Share my activity first':r&&r.same?'Your friends have this already':`Sent to ${r&&r.sent||0} friend${(r&&r.sent)===1?'':'s'}`)).catch(e=>toast(e.message));
    case 'hyIntegrations': return setOpen('integrations');
  }
}

// ---------- Discover ----------
function hpLoadDiscover(force){
  if(HP.discBusy || (HP.disc && !force)) return;
  HP.discBusy=true; HP.discErr=null;
  if(!HP.foss) hpCall('play.foss.list',{},15000).then(f=>{ HP.foss=f; }).catch(()=>{});
  hpCall('play.discover',{},180000).then(r=>{ HP.disc=r; }).catch(e=>{ HP.discErr=e.message; }).finally(()=>{ HP.discBusy=false; if(S.view==='discover') render(null,true); });
}
function hpStoreCard(it, key){
  const own=shopOwned(it), p=it.price;
  return `<button class="sft" data-f data-k="${esc(key)}" data-act="hyShop" data-s="${esc(it.store)}" data-id="${esc(it.id)}">
    <div class="sfa" style="${artBg({title:it.title||'?'})}">${shopImg([it.wide,it.thumb,it.tall])}${p&&p.discount?`<span class="sfd">-${p.discount}%</span>`:''}${own?`<span class="sfo">${own.inst?'Installed':'In your library'}</span>`:''}</div>
    <div class="sft-t">${esc(it.title)}</div><div class="sft-p">${shopPrice(p)}</div></button>`;
}
function hpFossCard(f, key){ return `<button class="hp-foss" data-f data-k="${esc(key)}" data-act="hyFoss" data-id="${esc(f.id)}"><i>FREE · ${esc(f.license||'Open source')}</i><b>${esc(f.title)}</b><small>${esc(f.about||'')}</small></button>`; }
function hpViewDiscover(){
  hpLoadDiscover();
  const r=HP.disc;
  let h=`<div class="hp-top"><button class="pill" data-f data-k="dc-surprise" data-act="hySurprise">${I.discover} Surprise me</button>
    <button class="pill" data-f data-k="dc-get" data-act="nav" data-view="hpget">${I.dl} Free games &amp; downloads</button>
    <button class="pill ghost" data-f data-k="dc-refresh" data-act="hyDiscRefresh">${I.refresh} ${HP.discBusy?'Looking…':'Refresh'}</button></div>`;
  if(!r && HP.discBusy) h += note('Finding games for you…');
  if(HP.discErr && !r) h += note(esc(HP.discErr));
  for(const s of (r&&r.shelves)||[]){
    const items = s.kind==='library' ? s.items.map(G).filter(Boolean) : s.items;
    if(!items.length) continue;
    h += `<h2>${esc(s.title)}</h2>${s.sub?`<div class="hp-sub">${esc(s.sub)}</div>`:''}<div class="shelf">${items.map((it,i)=>s.kind==='library'?tile(it,'p',`dc-${s.id}-${i}`):hpStoreCard(it,`dc-${s.id}-${i}`)).join('')}</div>`;
  }
  if(HP.foss&&HP.foss.length) h += `<h2>Free and open-source games</h2><div class="hp-sub">From each project's own releases. Hearth only downloads games you're entitled to.</div><div class="shelf">${HP.foss.map((f,i)=>hpFossCard(f,'dc-foss-'+i)).join('')}</div>`;
  if(r && !(r.shelves||[]).length) h += `<div class="empty"><div class="muted">Play a few games and Discover suggests more like them.</div></div>`;
  return h;
}
function hpFossAsk(id){
  const f=(HP.foss||[]).find(x=>x.id===id); if(!f) return;
  openSheet({title:`Download ${f.title}?`, sub:`${f.about||''} Free and open source (${f.license||''}), from the project's own latest release on GitHub. It goes to Downloads › Other downloads; then add it to your library.`, items:[
    {label:'Download', icon:'dl', run:()=>hpCall('play.foss.get',{id},120000).then(j=>{ toast(`Downloading ${j.name}`); hpDlLoad(); }).catch(e=>toast(e.message))}]});
}

// ---------- a game's page (index.html viewGame: hpGameCards(g)) ----------
function hpGameCards(g){
  if(!hpOn() || !g || g.cloud) return '';
  if(!HP.det[g.id]){ HP.det[g.id]={loading:true}; hpCall('play.details',{id:g.id},90000).then(r=>{ HP.det[g.id]=r||{}; }).catch(()=>{ HP.det[g.id]={}; }).finally(()=>{ if(S.view==='game'&&S.game===g.id) render(null,true); }); }
  // Achievements from an official service when Hearth has none from Steam's own files.
  const gi = GINFO[g.id];
  if(!HP.ach[g.id] && (!host || (gi && !gi.loading)) && !(gi && gi.achievements) && !g.ach){
    HP.ach[g.id]={loading:true};
    hpCall('play.achievements',{id:g.id},60000).then(r=>{ HP.ach[g.id]=r||{}; if(r && r.list && r.list.length){ GINFO[g.id]=Object.assign(GINFO[g.id]||{}, {achievements:r}); } })
      .catch(e=>{ HP.ach[g.id]={none:true, why:e.message}; }).finally(()=>{ if(S.view==='game'&&S.game===g.id) render(null,true); });
  }
  const d=HP.det[g.id]||{}, a=HP.ach[g.id]||{};
  let out='';
  if(d.found){
    const desc=(d.description||[]).slice(0,3).map(p=>`<p>${esc(p)}</p>`).join('');
    const facts=[d.developers&&d.developers.length?'By '+esc(d.developers.join(', ')):'', d.released?'Released '+esc(d.released):''].filter(Boolean).join(' · ');
    out += `<div class="card"><h3>More about it</h3><div class="shop-desc">${desc||'<p>No description.</p>'}</div>
      ${(d.genres||[]).length?`<div class="hp-tags">${d.genres.map(x=>`<span class="shop-tag">${esc(x)}</span>`).join('')}</div>`:''}
      ${facts?`<div class="muted" style="font-size:13px">${facts}</div>`:''}<div class="muted" style="font-size:12px;margin-top:6px">From the game's Steam store page</div></div>`;
    const req=d.requirements||{}, block=(k,l)=>req[k]?`<div><h4>${l}</h4>${['minimum','recommended'].filter(x=>req[k][x]).map(x=>`<div><b style="font-size:13px">${x==='minimum'?'Minimum':'Recommended'}</b>${req[k][x].map(t=>`<div>${esc(t)}</div>`).join('')}</div>`).join('')}</div>`:'';
    if(req.pc||req.linux) out += `<div class="card"><h3>System requirements</h3><div class="hp-req">${block('pc','PC (Windows, runs with Proton)')}${block('linux','Linux')}</div></div>`;
  }
  const t=d.timeToBeat;
  if(t && !t.none) out += `<div class="card"><h3>How long to beat</h3><div class="hp-ttb"><div><b>${hpHours(t.rushed)}</b><small>Main story, rushed</small></div><div><b>${hpHours(t.normally)}</b><small>Normally</small></div><div><b>${hpHours(t.completely)}</b><small>Everything</small></div></div><div class="muted" style="font-size:12px;margin-top:6px">From ${esc(t.source||'IGDB')}${t.count?`, ${t.count} players`:''}</div></div>`;
  else if(d.igdbOff && !d.loading) out += `<div class="card"><h3>How long to beat</h3><div class="muted">Connect IGDB in Settings › Games › Integrations (with your own free Twitch developer app) to see how long games take.</div><button class="pill" style="margin-top:10px" data-f data-k="g-hp-igdb" data-act="hyIntegrations">${I.link} Integrations</button></div>`;
  out += `<div class="card"><h3>Pictures</h3><div class="muted">Pick its cover, banner and logo from SteamGridDB's community art.</div><button class="pill" style="margin-top:10px" data-f data-k="g-hp-art" data-act="hyArt" data-id="${esc(g.id)}">${I.layers||I.layout} Change pictures</button>
    ${a.none && a.why && !(gi&&gi.achievements) && !g.ach ? `<div class="muted" style="font-size:12.5px;margin-top:12px">Achievements: ${esc(a.why)}</div>` : ''}</div>`;
  return out;
}

// ---------- Pictures (SteamGridDB) ----------
function hpArtLoad(id, q){
  HP.art[id]={busy:true, q:q||null};
  hpCall('play.art.search',{id, query:q||''},90000).then(r=>{ HP.art[id]=Object.assign(r||{}, {q:q||null}); }).catch(e=>{ HP.art[id]={err:e.message, q:q||null}; }).finally(()=>{ if(S.view==='hpart') render(null,true); });
}
function hpViewArt(){
  const g=G(S.game); if(!g) return '';
  if(!HP.art[g.id]) hpArtLoad(g.id);
  const r=HP.art[g.id]||{};
  let h=`<div class="hp-top"><b style="font-size:18px">${esc(g.title)}</b><button class="pill" data-f data-k="ar-name" data-act="hyArtName">${I.search} Search another name</button><button class="pill ghost" data-f data-k="ar-clear" data-act="hyArtClear">${I.restart} Use the store's pictures</button></div>`;
  if(r.busy) return h + note('Looking on SteamGridDB…');
  if(r.err) return h + note(esc(r.err)) + `<button class="pill" data-f data-k="ar-int" data-act="hyIntegrations">${I.link} Integrations</button>`;
  if(r.game) h += `<div class="hp-sub">SteamGridDB: ${esc(r.game.name)}. Pictures are made by its community.</div>`;
  for(const [k,l] of [['cover','Cover'],['wide','Banner'],['hero','Background'],['logo','Logo']]){
    const L=r[k]||[]; if(!L.length) continue;
    h += `<h2>${l}</h2><div class="shelf">${L.map((x,i)=>`<button class="hp-art ${k}" data-f data-k="ar-${k}-${i}" data-act="hyArtUse" data-kind="${k}" data-url="${esc(x.url)}"><img src="${esc(x.thumb||x.url)}" alt="" loading="lazy"></button>`).join('')}</div>`;
  }
  return h;
}

// ---------- Downloads › Other downloads (index.html viewDownloads: hpDownloadsSection()) ----------
function hpDlLoad(quiet){
  return hpCall('play.dl.list',{},15000).then(v=>{ const s=JSON.stringify(v); if(s===HP.dlLast) return; HP.dlLast=s; HP.dl=v; if(S.view==='downloads') render(null,true); }).catch(()=>{});
}
function hpDlPoll(){
  if(HP.dlTimer) return;
  HP.dlTimer=setInterval(()=>{ if(S.view!=='downloads'){ clearInterval(HP.dlTimer); HP.dlTimer=null; return; } hpDlLoad(true); }, 1500);
}
const HP_STATE = {downloading:'Downloading', queued:'Queued', paused:'Paused', failed:'Stopped', done:'Downloaded'};
const HP_SRC = {url:'Web address', foss:'Free and open source', itch:'itch.io'};
function hpJobRow(j){
  const id=esc(j.id), act=(x,ic,l)=>`<button class="ib" data-f data-k="hj-${x}-${id}" data-act="${x}" data-id="${id}" aria-label="${l}" title="${l}">${ic}</button>`;
  const meta=[HP_STATE[j.state]||j.state, j.total?`${j.state==='done'?'':hpBytes(j.done)+' of '}${hpBytes(j.total)}`:hpBytes(j.done), j.state==='downloading'&&j.bytesPerSec>1e4?fmtSpeed(j.bytesPerSec):'', HP_SRC[j.source]||''].filter(Boolean).join(' · ');
  let btn='';
  if(j.state==='downloading'||j.state==='queued') btn = act('hyDlFirst',I.up,'Start first') + act('hyDlPause',I.pause,'Pause') + act('hyDlCancel',I.x,'Cancel');
  else if(j.state==='paused'||j.state==='failed') btn = act('hyDlResume',I.play,'Resume') + act('hyDlCancel',I.x,'Cancel');
  else if(j.state==='done') btn = (j.kind==='play'?`<button class="pill" data-f data-k="hj-add-${id}" data-act="hyDlAdd" data-id="${id}">${I.plus} Add to library</button>`:j.kind==='unpack'?`<button class="pill" data-f data-k="hj-unp-${id}" data-act="hyDlUnpack" data-id="${id}">${I.layers||I.dl} Unpack</button>`:'') + act('hyDlCancel',I.x,'Take off the list');
  return `<div class="hp-job ${j.state}"><div class="hp-ic">${j.state==='done'?I.check:I.dl}</div><div class="rt"><b>${esc(j.name)}</b><small>${esc(j.state==='failed'&&j.error?j.error:meta)}</small>
    ${j.state!=='done'&&j.pct!=null?`<div class="bar"><i style="width:${j.pct}%"></i></div>`:''}</div>${btn}</div>`;
}
function hpDownloadsSection(){
  if(!hpOn()) return '';
  if(!HP.dl) hpDlLoad();
  hpDlPoll();
  const v=HP.dl||{jobs:[], settings:{}}, st=v.settings||{};
  const lim = st.limitKBps ? (st.limitKBps>=1024?Math.round(st.limitKBps/1024)+' MB/s':st.limitKBps+' KB/s') : 'No limit';
  let h=`<div class="dl-h"><h2>Other downloads${v.jobs.length?` · ${v.jobs.length}`:''}</h2>${v.jobs.some(j=>j.state==='done')?`<button class="pill ghost" data-f data-k="hj-clear" data-act="hyDlClear">Clear finished</button>`:''}</div>`;
  h += v.jobs.length ? v.jobs.map(hpJobRow).join('') : note('Free and open-source games, your itch.io games, or any file you have a link to. Hearth only downloads from legitimate sources.');
  h += `<div class="rows" style="margin-top:10px">
    <button class="row" data-f data-k="hd-url" data-act="hyUrl"><span class="ricon">${I.globe}</span><div class="rt"><b>Download from a web address</b><small>A free game's release, or a file from your own store account</small></div><div class="rv">›</div></button>
    <button class="row" data-f data-k="hd-get" data-act="nav" data-view="hpget"><span class="ricon">${I.discover}</span><div class="rt"><b>Free games</b><small>Free and open-source games, and itch.io</small></div><div class="rv">›</div></button>
    <button class="row" data-f data-k="hd-find" data-act="nav" data-view="hpfind"><span class="ricon">${I.search}</span><div class="rt"><b>Find games in a folder</b><small>Add games you already have on this device</small></div><div class="rv">›</div></button>
    <button class="row" data-f data-k="hd-limit" data-act="hyDlLimit"><span class="ricon">${I.sliders}</span><div class="rt"><b>Speed limit</b><small>For these downloads</small></div><div class="rv">${esc(lim)}</div></button>
    <button class="row" data-f data-k="hd-par" data-act="hyDlPar"><span class="ricon">${I.layers||I.dl}</span><div class="rt"><b>At once</b><small>How many download together</small></div><div class="rv">${st.parallel||1}</div></button></div>`;
  if(st.folder) h += note(`Saved in <code>${esc(st.folder)}</code>. Paused downloads carry on where they stopped, also after a restart.`);
  return h;
}

// ---------- Get games: free and open source, itch.io, a web address ----------
function hpItchLoad(){
  HP.itch.busy=true; HP.itch.err=null;
  hpCall('play.itch.library',{},90000).then(r=>{ HP.itch.lib=r||[]; }).catch(e=>{ HP.itch.err=e.message; }).finally(()=>{ HP.itch.busy=false; if(S.view==='hpget') render(null,true); });
}
function hpItchFiles(game, keyId, title){
  toast('Asking itch.io for its files…');
  hpCall('play.itch.uploads',{game, keyId},60000).then(L=>{
    if(!L||!L.length) return toast('itch.io has no downloads for that game (it may be a browser game).');
    openSheet({title:`Download ${title||'this game'}`, sub:'Pick a file. Linux builds come first; Windows builds run with Proton.', items:L.slice(0,8).map(u=>({label:`${u.label||u.name}${u.size?' · '+hpBytes(u.size):''}${u.demo?' · demo':''}`, icon:'dl',
      run:()=>hpCall('play.itch.get',{upload:u.id, keyId, name:u.name, size:u.size||0},60000).then(j=>{ toast(`Downloading ${j.name}`); hpDlLoad(); }).catch(e=>toast(e.message))}))});
  }).catch(e=>toast(e.message));
}
function hpViewGet(){
  if(!HP.foss) hpCall('play.foss.list',{},15000).then(f=>{ HP.foss=f; if(S.view==='hpget') render(null,true); }).catch(()=>{});
  const tabs=`<div class="chips">${[['foss','Free & open source'],['itch','itch.io'],['url','Web address']].map(([k,l])=>`<button class="chip ${HP.get===k?'on':''}" data-f data-k="hpg-${k}" data-act="hyGetTab" data-v="${k}">${l}</button>`).join('')}</div>`;
  let b='';
  if(HP.get==='foss'){
    b = note('Free and open-source games, each downloaded from its own project\'s latest release on GitHub (the Linux build).') + `<div class="rows">${(HP.foss||[]).map((f,i)=>`<button class="row" data-f data-k="hpf-${i}" data-act="hyFoss" data-id="${esc(f.id)}"><span class="ricon">${I.dl}</span><div class="rt"><b>${esc(f.title)}</b><small>${esc(f.about||'')} · ${esc(f.license||'')}</small></div><div class="rv">›</div></button>`).join('')}</div>`;
  } else if(HP.get==='itch'){
    const sub=`<div class="chips">${[['lib','Your itch.io library'],['free','Free games']].map(([k,l])=>`<button class="chip ${HP.itch.tab===k?'on':''}" data-f data-k="hpi-${k}" data-act="hyItchTab" data-v="${k}">${l}</button>`).join('')}</div>`;
    const list=(L,p)=>`<div class="rows">${L.map((x,i)=>`<button class="row" data-f data-k="hpi-${p}-${i}" data-act="hyItchGame" data-id="${x.id}" data-key="${x.keyId||''}" data-t="${esc(x.title)}"><span class="ricon">${I.dl}</span><div class="rt"><b>${esc(x.title)}</b><small>${esc([x.by?'by '+x.by:'', x.free?'Free':'', x.about||''].filter(Boolean).join(' · '))}</small></div><div class="rv">›</div></button>`).join('')}</div>`;
    b = sub;
    if(HP.itch.err) b += note(esc(HP.itch.err)) + `<button class="pill" data-f data-k="hpi-int" data-act="hyIntegrations">${I.link} Integrations</button>`;
    else if(HP.itch.tab==='lib') b += HP.itch.busy ? note('Loading your itch.io library…') : HP.itch.lib ? (HP.itch.lib.length ? list(HP.itch.lib,'l') : note('Your itch.io library is empty.')) : '';
    else b += `<button class="pill" style="margin:6px 0 10px" data-f data-k="hpi-search" data-act="hyItchSearch">${I.search} ${HP.itch.q?`“${esc(HP.itch.q)}”`:'Search free games'}</button>` + (HP.itch.res ? (HP.itch.res.length ? list(HP.itch.res,'r') : note('No free games found.')) : '');
    b += note('Paid games are bought on itch.io itself; then they show in your library here.');
  } else {
    b = `<div class="rows"><button class="row" data-f data-k="hpu-url" data-act="hyUrl"><span class="ricon">${I.globe}</span><div class="rt"><b>Type or paste a web address</b><small>http or https. Pause, resume and a speed limit work for every download.</small></div><div class="rv">›</div></button></div>`
      + note('Your GOG games already install through Hearth (Library). For a GOG offline installer, download it in GOG\'s own page (the store window), then use Find games in a folder or Add a game: Windows installers run with Proton.')
      + note('Hearth doesn\'t read download sources, repack sites or file hosters, and has no torrent or debrid support: it only downloads games you\'re entitled to.');
  }
  return tabs + b;
}

// ---------- Find games in a folder ----------
function hpFindLoad(){
  HP.findBusy=true;
  hpCall('play.finder.scan',{},120000).then(r=>{ HP.find=r; HP.findSel=new Set(); }).catch(e=>{ HP.find={dirs:[], found:[], err:e.message}; }).finally(()=>{ HP.findBusy=false; if(S.view==='hpfind') render(null,true); });
}
function hpViewFind(){
  if(!HP.find && !HP.findBusy) hpFindLoad();
  const f=HP.find, n=HP.findSel.size;
  let h=`<div class="hp-top"><button class="pill" data-f data-k="hf-add" data-act="hyFindAdd">${I.plus} Add ${n||''} game${n===1?'':'s'}</button>
    <button class="pill" data-f data-k="hf-pick" data-act="hyFindPick">${I.folder} Choose a folder…</button><button class="pill ghost" data-f data-k="hf-again" data-act="hyFindAgain">${I.refresh} Look again</button></div>`;
  if(HP.findBusy || !f) return h + note('Looking for games…');
  if(f.err) h += note(esc(f.err));
  h += note(`Looked in ${f.dirs&&f.dirs.length?f.dirs.map(d=>`<code>${esc(d)}</code>`).join(', '):'nothing (no Games or Downloads folder)'}. Installers, uninstallers and games already in your library are left out.`);
  if(!f.found.length) return h + `<div class="empty"><div class="muted">No games found here.</div></div>`;
  const kind={windows:'Windows (runs with Proton)', appimage:'AppImage', linux:'Linux'};
  return h + `<div class="rows">${f.found.map((x,i)=>`<button class="row" data-f data-k="hf-${i}" data-act="hyFindSel" data-i="${i}"><span class="hp-chk ${HP.findSel.has(x.path)?'on':''}">${HP.findSel.has(x.path)?'✓':''}</span><div class="rt"><b>${esc(x.title)}</b><small>${esc(kind[x.kind]||x.kind)} · ${esc(x.folder||'')}${x.size?' · '+hpBytes(x.size):''}</small></div></button>`).join('')}</div>`;
}

// ---------- Settings › Games › Integrations, and Appearance › Theme files (index.html settingsBody: hpSettingsBody) ----------
const HP_SERVICES = [
  ['steamgriddb','SteamGridDB','Pictures for games you added yourself, and for any game you like.','Get a free API key at steamgriddb.com › Preferences › API.',[['key','API key',1]]],
  ['igdb','IGDB: how long to beat','How long games take to beat, on game pages.','Make a free app at dev.twitch.tv/console and copy its Client ID and a new secret.',[['clientId','Client ID',0],['clientSecret','Client secret',1]]],
  ['steamweb','Steam Web API','Steam achievements on game pages when they aren\'t on this device.','Your key from steamcommunity.com/dev/apikey and your 17-digit SteamID64. Hearth never signs in to Steam.',[['key','Web API key',1],['steamId','SteamID64',0]]],
  ['retro','RetroAchievements','Achievements for your retro games (play them with RetroAchievements on in RetroArch).','Your user name, and your web API key from retroachievements.org › Settings.',[['user','User name',0],['key','Web API key',1]]],
  ['itch','itch.io','Download your itch.io games and free games with Hearth\'s download manager.','Make an API key at itch.io › Settings › API keys.',[['key','API key',1]]],
];
function hpKeySet(s, field, secret, label){
  openPrompt(Object.assign({title:label, secret, enterLabel:'Save'}, secret?{mode:'password'}:{})).then(v=>{ if(v==null) return;
    hpCall('play.keys.set',{service:s, field, value:v.trim()},15000).then(k=>{ HP.keys=k; render(null,true); toast(v.trim()?'Saved':'Cleared'); }).catch(e=>toast(e.message)); });
}
function hpPageIntegrations(){
  if(!HP.keys) hpCall('play.keys.get',{},15000).then(k=>{ HP.keys=k; if(S.view==='settings') render(null,true); }).catch(()=>{});
  const K=HP.keys||{};
  let h = note('Optional services for more on game pages, more pictures and more downloads. Each uses your own free key or account name, typed here, and is off until you do. Keys stay on this device (in your profile\'s accounts folder) and only go to that service.');
  for(const [s,name,what,how,fields] of HP_SERVICES){
    const k=K[s]||{on:false, fields:{}};
    h += shead(name) + sgrid(`<button class="row" data-f data-k="in-${s}-on" data-act="hyKeyOn" data-s="${s}"><div class="rt"><b>${esc(name)}</b><small>${esc(what)}</small></div><div class="sw ${k.on?'on':''}"></div></button>`
      + fields.map(([f,l,sec])=>`<button class="row" data-f data-k="in-${s}-${f}" data-act="hyKeySet" data-s="${s}" data-field="${f}" data-secret="${sec}" data-label="${esc(name+': '+l)}"><div class="rt"><b>${esc(l)}</b><small>${k.fields&&k.fields[f]?'Set':'Not set'}</small></div><div class="rv">${esc((k.fields&&k.fields[f])||'Type it')}</div></button>`).join('')
      + (Object.values(k.fields||{}).some(Boolean)?`<button class="row" data-f data-k="in-${s}-x" data-act="hyKeyClear" data-s="${s}"><div class="rt"><b>Forget</b><small>Removes what's kept for ${esc(name)}</small></div><div class="rv">${I.trash}</div></button>`:''))
      + note(esc(how));
  }
  return h + note('Hydra Launcher\'s "download sources", debrid services and achievement files from cracks are not part of Hearth: it only uses official services and legitimate downloads.');
}
function hpThemeApply(t){
  let el=document.getElementById('hp-theme');
  if(!t){ if(el) el.remove(); return; }
  if(!el){ el=document.createElement('style'); el.id='hp-theme'; document.head.appendChild(el); }
  el.textContent=t.css||'';
}
function hpThemeUse(id){
  try{ id ? localStorage.setItem('hearth.themeFile', id) : localStorage.removeItem('hearth.themeFile'); }catch(e){}
  const t=id && HP.themes && (HP.themes.themes||[]).find(x=>x.id===id);
  hpThemeApply(t||null);
  toast(t?`${t.name} theme file`:'No theme file');
  render(null,true);
}
function hpThemeSaved(){ try{ return localStorage.getItem('hearth.themeFile')||''; }catch(e){ return ''; } }
function hpThemesLoad(){ return hpCall('play.themes.list',{},15000).then(r=>{ HP.themes=r; const id=hpThemeSaved(); if(id) hpThemeApply((r.themes||[]).find(x=>x.id===id)||null); if(S.view==='settings') render(null,true); }).catch(()=>{}); }
function hpThemeSection(){
  if(!HP.themes) hpThemesLoad();
  const L=(HP.themes&&HP.themes.themes)||[], cur=hpThemeSaved();
  const card=(id,name,accent)=>`<button class="theme ${cur===id?'on':''}" data-f data-k="ap-tf-${esc(id||'none')}" data-act="hyTheme" data-v="${esc(id)}"><div class="tprev" style="background:linear-gradient(135deg,var(--panel2),var(--bg))"><i style="background:${esc(accent||'var(--dim)')}"></i></div><small>${esc(name)}</small></button>`;
  return shead('Theme files') + `<div class="opts">${card('', 'None', '')}${L.map(t=>card(t.id,t.name,t.accent)).join('')}</div>`
    + note(`Your own CSS themes, from <code>${esc((HP.themes&&HP.themes.folder)||'~/.local/share/nimbus/themes')}</code> (an example is there to copy). A theme file goes over the theme above; it can't load anything from the internet.`);
}
function hpSettingsBody(t, body){
  if(!hpOn()) return body;
  if(t==='integrations') return hpPageIntegrations();
  if(t==='appearance') return body + hpThemeSection();
  return body;
}

// ---------- Friends › Activity (index.html mfMain: hpActivityTab()) ----------
function hpActivityLoad(){
  hpCall('play.activity.mine',{},15000).then(r=>{ HP.act=r; if(S.view==='friends') render(null,true); }).catch(()=>{});
  hpCall('play.activity.friends',{},30000).then(r=>{ HP.fact=r||[]; if(S.view==='friends') render(null,true); }).catch(e=>{ HP.fact={err:e.message}; if(S.view==='friends') render(null,true); });
}
function hpGameLine(x){ return `${esc(x.title)}${x.minutes?` · ${hpMins(x.minutes)}`:''}${x.lastPlayed?` · ${ago(x.lastPlayed)}`:''}`; }
function hpActivityTab(){
  if(!hpOn()) return note('Activity is on Hearth OS.');
  if(!HP.actAt || Date.now()-HP.actAt>30000){ HP.actAt=Date.now(); hpActivityLoad(); }
  const m=HP.act, on=hpShareOn();
  let h='';
  if(m){
    h += `<div class="hp-prof"><div><b>${m.playing?esc(m.playing.title):'—'}</b><small>Playing now</small></div><div><b>${hpMins(m.totalMinutes)}</b><small>Total play time</small></div><div><b>${m.gamesPlayed}</b><small>Games played</small></div><div><b>${m.playedThisWeek}</b><small>Played this week</small></div></div>`;
    if(m.recent.length) h += shead('Recently played') + `<div class="rows">${m.recent.map((x,i)=>`<button class="row" data-f data-k="ha-r-${i}" data-act="game" data-id="${esc(x.id)}"><div class="rt"><b>${esc(x.title)}</b><small>${hpMins(x.minutes)} played · ${ago(x.lastPlayed)}</small></div><div class="rv">›</div></button>`).join('')}</div>`;
    if(m.top.length) h += shead('Most played') + `<div class="rows">${m.top.map((x,i)=>`<button class="row" data-f data-k="ha-t-${i}" data-act="game" data-id="${esc(x.id)}"><div class="rt"><b>${esc(x.title)}</b><small>${hpMins(x.minutes)}</small></div><div class="rv">›</div></button>`).join('')}</div>`;
  } else h += note('Loading your activity…');
  h += shead('Share with friends') + `<div class="rows"><button class="row" data-f data-k="ha-share" data-act="hyShare"><div class="rt"><b>Share my activity</b><small>Your friends see what you played lately and for how long (titles and play time only), sent end-to-end encrypted in your chats with them. Off until you turn it on.</small></div><div class="sw ${on?'on':''}"></div></button>
    ${on?`<button class="row" data-f data-k="ha-pub" data-act="hyPublish"><span class="ricon">${I.refresh}</span><div class="rt"><b>Send it now</b><small>It's also sent after each game</small></div></button>`:''}</div>`;
  const F=HP.fact;
  h += shead('Friends\' activity');
  if(F && F.err) h += note(esc(F.err));
  else if(!F) h += note('Loading…');
  else if(!F.length) h += note('No activity from friends yet. It shows here when a friend shares theirs.');
  else h += F.map(f=>{ const fr=(MF.friends||[]).find(x=>x.userId===f.userId); return `<div class="hp-fcard"><b>${esc(fr?fr.name:f.userId)}</b> <small>${f.playing?'· playing '+esc(f.playing.title):''}${f.at?' · '+ago(f.at):''}</small>
    <div style="margin-top:6px"><small>${hpMins(f.totalMinutes)} played · ${f.gamesPlayed||0} games</small></div>
    ${(f.recent||[]).length?`<div style="margin-top:6px">${f.recent.map(x=>`<div><small>${hpGameLine(x)}</small></div>`).join('')}</div>`:''}</div>`; }).join('');
  return h;
}

// ---------- at start ----------
if(hpOn()){
  if(hpThemeSaved()) hpThemesLoad();
  // Downloads left queued when Hearth closed carry on.
  if(host) setTimeout(()=>hpCall('play.dl.list',{},15000).then(v=>{ HP.dl=v; }).catch(()=>{}), 8000);
  // Your shared activity, once a little after start (only when Share my activity is on).
  if(host) setTimeout(()=>{ if(hpShareOn()) hpCall('play.activity.publish',{},30000).catch(()=>{}); }, 60000);
}
