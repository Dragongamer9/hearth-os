/* Windows apps end to end: makes a Direct3D 11 device (what Unigine Heaven's DirectX 11 mode needs; through DXVK and
   Vulkan in Proton) and writes the result to dx11.txt. */
#define COBJMACROS
#include <windows.h>
#include <d3d11.h>
#include <stdio.h>

int main(void) {
    ID3D11Device *dev = NULL;
    ID3D11DeviceContext *ctx = NULL;
    D3D_FEATURE_LEVEL got = 0;
    HRESULT hr = D3D11CreateDevice(NULL, D3D_DRIVER_TYPE_HARDWARE, NULL, 0, NULL, 0, D3D11_SDK_VERSION, &dev, &got, &ctx);
    FILE *f = fopen("dx11.txt", "w");
    printf("dx11 hr=0x%08lx feature_level=0x%x\n", (unsigned long)hr, (unsigned)got);
    if (f) { fprintf(f, "hr=0x%08lx feature_level=0x%x\n", (unsigned long)hr, (unsigned)got); fclose(f); }
    if (ctx) ID3D11DeviceContext_Release(ctx);
    if (dev) ID3D11Device_Release(dev);
    return SUCCEEDED(hr) ? 0 : 2;
}
