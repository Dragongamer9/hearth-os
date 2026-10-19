// Hearth Browser on Chromium (CEF): a window with Hearth's top bar (tabs, address bar, menus: chrome.html)
// above the open tab. Each tab is its own Chromium browser view; the top bar talks to the program through
// CEF's message router (window.cefQuery), wrapped as window.hb, the same API the Electron version gave it.
#pragma once

#include <map>
#include <memory>
#include <set>
#include <string>
#include <vector>

#include "include/cef_app.h"
#include "include/cef_client.h"
#include "include/cef_version.h"
#include "include/views/cef_browser_view.h"
#include "include/views/cef_box_layout.h"
#include "include/views/cef_panel.h"
#include "include/views/cef_window.h"
#include "include/wrapper/cef_message_router.h"
#include "store.h"

namespace hearth {

extern const int kChromeHeight;

struct Tab {
  int id = 0;
  CefRefPtr<CefBrowserView> view;
  std::string title, favicon, url;
  bool loading = false, back = false, forward = false;
};

struct Download {
  uint32_t id = 0;
  std::string name, file, url, state = "progressing";
  int64_t total = 0, got = 0;
  CefRefPtr<CefDownloadItemCallback> cancel;
};

class Client;

// One browser window.
class BrowserWin : public CefWindowDelegate {
 public:
  explicit BrowserWin(const std::vector<std::string>& urls);

  // CefWindowDelegate
  void OnWindowCreated(CefRefPtr<CefWindow> window) override;
  void OnWindowDestroyed(CefRefPtr<CefWindow> window) override;
  bool CanClose(CefRefPtr<CefWindow> window) override;
  CefSize GetPreferredSize(CefRefPtr<CefView> view) override { return CefSize(1280, 820); }
  CefSize GetMinimumSize(CefRefPtr<CefView> view) override { return CefSize(560, 400); }

  // Tabs
  Tab* NewTab(const std::string& url, bool focus, int after = 0);
  void Select(int id);
  void Close(int id);
  Tab* Active();
  Tab* ByBrowser(CefRefPtr<CefBrowser> browser);
  bool IsChrome(CefRefPtr<CefBrowser> browser);
  bool Owns(CefRefPtr<CefBrowser> browser) { return IsChrome(browser) || ByBrowser(browser); }

  // What the top bar asks for ({act: ...}), and keyboard shortcuts from anywhere in the window.
  void Act(CefRefPtr<CefDictionaryValue> m);
  bool Shortcut(const CefKeyEvent& e);

  // To the top bar: hb.on(({name, data}) => ...)
  void ToChrome(const std::string& name, CefRefPtr<CefValue> data);
  void PushAll();
  void PushTab(Tab* t);
  void Navigate(const std::string& text);
  void Fullscreen(bool on);
  void Expand(bool on);
  void UpdateTitle();
  std::vector<std::string> Urls();

  CefRefPtr<CefWindow> window() { return window_; }
  CefRefPtr<Client> client() { return client_; }

 private:
  CefRefPtr<CefDictionaryValue> State(Tab* t);
  std::vector<std::string> start_urls_;
  CefRefPtr<CefWindow> window_;
  CefRefPtr<CefBrowserView> chrome_;
  CefRefPtr<CefPanel> content_;
  CefRefPtr<CefBoxLayout> layout_;
  CefRefPtr<Client> client_;
  std::vector<std::unique_ptr<Tab>> tabs_;
  int active_ = 0;
  bool chrome_ready_ = false, fullscreen_ = false, expanded_ = false;

  IMPLEMENT_REFCOUNTING(BrowserWin);
};

// Everything the browsers in one window report back (titles, loading, downloads, popups, keys...).
class Client : public CefClient,
               public CefLifeSpanHandler,
               public CefDisplayHandler,
               public CefLoadHandler,
               public CefRequestHandler,
               public CefKeyboardHandler,
               public CefDownloadHandler,
               public CefFindHandler,
               public CefBrowserViewDelegate {
 public:
  explicit Client(BrowserWin* win) : win_(win) {}
  void Detach() { win_ = nullptr; }

  CefRefPtr<CefLifeSpanHandler> GetLifeSpanHandler() override { return this; }
  CefRefPtr<CefDisplayHandler> GetDisplayHandler() override { return this; }
  CefRefPtr<CefLoadHandler> GetLoadHandler() override { return this; }
  CefRefPtr<CefRequestHandler> GetRequestHandler() override { return this; }
  CefRefPtr<CefKeyboardHandler> GetKeyboardHandler() override { return this; }
  CefRefPtr<CefDownloadHandler> GetDownloadHandler() override { return this; }
  CefRefPtr<CefFindHandler> GetFindHandler() override { return this; }
  bool OnProcessMessageReceived(CefRefPtr<CefBrowser> browser, CefRefPtr<CefFrame> frame, CefProcessId source,
                                CefRefPtr<CefProcessMessage> message) override;

  // Life span
  void OnAfterCreated(CefRefPtr<CefBrowser> browser) override;
  void OnBeforeClose(CefRefPtr<CefBrowser> browser) override;
  bool OnBeforePopup(CefRefPtr<CefBrowser> browser, CefRefPtr<CefFrame> frame,
#if CEF_VERSION_MAJOR >= 132
                     int popup_id,
#endif
                     const CefString& target_url, const CefString& target_frame_name,
                     CefLifeSpanHandler::WindowOpenDisposition target_disposition, bool user_gesture,
                     const CefPopupFeatures& popupFeatures, CefWindowInfo& windowInfo, CefRefPtr<CefClient>& client,
                     CefBrowserSettings& settings, CefRefPtr<CefDictionaryValue>& extra_info, bool* no_javascript_access) override;

  // Display
  void OnTitleChange(CefRefPtr<CefBrowser> browser, const CefString& title) override;
  void OnAddressChange(CefRefPtr<CefBrowser> browser, CefRefPtr<CefFrame> frame, const CefString& url) override;
  void OnFaviconURLChange(CefRefPtr<CefBrowser> browser, const std::vector<CefString>& icon_urls) override;
  void OnFullscreenModeChange(CefRefPtr<CefBrowser> browser, bool fullscreen) override;

  // Load
  void OnLoadingStateChange(CefRefPtr<CefBrowser> browser, bool isLoading, bool canGoBack, bool canGoForward) override;
  void OnLoadEnd(CefRefPtr<CefBrowser> browser, CefRefPtr<CefFrame> frame, int httpStatusCode) override;
  void OnLoadError(CefRefPtr<CefBrowser> browser, CefRefPtr<CefFrame> frame, ErrorCode errorCode,
                   const CefString& errorText, const CefString& failedUrl) override;

  // Requests: Hearth's message router, ad blocking, links that open a new tab
  bool OnBeforeBrowse(CefRefPtr<CefBrowser> browser, CefRefPtr<CefFrame> frame, CefRefPtr<CefRequest> request,
                      bool user_gesture, bool is_redirect) override;
  bool OnOpenURLFromTab(CefRefPtr<CefBrowser> browser, CefRefPtr<CefFrame> frame, const CefString& target_url,
                        CefRequestHandler::WindowOpenDisposition target_disposition, bool user_gesture) override;
  CefRefPtr<CefResourceRequestHandler> GetResourceRequestHandler(CefRefPtr<CefBrowser> browser, CefRefPtr<CefFrame> frame,
                                                                 CefRefPtr<CefRequest> request, bool is_navigation, bool is_download,
                                                                 const CefString& request_initiator, bool& disable_default_handling) override;
  void OnRenderProcessTerminated(CefRefPtr<CefBrowser> browser, TerminationStatus status
#if CEF_VERSION_MAJOR >= 124
                                 , int error_code, const CefString& error_string
#endif
                                 ) override;

  // Keys
  bool OnPreKeyEvent(CefRefPtr<CefBrowser> browser, const CefKeyEvent& event, CefEventHandle os_event, bool* is_keyboard_shortcut) override;

  // Downloads
#if CEF_VERSION_MAJOR >= 125
  bool OnBeforeDownload(CefRefPtr<CefBrowser> browser, CefRefPtr<CefDownloadItem> item, const CefString& suggested_name,
                        CefRefPtr<CefBeforeDownloadCallback> callback) override;
#else
  void OnBeforeDownload(CefRefPtr<CefBrowser> browser, CefRefPtr<CefDownloadItem> item, const CefString& suggested_name,
                        CefRefPtr<CefBeforeDownloadCallback> callback) override;
#endif
  void OnDownloadUpdated(CefRefPtr<CefBrowser> browser, CefRefPtr<CefDownloadItem> item, CefRefPtr<CefDownloadItemCallback> callback) override;

  // Find
  void OnFindResult(CefRefPtr<CefBrowser> browser, int identifier, int count, const CefRect& selectionRect,
                    int activeMatchOrdinal, bool finalUpdate) override;

  // Browser view (the top bar's size)
  CefSize GetPreferredSize(CefRefPtr<CefView> view) override;

 private:
  BrowserWin* win_;
  IMPLEMENT_REFCOUNTING(Client);
};

// Shared by all windows.
struct Globals {
  std::unique_ptr<Store> store;
  CefRefPtr<CefMessageRouterBrowserSide> router;
  std::set<BrowserWin*> windows;
  int browsers = 0;
  std::vector<Download> downloads;
  std::set<std::string> adhosts;
  bool adblock = true;
  int blocked = 0;
  std::string res_dir, version;
  bool selftest = false;           // --self-test: check the top bar and start page work, print the result, quit
  bool selftest_chrome = false, selftest_page = false;
};
Globals& G();

void OpenWindow(const std::vector<std::string>& urls);
void RegisterHearthScheme();
void InstallRouter();
void LoadAdblockList();
void PushDownloads(bool force);
CefRefPtr<CefValue> Json(const std::string& s);
CefRefPtr<CefValue> V(CefRefPtr<CefDictionaryValue> d);
CefRefPtr<CefValue> V(CefRefPtr<CefListValue> l);

}  // namespace hearth
