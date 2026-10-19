#include "browser.h"

#include <cmath>
#include <cstdlib>
#include <fstream>
#include <sstream>
#include <sys/stat.h>
#include <unistd.h>

#include "include/base/cef_callback.h"
#include "include/cef_cookie.h"
#include "include/cef_parser.h"
#include "include/cef_scheme.h"
#include "include/cef_stream.h"
#include "include/views/cef_fill_layout.h"
#include "include/wrapper/cef_closure_task.h"
#include "include/wrapper/cef_helpers.h"
#include "include/wrapper/cef_stream_resource_handler.h"

namespace hearth {

const int kChromeHeight = 86;

Globals& G() { static Globals g; return g; }

CefRefPtr<CefValue> Json(const std::string& s) { return CefParseJSON(s, JSON_PARSER_ALLOW_TRAILING_COMMAS); }
CefRefPtr<CefValue> V(CefRefPtr<CefDictionaryValue> d) { auto v = CefValue::Create(); v->SetDictionary(d); return v; }
CefRefPtr<CefValue> V(CefRefPtr<CefListValue> l) { auto v = CefValue::Create(); v->SetList(l); return v; }

namespace {

int next_tab_id = 1;
bool StartsWith(const std::string& s, const std::string& p) { return s.compare(0, p.size(), p) == 0; }
std::string Home() { const char* h = getenv("HOME"); return h ? h : "/tmp"; }
std::string Str(CefRefPtr<CefDictionaryValue> d, const char* k) { return d && d->HasKey(k) && d->GetType(k) == VTYPE_STRING ? d->GetString(k).ToString() : ""; }
int Int(CefRefPtr<CefDictionaryValue> d, const char* k) {
  if (!d || !d->HasKey(k)) return 0;
  if (d->GetType(k) == VTYPE_INT) return d->GetInt(k);
  if (d->GetType(k) == VTYPE_DOUBLE) return (int)d->GetDouble(k);
  return 0;
}
double Num(CefRefPtr<CefDictionaryValue> d, const char* k) {
  if (!d || !d->HasKey(k)) return 0;
  if (d->GetType(k) == VTYPE_INT) return d->GetInt(k);
  if (d->GetType(k) == VTYPE_DOUBLE) return d->GetDouble(k);
  return 0;
}
BrowserWin* WindowOf(CefRefPtr<CefBrowser> b) {
  for (auto* w : G().windows) if (w->Owns(b)) return w;
  return nullptr;
}
std::string Downloads() {
  const char* x = getenv("XDG_DOWNLOAD_DIR");
  return x ? x : Home() + "/Downloads";
}
std::string HostOf(const std::string& url) {
  size_t a = url.find("://");
  if (a == std::string::npos) return "";
  a += 3;
  size_t b = url.find_first_of("/:?#", a);
  std::string h = url.substr(a, b == std::string::npos ? std::string::npos : b - a);
  size_t at = h.rfind('@');
  if (at != std::string::npos) h = h.substr(at + 1);
  for (auto& c : h) c = (char)tolower((unsigned char)c);
  return h;
}
bool Blocked(const std::string& host) {
  if (host.empty()) return false;
  std::string h = host;
  while (true) {
    if (G().adhosts.count(h)) return true;
    size_t dot = h.find('.');
    if (dot == std::string::npos) return false;
    h = h.substr(dot + 1);
    if (h.find('.') == std::string::npos) return false;   // never a bare TLD
  }
}

// ---------------------------------------------------------------- hearth:// pages

// window.hb (the top bar) and window.hearth (Hearth's own pages), on top of CEF's window.cefQuery.
const char* kBridge = R"JS(<script>
(() => { if (window.hb) return;
  const q = o => new Promise((ok, no) => window.cefQuery({ request: JSON.stringify(o), persistent: false,
    onSuccess: r => { try { ok(JSON.parse(r)); } catch (e) { ok(null); } }, onFailure: (c, m) => no(new Error(m)) }));
  const subs = [];
  window.__hbEvent = m => subs.forEach(f => { try { f(m); } catch (e) {} });
  window.hb = { act: m => { q({ t: 'act', m }).catch(() => {}); }, suggest: text => q({ t: 'suggest', text: String(text || '') }), on: fn => subs.push(fn) };
  window.hearth = { get: m => q({ t: 'page', m }) };
})();
</script>)JS";

std::map<std::string, std::string>& FileCache() { static std::map<std::string, std::string> c; return c; }

const std::string* PageFile(const std::string& name) {
  auto& c = FileCache();
  auto it = c.find(name);
  if (it != c.end()) return &it->second;
  std::ifstream f(G().res_dir + "/" + name, std::ios::binary);
  if (!f) return nullptr;
  std::stringstream ss; ss << f.rdbuf();
  std::string data = ss.str();
  if (name.size() > 5 && name.substr(name.size() - 5) == ".html") {
    size_t head = data.find("<head>");
    if (head != std::string::npos) data.insert(head + 6, kBridge);
  }
  return &(c[name] = data);
}

class HearthSchemeFactory : public CefSchemeHandlerFactory {
 public:
  CefRefPtr<CefResourceHandler> Create(CefRefPtr<CefBrowser> browser, CefRefPtr<CefFrame> frame, const CefString& scheme_name,
                                       CefRefPtr<CefRequest> request) override {
    std::string url = request->GetURL();
    std::string path = url.substr(std::min(url.size(), url.find("://") + 3));
    size_t q = path.find_first_of("?#");
    if (q != std::string::npos) path = path.substr(0, q);
    std::string base = path.substr(path.rfind('/') == std::string::npos ? 0 : path.rfind('/') + 1);
    std::string name = "pages.html", mime = "text/html";
    auto ends = [&](const char* e) { std::string s(e); return base.size() > s.size() && base.compare(base.size() - s.size(), s.size(), s) == 0; };
    if (base == "chrome.html") name = "chrome.html";
    else if (ends(".svg")) { name = base; mime = "image/svg+xml"; }
    else if (ends(".png")) { name = base; mime = "image/png"; }
    else if (ends(".js")) { name = base; mime = "text/javascript"; }
    else if (ends(".css")) { name = base; mime = "text/css"; }
    if (name.find('/') != std::string::npos || name.find("..") != std::string::npos) return nullptr;
    const std::string* data = PageFile(name);
    if (!data) return nullptr;
    auto stream = CefStreamReader::CreateForData(const_cast<char*>(data->data()), data->size());
    return new CefStreamResourceHandler(mime, stream);
  }
  IMPLEMENT_REFCOUNTING(HearthSchemeFactory);
};

// ---------------------------------------------------------------- ad blocking

class AdBlock : public CefResourceRequestHandler {
 public:
  cef_return_value_t OnBeforeResourceLoad(CefRefPtr<CefBrowser> browser, CefRefPtr<CefFrame> frame, CefRefPtr<CefRequest> request,
                                          CefRefPtr<CefCallback> callback) override {
    if (!G().adblock) return RV_CONTINUE;
    // A page you open yourself always loads; what it pulls in from ad and tracker servers doesn't.
    if (request->GetResourceType() == RT_MAIN_FRAME) return RV_CONTINUE;
    if (Blocked(HostOf(request->GetURL()))) { G().blocked++; return RV_CANCEL; }
    return RV_CONTINUE;
  }
  IMPLEMENT_REFCOUNTING(AdBlock);
};
CefRefPtr<AdBlock> TheAdBlock() { static CefRefPtr<AdBlock> a = new AdBlock(); return a; }

// ---------------------------------------------------------------- the message router (top bar and pages)

class Router : public CefMessageRouterBrowserSide::Handler {
 public:
  bool OnQuery(CefRefPtr<CefBrowser> browser, CefRefPtr<CefFrame> frame, int64_t query_id, const CefString& request,
               bool persistent, CefRefPtr<Callback> callback) override {
    std::string from = frame->GetURL();
    if (!StartsWith(from, "hearth://")) { callback->Failure(403, "not allowed"); return true; }
    auto v = Json(request);
    if (!v || v->GetType() != VTYPE_DICTIONARY) { callback->Failure(400, "bad request"); return true; }
    auto d = v->GetDictionary();
    std::string t = Str(d, "t");
    BrowserWin* w = WindowOf(browser);
    auto reply = [&](CefRefPtr<CefValue> r) { callback->Success(r ? CefWriteJSON(r, JSON_WRITER_DEFAULT) : CefString("null")); };
    if (t == "act") {
      if (w && d->GetType("m") == VTYPE_DICTIONARY) w->Act(d->GetDictionary("m"));
      reply(nullptr);
    } else if (t == "suggest") {
      reply(V(G().store->Suggest(Str(d, "text"))));
    } else if (t == "page") {
      reply(Page(w, d->GetType("m") == VTYPE_DICTIONARY ? d->GetDictionary("m") : CefDictionaryValue::Create()));
    } else callback->Failure(404, "unknown");
    return true;
  }

 private:
  CefRefPtr<CefValue> Page(BrowserWin* w, CefRefPtr<CefDictionaryValue> m) {
    auto& s = *G().store;
    std::string get = Str(m, "get");
    if (get == "start") {
      auto out = CefDictionaryValue::Create();
      auto top = s.TopSites(8);
      if (top->GetSize() < 4) {
        top = CefListValue::Create();
        const char* tiles[][2] = {{"YouTube", "https://www.youtube.com/"}, {"Twitch", "https://www.twitch.tv/"}, {"Steam Store", "https://store.steampowered.com/"},
                                  {"ProtonDB", "https://www.protondb.com/"}, {"Reddit", "https://www.reddit.com/"}, {"Hearth OS", "https://github.com/Dragongamer9/hearth-os"}};
        for (auto& x : tiles) { auto e = CefDictionaryValue::Create(); e->SetString("title", x[0]); e->SetString("url", x[1]); top->SetDictionary(top->GetSize(), e); }
      }
      out->SetList("tiles", top);
      auto bm = CefListValue::Create();
      for (size_t i = 0; i < s.bookmarks()->GetSize() && i < 12; i++) bm->SetDictionary(i, s.bookmarks()->GetDictionary(i)->Copy(false));
      out->SetList("bookmarks", bm);
      std::string engine = s.Setting("search", "duckduckgo"), name = "DuckDuckGo";
      for (auto& e : Engines()) if (e.id == engine) name = e.name;
      out->SetString("search", name);
      return V(out);
    }
    if (get == "history") return V(s.History(Str(m, "q")));
    if (get == "bookmarks") return V(s.bookmarks()->Copy());
    if (get == "downloads") return DownloadList();
    if (get == "settings") {
      auto out = CefDictionaryValue::Create();
      out->SetDictionary("settings", s.settings()->Copy(false));
      auto engines = CefListValue::Create();
      for (auto& e : Engines()) { auto x = CefDictionaryValue::Create(); x->SetString("id", e.id); x->SetString("name", e.name); engines->SetDictionary(engines->GetSize(), x); }
      out->SetList("engines", engines);
      out->SetString("version", G().version + " (Chromium " + std::to_string(CHROME_VERSION_MAJOR) + ")");
      out->SetInt("blocked", G().blocked);
      return V(out);
    }
    if (get == "set") {
      std::string key = Str(m, "key");
      if (m->HasKey("value")) s.Set(key, m->GetValue("value"));
      if (key == "adblock") G().adblock = s.SettingOn("adblock", true);
      s.Flush();
      return V(s.settings()->Copy(false));
    }
    if (get == "clearHistory") { s.ClearHistory(); s.Flush(); auto v = CefValue::Create(); v->SetBool(true); return v; }
    if (get == "clearData") {
      CefCookieManager::GetGlobalManager(nullptr)->DeleteCookies("", "", nullptr);
      if (w && w->Active()) {
        auto host = w->Active()->view->GetBrowser()->GetHost();
        host->ExecuteDevToolsMethod(0, "Network.clearBrowserCache", nullptr);
      }
      auto v = CefValue::Create(); v->SetBool(true); return v;
    }
    if (get == "removeBookmark") {
      s.RemoveBookmark(Str(m, "url")); s.Flush();
      for (auto* x : G().windows) x->PushAll();
      return V(s.bookmarks()->Copy());
    }
    if (get == "search") { if (w) w->Navigate(Str(m, "text")); auto v = CefValue::Create(); v->SetBool(true); return v; }
    if (get == "makeDefault") {
      int r = system("xdg-settings set default-web-browser hearth-browser.desktop >/dev/null 2>&1");
      auto v = CefValue::Create(); v->SetBool(r == 0); return v;
    }
    return nullptr;
  }

 public:
  static CefRefPtr<CefValue> DownloadList() {
    auto l = CefListValue::Create();
    for (auto& d : G().downloads) {
      auto x = CefDictionaryValue::Create();
      x->SetDouble("id", d.id); x->SetString("name", d.name); x->SetString("file", d.file); x->SetString("url", d.url);
      x->SetDouble("total", (double)d.total); x->SetDouble("got", (double)d.got); x->SetString("state", d.state);
      l->SetDictionary(l->GetSize(), x);
    }
    return V(l);
  }
};

}  // namespace

void RegisterHearthScheme() { CefRegisterSchemeHandlerFactory("hearth", "", new HearthSchemeFactory()); }

void LoadAdblockList() {
  // A hosts-style list of ad and tracker servers ("0.0.0.0 host"), made when Hearth OS is built.
  std::ifstream f(G().res_dir + "/adblock-hosts.txt");
  std::string line;
  while (std::getline(f, line)) {
    if (line.empty() || line[0] == '#') continue;
    std::istringstream ss(line);
    std::string a, b;
    ss >> a >> b;
    std::string host = b.empty() ? a : b;
    if (host.empty() || host == "localhost" || host.find('.') == std::string::npos) continue;
    G().adhosts.insert(host);
  }
}

void PushDownloads(bool force) {
  static double last = 0;
  double now = (double)time(nullptr) * 1000;
  if (!force && now - last < 400) return;
  last = now;
  auto list = Router::DownloadList();
  for (auto* w : G().windows) w->ToChrome("downloads", list);
}

void InstallRouter() {
  CefMessageRouterConfig config;
  G().router = CefMessageRouterBrowserSide::Create(config);
  static Router handler;
  G().router->AddHandler(&handler, true);
}

void OpenWindow(const std::vector<std::string>& urls) {
  CefRefPtr<BrowserWin> w = new BrowserWin(urls);
  CefWindow::CreateTopLevelWindow(w);
}

// ---------------------------------------------------------------- a window

BrowserWin::BrowserWin(const std::vector<std::string>& urls) : start_urls_(urls) {
  client_ = new Client(this);
  G().windows.insert(this);
}

void BrowserWin::OnWindowCreated(CefRefPtr<CefWindow> window) {
  window_ = window;
  window->SetTitle("Hearth Browser");
  window->SetBackgroundColor(0xFF1C1512);
  CefBoxLayoutSettings box;
  box.horizontal = false;
  layout_ = window->SetToBoxLayout(box);
  CefBrowserSettings chrome_settings;
  chrome_settings.background_color = 0xFF1C1512;
  chrome_ = CefBrowserView::CreateBrowserView(client_, "hearth://app/chrome.html", chrome_settings, nullptr, nullptr, client_);
  window->AddChildView(chrome_);
  content_ = CefPanel::CreatePanel(nullptr);
  content_->SetToFillLayout();
  window->AddChildView(content_);
  layout_->SetFlexForView(chrome_, 0);
  layout_->SetFlexForView(content_, 1);
  window->CenterWindow(CefSize(1280, 820));
  window->Show();
  if (start_urls_.empty()) NewTab("", true);
  else for (size_t i = 0; i < start_urls_.size(); i++) NewTab(start_urls_[i], i + 1 == start_urls_.size());
}

void BrowserWin::OnWindowDestroyed(CefRefPtr<CefWindow> window) {
  client_->Detach();
  G().windows.erase(this);
  tabs_.clear();
  chrome_ = nullptr; content_ = nullptr; layout_ = nullptr; window_ = nullptr;
  G().store->Flush();
}

bool BrowserWin::CanClose(CefRefPtr<CefWindow> window) {
  if (G().windows.size() == 1) G().store->SaveSession(Urls());
  G().store->Flush();
  bool all = true;
  for (auto& t : tabs_) if (auto b = t->view->GetBrowser()) all = b->GetHost()->TryCloseBrowser() && all;
  if (chrome_) if (auto b = chrome_->GetBrowser()) all = b->GetHost()->TryCloseBrowser() && all;
  return all;
}

std::vector<std::string> BrowserWin::Urls() {
  std::vector<std::string> v;
  for (auto& t : tabs_) if (!t->url.empty()) v.push_back(t->url);
  return v;
}

Tab* BrowserWin::Active() {
  for (auto& t : tabs_) if (t->id == active_) return t.get();
  return nullptr;
}
Tab* BrowserWin::ByBrowser(CefRefPtr<CefBrowser> b) {
  if (!b) return nullptr;
  for (auto& t : tabs_) { auto x = t->view ? t->view->GetBrowser() : nullptr; if (x && x->IsSame(b)) return t.get(); }
  return nullptr;
}
bool BrowserWin::IsChrome(CefRefPtr<CefBrowser> b) {
  auto x = chrome_ ? chrome_->GetBrowser() : nullptr;
  return x && b && x->IsSame(b);
}

Tab* BrowserWin::NewTab(const std::string& url, bool focus, int after) {
  auto t = std::make_unique<Tab>();
  t->id = next_tab_id++;
  std::string start = url.empty() ? G().store->Setting("home", "hearth://start") : url;
  t->url = start;
  CefBrowserSettings settings;
  settings.background_color = 0xFFFFFFFF;
  t->view = CefBrowserView::CreateBrowserView(client_, start, settings, nullptr, nullptr, client_);
  t->view->SetVisible(false);
  content_->AddChildView(t->view);
  Tab* raw = t.get();
  size_t at = tabs_.size();
  if (after) for (size_t i = 0; i < tabs_.size(); i++) if (tabs_[i]->id == after) at = i + 1;
  tabs_.insert(tabs_.begin() + (long)at, std::move(t));
  if (focus) Select(raw->id); else PushAll();
  return raw;
}

void BrowserWin::Select(int id) {
  Tab* t = nullptr;
  for (auto& x : tabs_) if (x->id == id) t = x.get();
  if (!t) return;
  for (auto& x : tabs_) x->view->SetVisible(x->id == id);
  active_ = id;
  if (content_) content_->Layout();
  PushAll();
  UpdateTitle();
  if (t->url.empty() || StartsWith(t->url, "hearth://start")) ToChrome("focusAddress", CefValue::Create());
  else t->view->RequestFocus();
}

void BrowserWin::Close(int id) {
  for (size_t i = 0; i < tabs_.size(); i++) {
    if (tabs_[i]->id != id) continue;
    auto view = tabs_[i]->view;
    if (auto b = view->GetBrowser()) b->GetHost()->CloseBrowser(true);
    content_->RemoveChildView(view);
    tabs_.erase(tabs_.begin() + (long)i);
    if (tabs_.empty()) { window_->Close(); return; }
    if (active_ == id) Select(tabs_[std::min(i, tabs_.size() - 1)]->id); else PushAll();
    return;
  }
}

void BrowserWin::Navigate(const std::string& text) {
  Tab* t = Active(); if (!t) return;
  std::string u = ToUrl(text, G().store->Setting("search", "duckduckgo"));
  if (u.empty()) return;
  if (auto b = t->view->GetBrowser()) { b->GetMainFrame()->LoadURL(u); t->view->RequestFocus(); }
}

void BrowserWin::UpdateTitle() {
  Tab* t = Active();
  if (window_) window_->SetTitle((t && !t->title.empty() ? t->title + " – " : std::string()) + "Hearth Browser");
}

void BrowserWin::Fullscreen(bool on) {
  fullscreen_ = on;
  if (!window_) return;
  chrome_->SetVisible(!on);
  window_->SetFullscreen(on);
  window_->Layout();
}

void BrowserWin::Expand(bool on) {
  // A menu of the top bar is open: it gets the whole window until it closes.
  if (expanded_ == on || !window_) return;
  expanded_ = on;
  content_->SetVisible(!on);
  layout_->SetFlexForView(chrome_, on ? 1 : 0);
  window_->Layout();
}

void BrowserWin::ToChrome(const std::string& name, CefRefPtr<CefValue> data) {
  if (!chrome_) return;
  auto b = chrome_->GetBrowser();
  if (!b) return;
  auto msg = CefDictionaryValue::Create();
  msg->SetString("name", name);
  msg->SetValue("data", data ? data : CefValue::Create());
  std::string js = "window.__hbEvent && window.__hbEvent(" + CefWriteJSON(V(msg), JSON_WRITER_DEFAULT).ToString() + ")";
  b->GetMainFrame()->ExecuteJavaScript(js, "hearth://app/chrome.html", 0);
}

CefRefPtr<CefDictionaryValue> BrowserWin::State(Tab* t) {
  auto d = CefDictionaryValue::Create();
  auto b = t->view ? t->view->GetBrowser() : nullptr;
  d->SetInt("id", t->id);
  std::string disp = DisplayUrl(t->url);
  d->SetString("title", !t->title.empty() ? t->title : !disp.empty() ? disp : "New tab");
  d->SetString("url", t->url);
  d->SetString("display", disp);
  if (t->favicon.empty()) d->SetNull("favicon"); else d->SetString("favicon", t->favicon);
  d->SetBool("loading", t->loading);
  d->SetBool("back", b ? b->CanGoBack() : false);
  d->SetBool("forward", b ? b->CanGoForward() : false);
  d->SetBool("bookmarked", G().store->IsBookmarked(t->url));
  d->SetBool("secure", StartsWith(t->url, "https:") || StartsWith(t->url, "hearth:"));
  d->SetInt("zoom", b ? (int)std::lround(std::pow(1.2, b->GetHost()->GetZoomLevel()) * 100) : 100);
  d->SetBool("audible", false);
  d->SetBool("muted", b ? b->GetHost()->IsAudioMuted() : false);
  return d;
}

void BrowserWin::PushAll() {
  if (!chrome_ready_) return;
  auto d = CefDictionaryValue::Create();
  auto list = CefListValue::Create();
  for (auto& t : tabs_) list->SetDictionary(list->GetSize(), State(t.get()));
  d->SetList("tabs", list);
  d->SetInt("active", active_);
  auto bm = CefListValue::Create();
  auto all = G().store->bookmarks();
  for (size_t i = 0; i < all->GetSize() && i < 30; i++) bm->SetDictionary(i, all->GetDictionary(i)->Copy(false));
  d->SetList("bookmarks", bm);
  d->SetValue("downloads", Router::DownloadList());
  ToChrome("tabs", V(d));
}

void BrowserWin::PushTab(Tab* t) {
  if (chrome_ready_) ToChrome("tab", V(State(t)));
  if (t->id == active_) UpdateTitle();
}

void BrowserWin::Act(CefRefPtr<CefDictionaryValue> m) {
  std::string act = Str(m, "act");
  Tab* t = Active();
  auto b = t && t->view ? t->view->GetBrowser() : nullptr;
  if (act == "ready") { chrome_ready_ = true; PushAll(); return; }
  if (act == "new") { NewTab(Str(m, "url"), true); return; }
  if (act == "select") { Select(Int(m, "id")); return; }
  if (act == "close") { Close(Int(m, "id")); return; }
  if (act == "move") {
    int id = Int(m, "id"), to = Int(m, "to");
    for (size_t i = 0; i < tabs_.size(); i++) if (tabs_[i]->id == id) {
      auto x = std::move(tabs_[i]); tabs_.erase(tabs_.begin() + (long)i);
      size_t j = (size_t)std::max(0, std::min((int)tabs_.size(), to));
      tabs_.insert(tabs_.begin() + (long)j, std::move(x)); PushAll(); return;
    }
    return;
  }
  if (act == "go") { Navigate(Str(m, "text")); return; }
  if (act == "newWindow") { OpenWindow({}); return; }
  if (act == "quit") { for (auto* w : std::set<BrowserWin*>(G().windows)) if (w->window()) w->window()->Close(); return; }
  if (act == "expand") { Expand(m->HasKey("on") && m->GetType("on") == VTYPE_BOOL && m->GetBool("on")); return; }
  if (act == "internal") { NewTab("hearth://" + Str(m, "page"), true, t ? t->id : 0); return; }
  if (act == "downloadOpen" || act == "downloadShow" || act == "downloadCancel") {
    uint32_t id = (uint32_t)Num(m, "id");
    for (auto& d : G().downloads) if (d.id == id) {
      if (act == "downloadCancel") { if (d.cancel) d.cancel->Cancel(); }
      else {
        std::string target = act == "downloadOpen" ? d.file : d.file.substr(0, d.file.rfind('/'));
        std::string cmd = "xdg-open '";
        for (char c : target) { if (c == '\'') cmd += "'\\''"; else cmd += c; }
        cmd += "' >/dev/null 2>&1 &";
        int r = system(cmd.c_str()); (void)r;
      }
    }
    return;
  }
  if (!b) return;
  auto host = b->GetHost();
  if (act == "open") { b->GetMainFrame()->LoadURL(Str(m, "url")); t->view->RequestFocus(); }
  else if (act == "back") b->GoBack();
  else if (act == "forward") b->GoForward();
  else if (act == "reload") { if (t->loading) b->StopLoad(); else b->Reload(); }
  else if (act == "home") b->GetMainFrame()->LoadURL(G().store->Setting("home", "hearth://start"));
  else if (act == "bookmark") { G().store->ToggleBookmark(t->url, t->title); G().store->Flush(); PushAll(); }
  else if (act == "mute") { host->SetAudioMuted(!host->IsAudioMuted()); PushTab(t); }
  else if (act == "zoom") {
    int v = Int(m, "v");
    double f = v == 0 ? 1.0 : std::max(0.3, std::min(3.0, std::pow(1.2, host->GetZoomLevel()) + v / 10.0));
    host->SetZoomLevel(std::log(f) / std::log(1.2));
    PushTab(t);
  } else if (act == "find") {
    std::string text = Str(m, "text");
    if (text.empty()) host->StopFinding(true);
    else host->Find(text, !(m->HasKey("forward") && m->GetType("forward") == VTYPE_BOOL && !m->GetBool("forward")), false,
                    m->HasKey("next") && m->GetType("next") == VTYPE_BOOL && m->GetBool("next"));
  } else if (act == "focusPage") t->view->RequestFocus();
  else if (act == "print") host->Print();
}

bool BrowserWin::Shortcut(const CefKeyEvent& e) {
  if (e.type != KEYEVENT_RAWKEYDOWN) return false;
  bool c = (e.modifiers & EVENTFLAG_CONTROL_DOWN) != 0, alt = (e.modifiers & EVENTFLAG_ALT_DOWN) != 0, shift = (e.modifiers & EVENTFLAG_SHIFT_DOWN) != 0;
  int k = e.windows_key_code;
  Tab* t = Active();
  auto b = t && t->view ? t->view->GetBrowser() : nullptr;
  auto act = [&](const char* a) { auto m = CefDictionaryValue::Create(); m->SetString("act", a); Act(m); };
  if (c && !shift && k == 'T') { NewTab("", true); return true; }
  if (c && k == 'W') { if (t) Close(t->id); return true; }
  if ((c && k == 'L') || (alt && k == 'D') || k == 0x75 /* F6 */) { ToChrome("focusAddress", CefValue::Create()); return true; }
  if ((c && k == 'R') || k == 0x74 /* F5 */) { if (b) { if (shift) b->ReloadIgnoreCache(); else b->Reload(); } return true; }
  if (alt && k == 0x25) { if (b) b->GoBack(); return true; }
  if (alt && k == 0x27) { if (b) b->GoForward(); return true; }
  if (c && k == 0x09 && !tabs_.empty() && t) {
    size_t i = 0; for (; i < tabs_.size(); i++) if (tabs_[i].get() == t) break;
    size_t n = tabs_.size();
    Select(tabs_[(i + (shift ? n - 1 : 1)) % n]->id); return true;
  }
  if (c && k >= '1' && k <= '9') { size_t i = k == '9' ? tabs_.size() - 1 : (size_t)(k - '1'); if (i < tabs_.size()) Select(tabs_[i]->id); return true; }
  if (c && k == 'F') { ToChrome("find", CefValue::Create()); return true; }
  if (c && k == 'D') { act("bookmark"); return true; }
  if (c && k == 'H') { NewTab("hearth://history", true, t ? t->id : 0); return true; }
  if (c && k == 'J') { ToChrome("openDownloads", CefValue::Create()); return true; }
  if (c && (k == 0xBB || k == 0x6B)) { auto m = CefDictionaryValue::Create(); m->SetString("act", "zoom"); m->SetInt("v", 1); Act(m); return true; }
  if (c && (k == 0xBD || k == 0x6D)) { auto m = CefDictionaryValue::Create(); m->SetString("act", "zoom"); m->SetInt("v", -1); Act(m); return true; }
  if (c && k == '0') { auto m = CefDictionaryValue::Create(); m->SetString("act", "zoom"); m->SetInt("v", 0); Act(m); return true; }
  if (k == 0x7A /* F11 */) { Fullscreen(!fullscreen_); return true; }
  if (c && k == 'N') { OpenWindow({}); return true; }
  return false;
}

// ---------------------------------------------------------------- the client

bool Client::OnProcessMessageReceived(CefRefPtr<CefBrowser> browser, CefRefPtr<CefFrame> frame, CefProcessId source,
                                      CefRefPtr<CefProcessMessage> message) {
  return G().router->OnProcessMessageReceived(browser, frame, source, message);
}

void Client::OnAfterCreated(CefRefPtr<CefBrowser> browser) { G().browsers++; }

void Client::OnBeforeClose(CefRefPtr<CefBrowser> browser) {
  G().router->OnBeforeClose(browser);
  if (--G().browsers <= 0 && G().windows.empty()) CefQuitMessageLoop();
}

bool Client::OnBeforePopup(CefRefPtr<CefBrowser> browser, CefRefPtr<CefFrame> frame,
#if CEF_VERSION_MAJOR >= 132
                           int popup_id,
#endif
                           const CefString& target_url, const CefString& target_frame_name,
                           CefLifeSpanHandler::WindowOpenDisposition target_disposition, bool user_gesture,
                           const CefPopupFeatures& popupFeatures, CefWindowInfo& windowInfo, CefRefPtr<CefClient>& client,
                           CefBrowserSettings& settings, CefRefPtr<CefDictionaryValue>& extra_info, bool* no_javascript_access) {
  // A link or window.open: a new tab next to this one. Pop-ups nobody asked for (no click) are blocked.
  if (!win_ || target_url.empty()) return true;
  if (!user_gesture && G().store->SettingOn("blockPopups", true)) return true;
  std::string url = target_url;
  bool focus = target_disposition != CEF_WOD_NEW_BACKGROUND_TAB;
  Tab* from = win_->ByBrowser(browser);
  int after = from ? from->id : 0;
  CefRefPtr<BrowserWin> w = win_;
  CefPostTask(TID_UI, base::BindOnce([](CefRefPtr<BrowserWin> w, std::string url, bool focus, int after) {
    if (G().windows.count(w.get())) w->NewTab(url, focus, after);
  }, w, url, focus, after));
  return true;
}

void Client::OnTitleChange(CefRefPtr<CefBrowser> browser, const CefString& title) {
  if (!win_) return;
  if (Tab* t = win_->ByBrowser(browser)) { t->title = title; G().store->Retitle(t->url, t->title); win_->PushTab(t); }
}
void Client::OnAddressChange(CefRefPtr<CefBrowser> browser, CefRefPtr<CefFrame> frame, const CefString& url) {
  if (!win_ || !frame->IsMain()) return;
  if (Tab* t = win_->ByBrowser(browser)) {
    t->url = url;
    if (!StartsWith(t->url, "hearth:")) G().store->Visit(t->url, t->title); else t->favicon.clear();
    win_->PushTab(t);
  }
}
void Client::OnFaviconURLChange(CefRefPtr<CefBrowser> browser, const std::vector<CefString>& icon_urls) {
  if (!win_) return;
  if (Tab* t = win_->ByBrowser(browser)) { t->favicon = icon_urls.empty() ? "" : icon_urls[0].ToString(); win_->PushTab(t); }
}
void Client::OnFullscreenModeChange(CefRefPtr<CefBrowser> browser, bool fullscreen) {
  if (win_ && win_->ByBrowser(browser)) win_->Fullscreen(fullscreen);
}

void Client::OnLoadingStateChange(CefRefPtr<CefBrowser> browser, bool isLoading, bool canGoBack, bool canGoForward) {
  if (!win_) return;
  if (Tab* t = win_->ByBrowser(browser)) { t->loading = isLoading; t->back = canGoBack; t->forward = canGoForward; win_->PushTab(t); }
}
void Client::OnLoadEnd(CefRefPtr<CefBrowser> browser, CefRefPtr<CefFrame> frame, int httpStatusCode) {
  // The top bar is up: tell it everything (it also says "ready" itself, in case this came first).
  if (win_ && frame->IsMain() && win_->IsChrome(browser)) { auto m = CefDictionaryValue::Create(); m->SetString("act", "ready"); win_->Act(m); }
}
void Client::OnLoadError(CefRefPtr<CefBrowser> browser, CefRefPtr<CefFrame> frame, ErrorCode errorCode,
                         const CefString& errorText, const CefString& failedUrl) {
  if (!win_ || !frame->IsMain() || errorCode == ERR_ABORTED) return;
  if (Tab* t = win_->ByBrowser(browser)) {
    auto d = CefDictionaryValue::Create();
    std::string u = failedUrl, shown = DisplayUrl(u);
    d->SetInt("id", t->id);
    d->SetString("text", "Couldn't open " + (shown.empty() ? u : shown) + ": " + errorText.ToString());
    win_->ToChrome("error", V(d));
  }
}

bool Client::OnBeforeBrowse(CefRefPtr<CefBrowser> browser, CefRefPtr<CefFrame> frame, CefRefPtr<CefRequest> request,
                            bool user_gesture, bool is_redirect) {
  G().router->OnBeforeBrowse(browser, frame);
  // The top bar only ever shows itself.
  if (win_ && win_->IsChrome(browser) && frame->IsMain() && !StartsWith(request->GetURL(), "hearth://app/")) return true;
  return false;
}
bool Client::OnOpenURLFromTab(CefRefPtr<CefBrowser> browser, CefRefPtr<CefFrame> frame, const CefString& target_url,
                              CefRequestHandler::WindowOpenDisposition target_disposition, bool user_gesture) {
  // Middle-click or Ctrl+click: a tab in the background.
  if (!win_ || (target_disposition != CEF_WOD_NEW_BACKGROUND_TAB && target_disposition != CEF_WOD_NEW_FOREGROUND_TAB)) return false;
  Tab* from = win_->ByBrowser(browser);
  win_->NewTab(target_url, target_disposition == CEF_WOD_NEW_FOREGROUND_TAB, from ? from->id : 0);
  return true;
}
CefRefPtr<CefResourceRequestHandler> Client::GetResourceRequestHandler(CefRefPtr<CefBrowser> browser, CefRefPtr<CefFrame> frame,
                                                                       CefRefPtr<CefRequest> request, bool is_navigation, bool is_download,
                                                                       const CefString& request_initiator, bool& disable_default_handling) {
  return TheAdBlock();
}
void Client::OnRenderProcessTerminated(CefRefPtr<CefBrowser> browser, TerminationStatus status
#if CEF_VERSION_MAJOR >= 124
                                       , int error_code, const CefString& error_string
#endif
                                       ) {
  G().router->OnRenderProcessTerminated(browser);
}

bool Client::OnPreKeyEvent(CefRefPtr<CefBrowser> browser, const CefKeyEvent& event, CefEventHandle os_event, bool* is_keyboard_shortcut) {
  return win_ ? win_->Shortcut(event) : false;
}

#if CEF_VERSION_MAJOR >= 125
bool
#else
void
#endif
Client::OnBeforeDownload(CefRefPtr<CefBrowser> browser, CefRefPtr<CefDownloadItem> item, const CefString& suggested_name,
                         CefRefPtr<CefBeforeDownloadCallback> callback) {
  std::string dir = Downloads();
  mkdir(dir.c_str(), 0755);
  std::string name = suggested_name.empty() ? "download" : suggested_name.ToString();
  std::string file = dir + "/" + name;
  for (int n = 1; access(file.c_str(), F_OK) == 0; n++) {
    size_t dot = name.rfind('.');
    std::string stem = dot == std::string::npos || dot == 0 ? name : name.substr(0, dot), ext = dot == std::string::npos || dot == 0 ? "" : name.substr(dot);
    file = dir + "/" + stem + " (" + std::to_string(n) + ")" + ext;
  }
  callback->Continue(file, false);
  Download d;
  d.id = item->GetId(); d.file = file; d.name = file.substr(file.rfind('/') + 1); d.url = item->GetURL(); d.total = item->GetTotalBytes();
  G().downloads.insert(G().downloads.begin(), d);
  if (G().downloads.size() > 50) G().downloads.resize(50);
  PushDownloads(true);
#if CEF_VERSION_MAJOR >= 125
  return true;
#endif
}

void Client::OnDownloadUpdated(CefRefPtr<CefBrowser> browser, CefRefPtr<CefDownloadItem> item, CefRefPtr<CefDownloadItemCallback> callback) {
  for (auto& d : G().downloads) if (d.id == item->GetId()) {
    d.got = item->GetReceivedBytes(); d.total = item->GetTotalBytes(); d.cancel = callback;
    bool done = item->IsComplete() || item->IsCanceled();
    d.state = item->IsComplete() ? "completed" : item->IsCanceled() ? "cancelled" : item->IsInterrupted() ? "interrupted" : "progressing";
    if (done) d.cancel = nullptr;
    PushDownloads(done);
    return;
  }
}

void Client::OnFindResult(CefRefPtr<CefBrowser> browser, int identifier, int count, const CefRect& selectionRect,
                          int activeMatchOrdinal, bool finalUpdate) {
  if (!win_) return;
  auto d = CefDictionaryValue::Create();
  d->SetInt("matches", count); d->SetInt("at", activeMatchOrdinal);
  win_->ToChrome("found", V(d));
}

CefSize Client::GetPreferredSize(CefRefPtr<CefView> view) {
  // The top bar is a fixed strip; tabs fill the rest.
  if (win_ && view && view->AsBrowserView()) {
    auto b = view->AsBrowserView()->GetBrowser();
    if (!b || win_->IsChrome(b)) return CefSize(800, kChromeHeight);
  }
  return CefSize(800, 600);
}

}  // namespace hearth
