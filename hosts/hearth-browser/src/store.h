// Hearth Browser's own data: bookmarks, history, settings and the open tabs, kept as JSON in its profile
// folder. Port of hosts/linux/src/web/store.js (same files and format, so the Electron version's data carries
// over). Only used on CEF's UI thread.
#pragma once

#include <string>
#include <vector>

#include "include/cef_values.h"

namespace hearth {

struct Engine { std::string id, name, url; };
const std::vector<Engine>& Engines();

// What the address bar opens for what was typed: a URL, a domain (https://), a local address (http://) or a search.
std::string ToUrl(const std::string& input, const std::string& engine);
// The address bar's quiet form: no https://, no www., no trailing slash; hearth:// pages show nothing.
std::string DisplayUrl(const std::string& url);

class Store {
 public:
  explicit Store(const std::string& dir);
  void Flush();

  CefRefPtr<CefDictionaryValue> settings() { return settings_; }
  std::string Setting(const std::string& key, const std::string& fallback);
  bool SettingOn(const std::string& key, bool fallback);
  bool Set(const std::string& key, CefRefPtr<CefValue> value);

  CefRefPtr<CefListValue> bookmarks() { return bookmarks_; }
  bool IsBookmarked(const std::string& url);
  void ToggleBookmark(const std::string& url, const std::string& title);
  void RemoveBookmark(const std::string& url);

  void Visit(const std::string& url, const std::string& title);
  void Retitle(const std::string& url, const std::string& title);
  CefRefPtr<CefListValue> History(const std::string& query, size_t limit = 300);
  void ClearHistory();
  CefRefPtr<CefListValue> TopSites(size_t n = 8);
  CefRefPtr<CefListValue> Suggest(const std::string& text, size_t n = 6);

  void SaveSession(const std::vector<std::string>& urls);
  std::vector<std::string> Session();

 private:
  CefRefPtr<CefValue> Read(const std::string& name);
  void Dirty(const std::string& name);
  std::string dir_;
  CefRefPtr<CefDictionaryValue> settings_;
  CefRefPtr<CefListValue> bookmarks_, history_, session_;
  std::vector<std::string> dirty_;
};

}  // namespace hearth
