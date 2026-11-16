/* My Media: your PC's library through Jellyfin (Hearth Server's media server, docs/HOME-SERVER.md) or Plex.
   Both are turned into the same items: {id, kind: show|movie|episode|season|album|track, title, sub, year, overview,
   img, backdrop, pct, played, mins, seriesId, seasonId, index, parentIndex, rating, tags}. */
(function () {
  'use strict';
  var T = window.hearthTV;
  var M = T.Media = { kind: null, ready: false, name: '', error: '' };
  var DEVICE = 'Hearth TV';

  function deviceId() {
    var id = T.state.deviceId;
    if (!id) { id = 'hearthtv-' + Math.random().toString(36).slice(2, 12); T.setState({ deviceId: id }); }
    return id;
  }
  function session() {               // this profile's sign-in, or the TV's shared one
    var p = T.currentProfile(), s = T.state.media || {};
    return (p && s['p:' + p.id]) || s.shared || null;
  }
  M.session = session;
  function saveSession(sess, forProfile) {
    var s = Object.assign({}, T.state.media || {});
    if (forProfile) s['p:' + T.currentProfile().id] = sess; else s.shared = sess;
    T.setState({ media: s });
  }
  M.signOut = function (forProfile) {
    var s = Object.assign({}, T.state.media || {});
    if (forProfile) delete s['p:' + T.currentProfile().id]; else delete s.shared;
    T.setState({ media: s }); M.init();
  };
  function kidsOk(it) {
    if (!T.kids()) return true;
    var r = (it.rating || '').toUpperCase();
    return /^(G|PG|TV-Y|TV-Y7|TV-Y7-FV|TV-G|TV-PG|U|0|6|7|FSK-0|FSK-6)$/.test(r);
  }
  function filt(list) { return (list || []).filter(kidsOk); }

  // ---------------- Jellyfin ----------------
  var J = {};
  J.auth = function (token) {
    return 'MediaBrowser Client="Hearth TV", Device="' + DEVICE + '", DeviceId="' + deviceId() + '", Version="' + (T.n('version') || '1.0') + '"' + (token ? ', Token="' + token + '"' : '');
  };
  J.req = function (path, opt) {
    var s = session(); opt = opt || {};
    var h = { 'Content-Type': 'application/json', 'X-Emby-Authorization': J.auth(s && s.token), 'Authorization': J.auth(s && s.token) };
    return fetch((opt.server || s.server) + path, { method: opt.method || 'GET', headers: h, body: opt.body ? JSON.stringify(opt.body) : undefined })
      .then(function (r) { if (r.status === 401) { M.error = 'signedOut'; throw new Error('signed out'); } if (!r.ok) throw new Error('HTTP ' + r.status); return r.status === 204 ? null : r.text().then(function (t) { return t ? JSON.parse(t) : null; }); });
  };
  J.img = function (id, type, tag, w) { var s = session(); return tag ? s.server + '/Items/' + id + '/Images/' + type + '?maxWidth=' + (w || 640) + '&quality=85&tag=' + tag : ''; };
  J.item = function (x) {
    var s = session(), ud = x.UserData || {}, mins = x.RunTimeTicks ? x.RunTimeTicks / 600000000 : 0;
    var kind = { Series: 'show', Movie: 'movie', Episode: 'episode', Season: 'season', MusicAlbum: 'album', Audio: 'track' }[x.Type] || 'movie';
    var img = x.ImageTags && x.ImageTags.Thumb ? J.img(x.Id, 'Thumb', x.ImageTags.Thumb) :
      (kind === 'episode' && x.ImageTags && x.ImageTags.Primary) ? J.img(x.Id, 'Primary', x.ImageTags.Primary) :
      (x.BackdropImageTags && x.BackdropImageTags.length) ? J.img(x.Id, 'Backdrop', x.BackdropImageTags[0]) :
      (x.ParentBackdropImageTags && x.ParentBackdropImageTags.length) ? J.img(x.ParentBackdropItemId, 'Backdrop', x.ParentBackdropImageTags[0]) :
      (x.ImageTags && x.ImageTags.Primary) ? J.img(x.Id, 'Primary', x.ImageTags.Primary) : '';
    var backdrop = (x.BackdropImageTags && x.BackdropImageTags.length) ? J.img(x.Id, 'Backdrop', x.BackdropImageTags[0], 1920) :
      (x.ParentBackdropImageTags && x.ParentBackdropImageTags.length) ? J.img(x.ParentBackdropItemId, 'Backdrop', x.ParentBackdropImageTags[0], 1920) : img;
    var stream = (x.MediaStreams || []).filter(function (m) { return m.Type === 'Video'; })[0] || {};
    var tags = [];
    if (stream.Width >= 3800) tags.push('4K'); else if (stream.Width >= 1900) tags.push('HD');
    if (stream.VideoRange && stream.VideoRange !== 'SDR') tags.push('HDR');
    return {
      src: 'jellyfin', id: x.Id, kind: kind, title: x.Name || '', year: x.ProductionYear || '', overview: x.Overview || '',
      img: img, backdrop: backdrop, poster: x.ImageTags && x.ImageTags.Primary ? J.img(x.Id, 'Primary', x.ImageTags.Primary, 400) : '',
      pct: ud.PlayedPercentage || 0, played: !!ud.Played, posTicks: ud.PlaybackPositionTicks || 0, mins: mins,
      seriesId: x.SeriesId || '', seriesName: x.SeriesName || '', seasonId: x.SeasonId || x.ParentId || '', index: x.IndexNumber, parentIndex: x.ParentIndexNumber,
      rating: x.OfficialRating || '', genre: (x.Genres || [])[0] || '', childCount: x.ChildCount || x.RecursiveItemCount || 0, unplayed: ud.UnplayedItemCount || 0,
      artist: x.AlbumArtist || (x.Artists || [])[0] || '', added: x.DateCreated || '', tags: tags, favorite: !!ud.IsFavorite, created: Date.parse(x.DateCreated || '') || 0
    };
  };
  var FIELDS = 'Overview,ProductionYear,ChildCount,RecursiveItemCount,DateCreated,MediaStreams,Genres,OfficialRating,UserData,PrimaryImageAspectRatio';
  J.list = function (q) { var s = session(); return J.req('/Users/' + s.userId + '/Items?' + q + '&Fields=' + FIELDS + '&EnableImageTypes=Primary,Backdrop,Thumb').then(function (r) { return filt((r.Items || []).map(J.item)); }); };
  J.resume = function () { var s = session(); return J.req('/Users/' + s.userId + '/Items/Resume?Limit=12&MediaTypes=Video&Fields=' + FIELDS).then(function (r) { return filt((r.Items || []).map(J.item)); }); };
  J.nextUp = function (seriesId) { var s = session(); return J.req('/Shows/NextUp?userId=' + s.userId + '&Limit=12' + (seriesId ? '&seriesId=' + seriesId : '') + '&Fields=' + FIELDS).then(function (r) { return filt((r.Items || []).map(J.item)); }); };
  J.latest = function () { var s = session(); return J.req('/Users/' + s.userId + '/Items/Latest?Limit=16&Fields=' + FIELDS).then(function (r) { return filt((r || []).map(J.item)); }); };
  J.library = function () { return J.list('IncludeItemTypes=Series,Movie,MusicAlbum&Recursive=true&SortBy=DateCreated&SortOrder=Descending&Limit=400' + J.parentFilter()); };
  J.views = function () { var s = session(); return J.req('/Users/' + s.userId + '/Views').then(function (r) { return (r.Items || []).map(function (v) { return { id: v.Id, name: v.Name, type: v.CollectionType || '' }; }); }); };
  J.parentFilter = function () { return ''; };
  J.seasons = function (id) { var s = session(); return J.req('/Shows/' + id + '/Seasons?userId=' + s.userId + '&Fields=' + FIELDS).then(function (r) { return (r.Items || []).map(J.item); }); };
  J.episodes = function (id, seasonId) { var s = session(); return J.req('/Shows/' + id + '/Episodes?userId=' + s.userId + '&seasonId=' + seasonId + '&Fields=' + FIELDS).then(function (r) { return filt((r.Items || []).map(J.item)); }); };
  J.get = function (id) { var s = session(); return J.req('/Users/' + s.userId + '/Items/' + id + '?Fields=' + FIELDS).then(J.item); };
  J.similar = function (id) { var s = session(); return J.req('/Items/' + id + '/Similar?userId=' + s.userId + '&Limit=6&Fields=' + FIELDS).then(function (r) { return filt((r.Items || []).map(J.item)); }); };
  J.tracks = function (id) { return J.list('ParentId=' + id + '&SortBy=ParentIndexNumber,IndexNumber&IncludeItemTypes=Audio&Recursive=true'); };
  J.search = function (q) { return J.list('searchTerm=' + encodeURIComponent(q) + '&IncludeItemTypes=Movie,Series,MusicAlbum&Recursive=true&Limit=12'); };
  J.setPlayed = function (id, on) { var s = session(); return J.req('/Users/' + s.userId + '/PlayedItems/' + id, { method: on ? 'POST' : 'DELETE' }); };
  J.audioUrl = function (id) { var s = session(); return s.server + '/Audio/' + id + '/universal?UserId=' + s.userId + '&DeviceId=' + deviceId() + '&api_key=' + s.token + '&Container=mp3,aac,m4a,flac,ogg,opus&TranscodingContainer=aac&TranscodingProtocol=http&AudioCodec=aac&MaxStreamingBitrate=320000'; };
  /** What to play: Jellyfin decides direct play or HLS for this TV's browser, with its subtitle files. */
  J.playback = function (id, startTicks) {
    var s = session(), set = T.settings, maxRate = { original: 120000000, '1080p': 20000000, '720p': 6000000 }[set.media.quality] || 120000000;
    var profile = {
      MaxStreamingBitrate: maxRate, MaxStaticBitrate: maxRate,
      DirectPlayProfiles: [{ Container: 'mp4,m4v,mov', Type: 'Video', VideoCodec: 'h264,hevc,av1,vp9', AudioCodec: 'aac,mp3,ac3,eac3,opus,flac' }, { Container: 'webm', Type: 'Video', VideoCodec: 'vp8,vp9,av1', AudioCodec: 'opus,vorbis' }],
      TranscodingProfiles: [{ Container: 'ts', Type: 'Video', VideoCodec: 'h264,hevc', AudioCodec: 'aac,ac3,eac3', Protocol: 'hls', Context: 'Streaming', MaxAudioChannels: '6', BreakOnNonKeyFrames: true, MinSegments: 1 }],
      CodecProfiles: set.player.hdr === 'off' ? [{ Type: 'Video', Conditions: [{ Condition: 'EqualsAny', Property: 'VideoRangeType', Value: 'SDR', IsRequired: true }] }] : [],
      SubtitleProfiles: [{ Format: 'vtt', Method: 'External' }, { Format: 'srt', Method: 'External' }, { Format: 'ass', Method: 'External' }]
    };
    return J.req('/Items/' + id + '/PlaybackInfo?UserId=' + s.userId + '&StartTimeTicks=' + (startTicks || 0) + '&MaxStreamingBitrate=' + maxRate + '&AutoOpenLiveStream=true', { method: 'POST', body: { DeviceProfile: profile } }).then(function (r) {
      var ms = (r.MediaSources || [])[0]; if (!ms) throw new Error('nothing to play');
      var url = ms.SupportsDirectPlay || ms.SupportsDirectStream && !ms.TranscodingUrl
        ? s.server + '/Videos/' + id + '/stream?static=true&MediaSourceId=' + ms.Id + '&api_key=' + s.token + '&DeviceId=' + deviceId()
        : s.server + ms.TranscodingUrl + (ms.TranscodingUrl.indexOf('api_key') < 0 ? '&api_key=' + s.token : '');
      var subs = (ms.MediaStreams || []).filter(function (x) { return x.Type === 'Subtitle' && (x.IsExternal || x.DeliveryMethod === 'External' || x.SupportsExternalStream); }).map(function (x) {
        return { index: x.Index, lang: x.Language || '', label: x.DisplayTitle || x.Language || 'Subtitles', sdh: !!x.IsHearingImpaired, url: s.server + '/Videos/' + id + '/' + ms.Id + '/Subtitles/' + x.Index + '/0/Stream.vtt?api_key=' + s.token };
      });
      var audio = (ms.MediaStreams || []).filter(function (x) { return x.Type === 'Audio'; }).map(function (x) { return { index: x.Index, label: x.DisplayTitle || x.Language || 'Audio' }; });
      var vs = (ms.MediaStreams || []).filter(function (x) { return x.Type === 'Video'; })[0] || {};
      return { fps: vs.RealFrameRate || vs.AverageFrameRate || 0, url: url, hls: url.indexOf('.m3u8') >= 0, sessionId: r.PlaySessionId, mediaSourceId: ms.Id, subs: subs, audio: audio, direct: !!ms.SupportsDirectPlay };
    });
  };
  J.report = function (what, id, pb, posSec, paused) {
    var body = { ItemId: id, PlaySessionId: pb.sessionId, MediaSourceId: pb.mediaSourceId, PositionTicks: Math.round(posSec * 10000000), IsPaused: !!paused, CanSeek: true, PlayMethod: pb.direct ? 'DirectPlay' : 'Transcode' };
    return J.req('/Sessions/Playing' + (what === 'start' ? '' : what === 'stop' ? '/Stopped' : '/Progress'), { method: 'POST', body: body }).catch(function () {});
  };

  // Jellyfin sign-in: Quick Connect (a code you approve in Jellyfin on the PC) or name and password.
  M.jfServers = function () { return T.call('findJellyfin').then(function (r) { return (r || []).map(function (x) { return { name: x.Name, url: (x.Address || '').replace(/\/$/, ''), id: x.Id }; }); }); };
  M.jfQuickStart = function (server) {
    return fetch(server + '/QuickConnect/Initiate', { method: 'POST', headers: { 'X-Emby-Authorization': J.auth(), 'Authorization': J.auth() } }).then(function (r) { if (!r.ok) throw new Error(r.status === 401 ? 'Quick Connect is off on this server' : 'HTTP ' + r.status); return r.json(); });
  };
  M.jfQuickPoll = function (server, secret, forProfile) {
    return fetch(server + '/QuickConnect/Connect?Secret=' + encodeURIComponent(secret)).then(function (r) { return r.json(); }).then(function (q) {
      if (!q.Authenticated) return false;
      return fetch(server + '/Users/AuthenticateWithQuickConnect', { method: 'POST', headers: { 'Content-Type': 'application/json', 'X-Emby-Authorization': J.auth(), 'Authorization': J.auth() }, body: JSON.stringify({ Secret: secret }) })
        .then(function (r) { return r.json(); }).then(function (a) { saveSession({ app: 'jellyfin', server: server, token: a.AccessToken, userId: a.User.Id, user: a.User.Name, name: (a.ServerId && M.serverName) || 'Jellyfin' }, forProfile); M.init(); return true; });
    });
  };
  M.jfLogin = function (server, user, pw, forProfile) {
    return fetch(server + '/Users/AuthenticateByName', { method: 'POST', headers: { 'Content-Type': 'application/json', 'X-Emby-Authorization': J.auth(), 'Authorization': J.auth() }, body: JSON.stringify({ Username: user, Pw: pw }) })
      .then(function (r) { if (!r.ok) throw new Error(r.status === 401 ? 'That name or password isn\'t right' : 'HTTP ' + r.status); return r.json(); })
      .then(function (a) { saveSession({ app: 'jellyfin', server: server, token: a.AccessToken, userId: a.User.Id, user: a.User.Name, name: 'Jellyfin' }, forProfile); M.init(); return true; });
  };

  // ---------------- Plex ----------------
  var P = {};
  var PLEX_ID = function () { return deviceId(); };
  P.hdr = function (token) { var h = { 'Accept': 'application/json', 'X-Plex-Product': 'Hearth TV', 'X-Plex-Client-Identifier': PLEX_ID(), 'X-Plex-Device-Name': 'Hearth TV', 'X-Plex-Platform': 'Android', 'X-Plex-Version': T.n('version') || '1.0' }; if (token) h['X-Plex-Token'] = token; return h; };
  P.req = function (path) { var s = session(); return fetch(s.server + path + (path.indexOf('?') < 0 ? '?' : '&') + 'X-Plex-Token=' + s.token, { headers: P.hdr(s.token) }).then(function (r) { if (r.status === 401) { M.error = 'signedOut'; throw new Error('signed out'); } if (!r.ok) throw new Error('HTTP ' + r.status); return r.json(); }).then(function (j) { return j.MediaContainer || {}; }); };
  P.img = function (path, w, h) { var s = session(); return path ? s.server + '/photo/:/transcode?width=' + (w || 640) + '&height=' + (h || 360) + '&minSize=1&upscale=1&url=' + encodeURIComponent(path) + '&X-Plex-Token=' + s.token : ''; };
  P.item = function (x) {
    var kind = { show: 'show', movie: 'movie', episode: 'episode', season: 'season', album: 'album', track: 'track' }[x.type] || 'movie';
    var media = (x.Media || [])[0] || {}, mins = (x.duration || 0) / 60000;
    var tags = []; if (media.videoResolution === '4k') tags.push('4K'); else if (/^(1080|720)$/.test(media.videoResolution || '')) tags.push('HD');
    return {
      src: 'plex', id: x.ratingKey, kind: kind, title: x.title || '', year: x.year || '', overview: x.summary || '',
      img: P.img(kind === 'episode' ? x.thumb : (x.art || x.thumb)), backdrop: P.img(x.art || x.grandparentArt || x.thumb, 1920, 1080), poster: P.img(x.thumb, 400, 600),
      pct: x.viewOffset && x.duration ? x.viewOffset / x.duration * 100 : 0, played: (x.viewCount || 0) > 0 && !x.viewOffset, posTicks: (x.viewOffset || 0) * 10000, mins: mins,
      seriesId: x.grandparentRatingKey || '', seriesName: x.grandparentTitle || '', seasonId: x.parentRatingKey || '', index: x.index, parentIndex: x.parentIndex,
      rating: x.contentRating || '', genre: ((x.Genre || [])[0] || {}).tag || '', childCount: x.leafCount || x.childCount || 0, unplayed: (x.leafCount || 0) - (x.viewedLeafCount || 0),
      artist: x.parentTitle || '', tags: tags, created: (x.addedAt || 0) * 1000, key: x.key
    };
  };
  P.items = function (c) { return filt((c.Metadata || []).map(P.item)); };
  P.resume = function () { return P.req('/library/onDeck').then(P.items); };
  P.nextUp = function () { return Promise.resolve([]); };
  P.latest = function () { return P.req('/library/recentlyAdded?X-Plex-Container-Size=16').then(P.items); };
  P.library = function () {
    return P.req('/library/sections').then(function (c) {
      var secs = (c.Directory || []).filter(function (d) { return /^(movie|show|artist)$/.test(d.type); });
      return Promise.all(secs.map(function (d) { return P.req('/library/sections/' + d.key + '/all?type=' + (d.type === 'artist' ? 9 : d.type === 'show' ? 2 : 1)).then(P.items).catch(function () { return []; }); }))
        .then(function (ls) { return [].concat.apply([], ls).sort(function (a, b) { return b.created - a.created; }); });
    });
  };
  P.views = function () { return P.req('/library/sections').then(function (c) { return (c.Directory || []).map(function (d) { return { id: d.key, name: d.title, type: d.type }; }); }); };
  P.seasons = function (id) { return P.req('/library/metadata/' + id + '/children').then(function (c) { return (c.Metadata || []).map(P.item); }); };
  P.episodes = function (id, seasonId) { return P.req('/library/metadata/' + seasonId + '/children').then(P.items); };
  P.get = function (id) { return P.req('/library/metadata/' + id).then(function (c) { return P.item((c.Metadata || [])[0] || {}); }); };
  P.similar = function (id) { return P.req('/library/metadata/' + id + '/similar').then(P.items).catch(function () { return []; }); };
  P.tracks = function (id) { return P.req('/library/metadata/' + id + '/children').then(P.items); };
  P.search = function (q) { return P.req('/hubs/search?query=' + encodeURIComponent(q) + '&limit=8').then(function (c) { var out = []; (c.Hub || []).forEach(function (h) { if (/^(movie|show|album)$/.test(h.type)) out = out.concat(P.items(h)); }); return out; }); };
  P.setPlayed = function (id, on) { return P.req('/:/' + (on ? 'scrobble' : 'unscrobble') + '?identifier=com.plexapp.plugins.library&key=' + id); };
  P.audioUrl = function (id) { var s = session(); return s.server + '/music/:/transcode/universal/start.mp3?path=' + encodeURIComponent('/library/metadata/' + id) + '&X-Plex-Token=' + s.token + '&X-Plex-Client-Identifier=' + PLEX_ID(); };
  P.playback = function (id, startTicks) {
    var s = session(), q = { original: '', '1080p': '&maxVideoBitrate=20000&videoResolution=1920x1080', '720p': '&maxVideoBitrate=4000&videoResolution=1280x720' }[T.settings.media.quality] || '';
    var sid = 'hearthtv' + Date.now();
    var url = s.server + '/video/:/transcode/universal/start.m3u8?path=' + encodeURIComponent('/library/metadata/' + id) + '&mediaIndex=0&partIndex=0&protocol=hls&fastSeek=1&directPlay=0&directStream=1&subtitles=burn&copyts=1&offset=' + Math.round((startTicks || 0) / 10000000) + q + '&session=' + sid + '&X-Plex-Session-Identifier=' + sid + '&X-Plex-Token=' + s.token + '&X-Plex-Client-Identifier=' + PLEX_ID() + '&X-Plex-Product=Hearth%20TV&X-Plex-Platform=Chrome';
    return Promise.resolve({ url: url, hls: true, sessionId: sid, subs: [], audio: [], direct: false, plexKey: '/library/metadata/' + id });
  };
  P.report = function (what, id, pb, posSec, paused, durSec) {
    return P.req('/:/timeline?ratingKey=' + id + '&key=' + encodeURIComponent(pb.plexKey) + '&state=' + (what === 'stop' ? 'stopped' : paused ? 'paused' : 'playing') + '&time=' + Math.round(posSec * 1000) + '&duration=' + Math.round((durSec || 0) * 1000)).catch(function () {});
  };
  // Plex sign-in: a 4-character code typed at plex.tv/link.
  M.plexStart = function () { return fetch('https://plex.tv/api/v2/pins?strong=false', { method: 'POST', headers: P.hdr() }).then(function (r) { return r.json(); }); };
  M.plexPoll = function (pin, forProfile) {
    return fetch('https://plex.tv/api/v2/pins/' + pin.id, { headers: P.hdr() }).then(function (r) { return r.json(); }).then(function (p) {
      if (!p.authToken) return false;
      return fetch('https://plex.tv/api/v2/resources?includeHttps=1&includeRelay=0', { headers: P.hdr(p.authToken) }).then(function (r) { return r.json(); }).then(function (res) {
        var servers = (res || []).filter(function (x) { return (x.provides || '').indexOf('server') >= 0; });
        var srv = servers[0]; if (!srv) throw new Error('No Plex server on this account');
        var conns = (srv.connections || []).sort(function (a, b) { return (b.local ? 1 : 0) - (a.local ? 1 : 0); });
        var c = conns[0];
        saveSession({ app: 'plex', server: c.uri, token: srv.accessToken || p.authToken, user: '', name: srv.name }, forProfile); M.init(); return true;
      });
    });
  };

  // ---------------- The one My Media the screens use ----------------
  function api() { var s = session(); return s && s.app === 'plex' ? P : J; }
  ['resume', 'nextUp', 'latest', 'library', 'views', 'seasons', 'episodes', 'get', 'similar', 'tracks', 'search', 'setPlayed', 'audioUrl', 'playback', 'report'].forEach(function (k) {
    M[k] = function () { if (!session()) return Promise.reject(new Error('My Media isn\'t set up')); var r = api()[k].apply(null, arguments); return r; };
  });
  M.audioUrl = function (id) { return session() ? api().audioUrl(id) : ''; };
  M.init = function () {
    var s = session();
    M.ready = !!s; M.kind = s ? s.app : null; M.name = s ? (s.name || (s.app === 'plex' ? 'Plex' : 'Jellyfin')) : ''; M.error = '';
    T.data.mediaName = M.ready ? 'Nvidia PC' : '';
    M.cache = {};
  };
  /** Keep watching (resume + next episodes), newest first. Cached for a minute. */
  M.keepWatching = function () {
    if (!M.ready) return Promise.resolve([]);
    return Promise.all([M.resume().catch(function () { return []; }), M.nextUp().catch(function () { return []; })]).then(function (r) {
      var seen = {}, out = [];
      r[0].concat(r[1]).forEach(function (x) { var k = x.seriesId || x.id; if (!seen[k]) { seen[k] = 1; out.push(x); } });
      return out;
    });
  };

  // ---------------- Wake the PC first (Settings › My Media › Wake the PC when needed) ----------------
  M.wake = function () {
    if (!T.settings.media.wake) return;
    var mac = T.state.pcMac; if (mac) T.n('wake', mac);
  };
})();
