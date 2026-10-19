// Hearth Browser (Chromium/CEF): program start. Every Chromium process (the browser, page renderers, the GPU
// process...) starts here; only the browser process gets past CefExecuteProcess.
#include <cstdio>
#include <cstdlib>
#include <regex>
#include <string>
#include <sys/stat.h>
#include <unistd.h>
#include <limits.h>

#include "browser.h"
#include "include/cef_app.h"
#include "include/cef_command_line.h"
#include "include/cef_version.h"
#include "include/wrapper/cef_message_router.h"

using namespace hearth;

namespace {

std::string Home() { const char* h = getenv("HOME"); return h ? h : "/tmp"; }
std::string ExeDir() {
  char buf[PATH_MAX];
  ssize_t n = readlink("/proc/self/exe", buf, sizeof(buf) - 1);
  if (n <= 0) return ".";
  std::string p(buf, (size_t)n);
  return p.substr(0, p.rfind('/'));
}
void MkdirP(const std::string& p) {
  for (size_t i = 1; i <= p.size(); i++)
    if (i == p.size() || p[i] == '/') mkdir(p.substr(0, i).c_str(), 0700);
}

// Web addresses given on the command line (from a link opened elsewhere, or "hearth-browser example.com").
std::vector<std::string> UrlsFrom(CefRefPtr<CefCommandLine> cl) {
  std::vector<std::string> out;
  CefCommandLine::ArgumentList args;
  cl->GetArguments(args);
  static const std::regex looks(R"(^(https?:|file:|hearth:|[\w-]+\.[\w.-]+))", std::regex::icase);
  for (auto& a : args) {
    std::string s = a;
    if (s.empty() || s[0] == '-' || !std::regex_search(s, looks)) continue;
    std::string u = ToUrl(s, G().store ? G().store->Setting("search", "duckduckgo") : "duckduckgo");
    if (!u.empty()) out.push_back(u);
  }
  return out;
}

class RenderSide : public CefRenderProcessHandler {
 public:
  RenderSide() { CefMessageRouterConfig c; router_ = CefMessageRouterRendererSide::Create(c); }
  void OnContextCreated(CefRefPtr<CefBrowser> b, CefRefPtr<CefFrame> f, CefRefPtr<CefV8Context> ctx) override {
    // window.cefQuery only for Hearth's own pages; websites never see it.
    if (std::string(f->GetURL()).rfind("hearth://", 0) == 0) router_->OnContextCreated(b, f, ctx);
  }
  void OnContextReleased(CefRefPtr<CefBrowser> b, CefRefPtr<CefFrame> f, CefRefPtr<CefV8Context> ctx) override {
    router_->OnContextReleased(b, f, ctx);
  }
  bool OnProcessMessageReceived(CefRefPtr<CefBrowser> b, CefRefPtr<CefFrame> f, CefProcessId src, CefRefPtr<CefProcessMessage> m) override {
    return router_->OnProcessMessageReceived(b, f, src, m);
  }
 private:
  CefRefPtr<CefMessageRouterRendererSide> router_;
  IMPLEMENT_REFCOUNTING(RenderSide);
};

class App : public CefApp, public CefBrowserProcessHandler {
 public:
  App() : render_(new RenderSide()) {}
  CefRefPtr<CefBrowserProcessHandler> GetBrowserProcessHandler() override { return this; }
  CefRefPtr<CefRenderProcessHandler> GetRenderProcessHandler() override { return render_; }

  void OnRegisterCustomSchemes(CefRawPtr<CefSchemeRegistrar> registrar) override {
    registrar->AddCustomScheme("hearth", CEF_SCHEME_OPTION_STANDARD | CEF_SCHEME_OPTION_SECURE | CEF_SCHEME_OPTION_CORS_ENABLED |
                                             CEF_SCHEME_OPTION_FETCH_ENABLED);
  }

  void OnBeforeCommandLineProcessing(const CefString& process_type, CefRefPtr<CefCommandLine> cl) override {
    if (!process_type.empty()) return;
    // Video decoding on the GPU; saved site passwords in the desktop keyring.
    if (!cl->HasSwitch("enable-features")) cl->AppendSwitchWithValue("enable-features", "VaapiVideoDecodeLinuxGL,VaapiVideoDecoder");
    if (!cl->HasSwitch("password-store")) cl->AppendSwitchWithValue("password-store", "gnome-libsecret");
  }

  void OnContextInitialized() override {
    InstallRouter();
    RegisterHearthScheme();
    G().adblock = G().store->SettingOn("adblock", true);
    if (G().adblock) LoadAdblockList();
    auto cl = CefCommandLine::GetGlobalCommandLine();
    auto urls = UrlsFrom(cl);
    std::vector<std::string> open;
    if (G().store->SettingOn("restoreTabs", true)) open = G().store->Session();
    open.insert(open.end(), urls.begin(), urls.end());
    if (G().selftest) fprintf(stderr, "self-test step: opening the window\n");
    OpenWindow(open);
  }

  // "hearth-browser <link>" while it's already open: a new tab in the open window.
  bool OnAlreadyRunningAppRelaunch(CefRefPtr<CefCommandLine> cl, const CefString& dir) override {
    auto urls = UrlsFrom(cl);
    if (G().windows.empty()) { OpenWindow(urls); return true; }
    BrowserWin* w = *G().windows.rbegin();
    if (urls.empty()) w->NewTab("", true);
    else for (auto& u : urls) w->NewTab(u, true);
    if (w->window()) { w->window()->Show(); w->window()->Activate(); }
    return true;
  }

 private:
  CefRefPtr<RenderSide> render_;
  IMPLEMENT_REFCOUNTING(App);
};

}  // namespace

int main(int argc, char* argv[]) {
  CefMainArgs args(argc, argv);
  CefRefPtr<App> app = new App();
  int code = CefExecuteProcess(args, app, nullptr);
  if (code >= 0) return code;   // a helper process (renderer, GPU...) that has finished

  if (argc > 1 && std::string(argv[1]) == "--version") { printf("Hearth Browser %s (Chromium %d)\n", HEARTH_VERSION, CHROME_VERSION_MAJOR); return 0; }

  for (int i = 1; i < argc; i++) if (std::string(argv[i]) == "--self-test") G().selftest = true;
  std::string exe = ExeDir();
  G().res_dir = exe + "/hearth";
  G().version = HEARTH_VERSION;
  // Same data folder and files as the Electron version, so bookmarks, history and settings carry over.
  const char* xc = getenv("XDG_CONFIG_HOME");
  std::string data = (xc ? std::string(xc) : Home() + "/.config") + "/hearth-browser/data";
  MkdirP(data);
  G().store.reset(new Store(data));

  const char* xd = getenv("XDG_DATA_HOME");
  std::string cache = (xd ? std::string(xd) : Home() + "/.local/share") + "/hearth-browser-cef";
  MkdirP(cache);

  CefSettings s;
  s.no_sandbox = true;
  // Chromium's sandbox needs chrome-sandbox owned by root with the setuid bit (Hearth OS installs it so).
  struct stat st;
  std::string sb = exe + "/chrome-sandbox";
  if (stat(sb.c_str(), &st) == 0 && st.st_uid == 0 && (st.st_mode & S_ISUID)) s.no_sandbox = false;
  CefString(&s.root_cache_path) = cache;
  CefString(&s.cache_path) = cache + "/Default";
  CefString(&s.locale) = "en-US";
  s.persist_session_cookies = true;
  s.background_color = 0xFF1C1512;
  s.log_severity = LOGSEVERITY_WARNING;
  if (!G().selftest) CefString(&s.log_file) = cache + "/hearth-browser.log";
  else s.log_severity = LOGSEVERITY_INFO;

  if (G().selftest) fprintf(stderr, "self-test step: initializing Chromium\n");
  if (!CefInitialize(args, s, app, nullptr)) {
    // Another copy is already running (it got our links through OnAlreadyRunningAppRelaunch), or Chromium failed.
    return 0;
  }
  if (G().selftest) fprintf(stderr, "self-test step: running\n");
  CefRunMessageLoop();
  G().store->Flush();
  G().windows.clear();
  G().router = nullptr;
  CefShutdown();
  return 0;
}
