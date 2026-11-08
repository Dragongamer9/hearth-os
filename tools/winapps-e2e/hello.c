/* Windows apps end to end (winapps.yml): a tiny console program. Prints what it got and writes ran.txt in the folder
   it was started in, so the test sees that Proton really ran it, as which bitness, where, and with which arguments. */
#include <windows.h>
#include <stdio.h>

int main(int argc, char **argv) {
    char cwd[MAX_PATH] = "";
    int i;
    FILE *f;
    GetCurrentDirectoryA(MAX_PATH, cwd);
    printf("hearth-winapps-test ok bits=%d cwd=%s args=", (int)(sizeof(void *) * 8), cwd);
    for (i = 1; i < argc; i++) printf("[%s]", argv[i]);
    printf("\n");
    fflush(stdout);
    fprintf(stderr, "hearth-winapps-test stderr line\n");
    f = fopen("ran.txt", "w");
    if (f) {
        fprintf(f, "ran bits=%d args=", (int)(sizeof(void *) * 8));
        for (i = 1; i < argc; i++) fprintf(f, "[%s]", argv[i]);
        fprintf(f, "\n");
        fclose(f);
    }
    return 0;
}
