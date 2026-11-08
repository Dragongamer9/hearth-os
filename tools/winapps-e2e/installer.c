/* Windows apps end to end: a stand-in for an Inno Setup installer like Unigine Heaven 4.0's. Its name doesn't say
   "setup"; it carries Inno Setup's marks; it installs a program with its files' old dates (Inno keeps them), an
   uninstaller, and Start menu shortcuts (the program's with arguments and a working folder, and "Uninstall"). */
#define COBJMACROS
#include <windows.h>
#include <shlobj.h>
#include <shobjidl.h>
#include <objidl.h>
#include <stdio.h>

__attribute__((used)) static const char inno_marks[] = "Inno Setup Setup Data (6.2.0) InnoSetupLdrWindow";

static void old_date(const wchar_t *path) {
    HANDLE h = CreateFileW(path, FILE_WRITE_ATTRIBUTES, 0, NULL, OPEN_EXISTING, 0, NULL);
    SYSTEMTIME st = {2013, 6, 0, 5, 12, 0, 0, 0};
    FILETIME ft;
    if (h == INVALID_HANDLE_VALUE) return;
    SystemTimeToFileTime(&st, &ft);
    SetFileTime(h, &ft, &ft, &ft);
    CloseHandle(h);
}

static int link(const wchar_t *lnk, const wchar_t *target, const wchar_t *args, const wchar_t *dir) {
    IShellLinkW *sl = NULL;
    IPersistFile *pf = NULL;
    HRESULT hr = CoCreateInstance(&CLSID_ShellLink, NULL, CLSCTX_INPROC_SERVER, &IID_IShellLinkW, (void **)&sl);
    if (FAILED(hr)) return 1;
    IShellLinkW_SetPath(sl, target);
    if (args) IShellLinkW_SetArguments(sl, args);
    if (dir) IShellLinkW_SetWorkingDirectory(sl, dir);
    IShellLinkW_SetIconLocation(sl, target, 0);
    hr = IShellLinkW_QueryInterface(sl, &IID_IPersistFile, (void **)&pf);
    if (SUCCEEDED(hr)) { hr = IPersistFile_Save(pf, lnk, TRUE); IPersistFile_Release(pf); }
    IShellLinkW_Release(sl);
    return FAILED(hr);
}

int wmain(int argc, wchar_t **argv) {
    wchar_t me[MAX_PATH], src[MAX_PATH], menu[MAX_PATH], path[MAX_PATH], *slash;
    const wchar_t *top = L"C:\\Program Files\\Hearth Test";
    int bad = 0;
    if (argc > 99) puts(inno_marks);
    CoInitialize(NULL);
    GetModuleFileNameW(NULL, me, MAX_PATH);
    wcscpy(src, me);
    slash = wcsrchr(src, L'\\');
    if (slash) wcscpy(slash + 1, L"hello64.exe");
    CreateDirectoryW(top, NULL);
    CreateDirectoryW(L"C:\\Program Files\\Hearth Test\\bin", NULL);
    bad |= !CopyFileW(src, L"C:\\Program Files\\Hearth Test\\bin\\HearthTest.exe", FALSE);
    bad |= !CopyFileW(src, L"C:\\Program Files\\Hearth Test\\unins000.exe", FALSE);
    old_date(L"C:\\Program Files\\Hearth Test\\bin\\HearthTest.exe");
    old_date(L"C:\\Program Files\\Hearth Test\\unins000.exe");
    if (FAILED(SHGetFolderPathW(NULL, CSIDL_COMMON_PROGRAMS, NULL, 0, menu))) { printf("no start menu folder\n"); return 3; }
    wcscat(menu, L"\\Hearth Test");
    CreateDirectoryW(menu, NULL);
    swprintf(path, MAX_PATH, L"%ls\\Hearth Test.lnk", menu);
    bad |= link(path, L"C:\\Program Files\\Hearth Test\\bin\\HearthTest.exe", L"--from-shortcut \"two words\"", top);
    swprintf(path, MAX_PATH, L"%ls\\Uninstall Hearth Test.lnk", menu);
    bad |= link(path, L"C:\\Program Files\\Hearth Test\\unins000.exe", NULL, NULL);
    printf("hearth test installer: %s\n", bad ? "something failed" : "installed");
    CoUninitialize();
    return bad ? 4 : 0;
}
