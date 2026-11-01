using HearthSteam.Open;
using SteamKit2;
using SteamKit2.Internal;

namespace HearthSteam;

/// <summary>Steam Cloud through SteamKit2's public unified Cloud service messages (the same messages open projects use:
/// the app's file changelist, file download URLs, upload batches with per-file begin / blocks / commit). The sync's
/// rules are HearthSteam.Core's CloudSync; this only talks to Steam. Encrypted cloud files aren't handled (refused, so
/// Valve's client deals with that game).</summary>
sealed class SteamCloudApi : ICloudApi
{
    const ulong ClientId = 0x4853747043;   // this client's id in upload batches
    static readonly HttpClient Http = new() { Timeout = TimeSpan.FromMinutes(5) };
    readonly Cloud cloud;
    public string MachineName { get; init; } = "Hearth OS";

    public SteamCloudApi(SteamClient client) { cloud = client.GetHandler<SteamUnifiedMessages>()!.CreateService<Cloud>(); }

    static void Check(EResult r, string what) { if (r != EResult.OK) throw new EngineException(Codes.SteamError, $"Steam Cloud said no ({what}: {r})."); }

    public async Task<CloudList> List(uint app, ulong syncedChangeNumber)
    {
        var r = await cloud.GetAppFileChangelist(new CCloud_GetAppFileChangelist_Request { appid = app, synced_change_number = syncedChangeNumber });
        Check(r.Result, "file list");
        var b = r.Body;
        var files = new List<CloudFile>();
        foreach (var f in b.files)
        {
            var prefix = f.path_prefix_index < b.path_prefixes.Count ? b.path_prefixes[(int)f.path_prefix_index] : "";
            if (prefix.Length > 0 && !prefix.EndsWith('/')) prefix += "/";
            files.Add(new CloudFile(prefix + f.file_name, Convert.ToHexString(f.sha_file ?? Array.Empty<byte>()).ToLowerInvariant(), (long)f.time_stamp,
                f.raw_file_size, f.persist_state != ECloudStoragePersistState.k_ECloudStoragePersistStatePersisted));
        }
        return new CloudList(b.current_change_number, files);
    }

    public async Task<byte[]> Download(uint app, string name)
    {
        var r = await cloud.ClientFileDownload(new CCloud_ClientFileDownload_Request { appid = app, filename = name });
        Check(r.Result, "download");
        var b = r.Body;
        if (b.encrypted) throw new EngineException(Codes.Unsupported, "An encrypted Steam Cloud file: Valve's client syncs this game.");
        using var req = new HttpRequestMessage(HttpMethod.Get, (b.use_https ? "https://" : "http://") + b.url_host + b.url_path);
        foreach (var h in b.request_headers) req.Headers.TryAddWithoutValidation(h.name, h.value);
        using var resp = await Http.SendAsync(req);
        resp.EnsureSuccessStatusCode();
        return await resp.Content.ReadAsByteArrayAsync();
    }

    public async Task<ulong> BeginBatch(uint app, IReadOnlyList<string> upload, IReadOnlyList<string> delete)
    {
        var req = new CCloud_BeginAppUploadBatch_Request { appid = app, machine_name = MachineName, client_id = ClientId };
        req.files_to_upload.AddRange(upload);
        req.files_to_delete.AddRange(delete);
        var r = await cloud.BeginAppUploadBatch(req);
        Check(r.Result, "upload batch");
        return r.Body.batch_id;
    }

    static HttpMethod Method(int m) => m switch { 1 => HttpMethod.Get, 2 => HttpMethod.Head, 3 => HttpMethod.Post, 5 => HttpMethod.Delete, 7 => HttpMethod.Patch, _ => HttpMethod.Put };

    public async Task Upload(uint app, ulong batch, string name, byte[] data, byte[] sha, long time)
    {
        var r = await cloud.ClientBeginFileUpload(new CCloud_ClientBeginFileUpload_Request
        {
            appid = app, file_size = (uint)data.Length, raw_file_size = (uint)data.Length, file_sha = sha, time_stamp = (ulong)Math.Max(0, time),
            filename = name, platforms_to_sync = uint.MaxValue, can_encrypt = false, upload_batch_id = batch,
        });
        Check(r.Result, "begin upload");
        bool sent = true;
        Exception? failure = null;
        try
        {
            foreach (var blk in r.Body.block_requests)
            {
                byte[] body;
                if (blk.explicit_body_data is { Length: > 0 } x) body = x;
                else
                {
                    if (blk.block_offset + blk.block_length > (ulong)data.Length) throw new InvalidDataException("Steam asked for a block past the file's end.");
                    body = data.AsSpan((int)blk.block_offset, (int)blk.block_length).ToArray();
                }
                using var req = new HttpRequestMessage(Method(blk.http_method), (blk.use_https ? "https://" : "http://") + blk.url_host + blk.url_path) { Content = new ByteArrayContent(body) };
                foreach (var h in blk.request_headers)
                {
                    if (h.name.Equals("Content-Length", StringComparison.OrdinalIgnoreCase)) continue;
                    if (!req.Headers.TryAddWithoutValidation(h.name, h.value)) req.Content.Headers.TryAddWithoutValidation(h.name, h.value);
                }
                using var resp = await Http.SendAsync(req);
                resp.EnsureSuccessStatusCode();
            }
        }
        catch (Exception e) { sent = false; failure = e; }
        var c = await cloud.ClientCommitFileUpload(new CCloud_ClientCommitFileUpload_Request { transfer_succeeded = sent, appid = app, file_sha = sha, filename = name });
        if (!sent) throw new EngineException(Codes.SteamError, $"A save didn't go up to Steam Cloud ({failure?.GetType().Name}).");
        if (c.Result != EResult.OK || !c.Body.file_committed) throw new EngineException(Codes.SteamError, "Steam Cloud didn't keep a save.");
    }

    public async Task CompleteBatch(uint app, ulong batch, bool ok)
    {
        var r = await cloud.CompleteAppUploadBatchBlocking(new CCloud_CompleteAppUploadBatch_Request { appid = app, batch_id = batch, batch_eresult = (uint)(ok ? EResult.OK : EResult.Fail) });
        if (ok) Check(r.Result, "finish upload batch");
    }
}
