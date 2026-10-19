#include "store.h"

#include <algorithm>
#include <cctype>
#include <chrono>
#include <fstream>
#include <map>
#include <regex>
#include <set>
#include <sstream>
#include <sys/stat.h>

#include "include/cef_parser.h"

namespace hearth {

namespace {
const size_t kMaxHistory = 5000;
double Now() { return (double)std::chrono::duration_cast<std::chrono::milliseconds>(std::chrono::system_clock::now().time_since_epoch()).count(); }
std::string Lower(std::string s) { for (auto& c : s) c = (char)std::tolower((unsigned char)c); return s; }
bool StartsWith(const std::string& s, const std::string& p) { return s.compare(0, p.size(), p) == 0; }
std::string Trim(const std::string& s) {
  size_t a = s.find_first_not_of(" \t\r\n"), b = s.find_last_not_of(" \t\r\n");
  return a == std::string::npos ? "" : s.substr(a, b - a + 1);
}
std::string Clip(const std::string& s, size_t n) { return s.size() > n ? s.substr(0, n) : s; }
std::string UrlEncode(const std::string& s) {
  std::ostringstream o;
  const char* hex = "0123456789ABCDEF";
  for (unsigned char c : s) {
    if (std::isalnum(c) || c == '-' || c == '_' || c == '.' || c == '~') o << c;
    else { o << '%' << hex[c >> 4] << hex[c & 15]; }
  }
  return o.str();
}
bool IsHttp(const std::string& u) { auto l = Lower(u); return StartsWith(l, "http://") || StartsWith(l, "https://"); }
std::string Str(CefRefPtr<CefDictionaryValue> d, const char* k) { return d && d->HasKey(k) && d->GetType(k) == VTYPE_STRING ? d->GetString(k).ToString() : ""; }
CefRefPtr<CefDictionaryValue> Entry(const std::string& url, const std::string& title) {
  auto d = CefDictionaryValue::Create();
  d->SetString("url", url);
  d->SetString("title", Clip(title, 200));
  d->SetDouble("at", Now());
  return d;
}
}  // namespace

const std::vector<Engine>& Engines() {
  static const std::vector<Engine> e = {
      {"duckduckgo", "DuckDuckGo", "https://duckduckgo.com/?q=%s"},
      {"google", "Google", "https://www.google.com/search?q=%s"},
      {"bing", "Bing", "https://www.bing.com/search?q=%s"},
      {"brave", "Brave Search", "https://search.brave.com/search?q=%s"},
  };
  return e;
}

std::string ToUrl(const std::string& input, const std::string& engine) {
  std::string s = Trim(input);
  if (s.empty()) return "";
  static const std::regex scheme("^(https?|file|hearth|about|data|view-source|chrome):", std::regex::icase);
  if (std::regex_search(s, scheme)) return s;
  static const std::regex local("^(localhost|127\\.\\d+\\.\\d+\\.\\d+|\\[::1\\]|192\\.168\\.\\d+\\.\\d+|10\\.\\d+\\.\\d+\\.\\d+)(:\\d+)?(/.*)?$", std::regex::icase);
  if (std::regex_match(s, local)) return "http://" + s;
  static const std::regex domain("^[\\w-]+(?:\\.[\\w-]+)*\\.([a-zA-Z]{2,})(?::\\d+)?(?:/.*)?$");
  static const std::set<std::string> notTld = {"js", "ts", "py", "sh", "md", "rs", "cs", "cpp", "exe", "txt", "json", "html", "css", "zip", "pdf", "png", "jpg"};
  std::smatch m;
  if (s.find_first_of(" \t") == std::string::npos && std::regex_match(s, m, domain) && !notTld.count(Lower(m[1].str()))) return "https://" + s;
  std::string tpl = Engines()[0].url;
  for (auto& e : Engines()) if (e.id == engine) tpl = e.url;
  size_t at = tpl.find("%s");
  return tpl.substr(0, at) + UrlEncode(s) + tpl.substr(at + 2);
}

std::string DisplayUrl(const std::string& url) {
  if (url.empty() || StartsWith(url, "hearth://")) return "";
  std::string u = url;
  if (StartsWith(u, "https://")) u = u.substr(8);
  if (StartsWith(u, "www.")) u = u.substr(4);
  if (!u.empty() && u.back() == '/') u.pop_back();
  return u;
}

// ---------------------------------------------------------------- the store

Store::Store(const std::string& dir) : dir_(dir) {
  std::string cmd = dir;
  for (size_t i = 1; i <= cmd.size(); i++) if (i == cmd.size() || cmd[i] == '/') mkdir(cmd.substr(0, i).c_str(), 0700);
  auto v = Read("settings");
  settings_ = v && v->GetType() == VTYPE_DICTIONARY ? v->GetDictionary()->Copy(false) : CefDictionaryValue::Create();
  // Defaults (the same as the Electron version's).
  if (!settings_->HasKey("search")) settings_->SetString("search", "duckduckgo");
  if (!settings_->HasKey("home")) settings_->SetString("home", "hearth://start");
  if (!settings_->HasKey("restoreTabs")) settings_->SetBool("restoreTabs", true);
  if (!settings_->HasKey("blockPopups")) settings_->SetBool("blockPopups", true);
  if (!settings_->HasKey("adblock")) settings_->SetBool("adblock", true);
  auto list = [&](const char* n) { auto x = Read(n); return x && x->GetType() == VTYPE_LIST ? x->GetList()->Copy() : CefListValue::Create(); };
  bookmarks_ = list("bookmarks");
  history_ = list("history");
  session_ = list("session");
}

CefRefPtr<CefValue> Store::Read(const std::string& name) {
  std::ifstream f(dir_ + "/" + name + ".json");
  if (!f) return nullptr;
  std::stringstream ss; ss << f.rdbuf();
  return CefParseJSON(ss.str(), JSON_PARSER_ALLOW_TRAILING_COMMAS);
}

void Store::Dirty(const std::string& name) {
  if (std::find(dirty_.begin(), dirty_.end(), name) == dirty_.end()) dirty_.push_back(name);
}

void Store::Flush() {
  for (auto& n : dirty_) {
    auto v = CefValue::Create();
    if (n == "settings") v->SetDictionary(settings_->Copy(false));
    else if (n == "bookmarks") v->SetList(bookmarks_->Copy());
    else if (n == "history") v->SetList(history_->Copy());
    else if (n == "session") v->SetList(session_->Copy());
    std::string tmp = dir_ + "/" + n + ".json.tmp";
    { std::ofstream f(tmp, std::ios::trunc); f << CefWriteJSON(v, JSON_WRITER_DEFAULT).ToString(); }
    std::rename(tmp.c_str(), (dir_ + "/" + n + ".json").c_str());
  }
  dirty_.clear();
}

std::string Store::Setting(const std::string& key, const std::string& fallback) {
  return settings_->HasKey(key) && settings_->GetType(key) == VTYPE_STRING ? settings_->GetString(key).ToString() : fallback;
}
bool Store::SettingOn(const std::string& key, bool fallback) {
  return settings_->HasKey(key) && settings_->GetType(key) == VTYPE_BOOL ? settings_->GetBool(key) : fallback;
}
bool Store::Set(const std::string& key, CefRefPtr<CefValue> value) {
  static const std::set<std::string> known = {"search", "home", "restoreTabs", "blockPopups", "adblock"};
  if (!known.count(key) || !value) return false;
  if (key == "search") {
    bool ok = false;
    for (auto& e : Engines()) if (value->GetType() == VTYPE_STRING && e.id == value->GetString().ToString()) ok = true;
    if (!ok) return false;
  }
  settings_->SetValue(key, value->Copy());
  Dirty("settings");
  return true;
}

bool Store::IsBookmarked(const std::string& url) {
  for (size_t i = 0; i < bookmarks_->GetSize(); i++) if (Str(bookmarks_->GetDictionary(i), "url") == url) return true;
  return false;
}
void Store::ToggleBookmark(const std::string& url, const std::string& title) {
  if (!IsHttp(url)) return;
  for (size_t i = 0; i < bookmarks_->GetSize(); i++)
    if (Str(bookmarks_->GetDictionary(i), "url") == url) { bookmarks_->Remove(i); Dirty("bookmarks"); return; }
  auto fresh = CefListValue::Create();
  fresh->SetDictionary(0, Entry(url, title.empty() ? url : title));
  for (size_t i = 0; i < bookmarks_->GetSize(); i++) fresh->SetDictionary(i + 1, bookmarks_->GetDictionary(i)->Copy(false));
  bookmarks_ = fresh;
  Dirty("bookmarks");
}
void Store::RemoveBookmark(const std::string& url) {
  for (size_t i = bookmarks_->GetSize(); i-- > 0;) if (Str(bookmarks_->GetDictionary(i), "url") == url) bookmarks_->Remove(i);
  Dirty("bookmarks");
}

void Store::Visit(const std::string& url, const std::string& title) {
  if (!IsHttp(url)) return;
  if (history_->GetSize() && Str(history_->GetDictionary(0), "url") == url) {
    auto d = history_->GetDictionary(0);
    if (!title.empty()) d->SetString("title", Clip(title, 200));
    d->SetDouble("at", Now());
  } else {
    auto fresh = CefListValue::Create();
    fresh->SetDictionary(0, Entry(url, title));
    size_t n = std::min(history_->GetSize(), kMaxHistory - 1);
    for (size_t i = 0; i < n; i++) fresh->SetDictionary(i + 1, history_->GetDictionary(i)->Copy(false));
    history_ = fresh;
  }
  Dirty("history");
}
void Store::Retitle(const std::string& url, const std::string& title) {
  if (title.empty()) return;
  for (size_t i = 0; i < history_->GetSize(); i++) {
    auto d = history_->GetDictionary(i);
    if (Str(d, "url") == url) { d->SetString("title", Clip(title, 200)); Dirty("history"); return; }
  }
}
CefRefPtr<CefListValue> Store::History(const std::string& query, size_t limit) {
  auto out = CefListValue::Create();
  std::string q = Lower(query);
  for (size_t i = 0; i < history_->GetSize() && out->GetSize() < limit; i++) {
    auto d = history_->GetDictionary(i);
    if (q.empty() || Lower(Str(d, "url")).find(q) != std::string::npos || Lower(Str(d, "title")).find(q) != std::string::npos)
      out->SetDictionary(out->GetSize(), d->Copy(false));
  }
  return out;
}
void Store::ClearHistory() { history_ = CefListValue::Create(); Dirty("history"); }

CefRefPtr<CefListValue> Store::TopSites(size_t n) {
  struct Site { std::string url, title; int count = 0; size_t first = 0; };
  std::map<std::string, Site> by;
  static const std::regex origin("^(https?://[^/]+)", std::regex::icase);
  for (size_t i = 0; i < history_->GetSize(); i++) {
    auto d = history_->GetDictionary(i);
    std::smatch m; std::string u = Str(d, "url");
    if (!std::regex_search(u, m, origin)) continue;
    auto& s = by[m[1].str()];
    if (!s.count) { s.url = m[1].str() + "/"; s.first = i; }
    s.count++;
    if (s.title.empty()) s.title = Str(d, "title");
  }
  std::vector<Site> v;
  for (auto& [k, s] : by) v.push_back(s);
  std::sort(v.begin(), v.end(), [](const Site& a, const Site& b) { return a.count != b.count ? a.count > b.count : a.first < b.first; });
  auto out = CefListValue::Create();
  for (size_t i = 0; i < v.size() && i < n; i++) {
    auto d = CefDictionaryValue::Create();
    d->SetString("url", v[i].url); d->SetString("title", v[i].title);
    out->SetDictionary(i, d);
  }
  return out;
}

CefRefPtr<CefListValue> Store::Suggest(const std::string& text, size_t n) {
  auto out = CefListValue::Create();
  std::string q = Lower(Trim(text));
  if (q.empty()) return out;
  std::set<std::string> seen;
  static const std::regex prefix("^https?://(www\\.)?", std::regex::icase);
  auto consider = [&](CefRefPtr<CefDictionaryValue> d, bool bookmark) {
    if (out->GetSize() >= n) return;
    std::string u = Str(d, "url");
    if (seen.count(u)) return;
    std::string bare = Lower(std::regex_replace(u, prefix, ""));
    if (StartsWith(bare, q) || Lower(Str(d, "title")).find(q) != std::string::npos) {
      seen.insert(u);
      auto c = d->Copy(false);
      if (bookmark) c->SetBool("bookmark", true);
      out->SetDictionary(out->GetSize(), c);
    }
  };
  for (size_t i = 0; i < bookmarks_->GetSize(); i++) consider(bookmarks_->GetDictionary(i), true);
  for (size_t i = 0; i < history_->GetSize(); i++) consider(history_->GetDictionary(i), false);
  return out;
}

void Store::SaveSession(const std::vector<std::string>& urls) {
  session_ = CefListValue::Create();
  for (auto& u : urls) {
    if (session_->GetSize() >= 30) break;
    if (IsHttp(u) || StartsWith(u, "hearth:")) session_->SetString(session_->GetSize(), u);
  }
  Dirty("session");
}
std::vector<std::string> Store::Session() {
  std::vector<std::string> v;
  for (size_t i = 0; i < session_->GetSize(); i++) if (session_->GetType(i) == VTYPE_STRING) v.push_back(session_->GetString(i));
  return v;
}

}  // namespace hearth
