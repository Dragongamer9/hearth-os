using HearthSteam.Open;
using SteamKit2;
using SteamKit2.Internal;

namespace HearthSteam;

/// <summary>The player's Workshop subscriptions through SteamKit2's public PublishedFile service messages (GetUserFiles
/// with "mysubscriptions", Unsubscribe). Downloading the items is the engine's depot code (UGC manifests).</summary>
sealed class SteamWorkshopApi
{
    readonly PublishedFile pub;
    public SteamWorkshopApi(SteamClient client) { pub = client.GetHandler<SteamUnifiedMessages>()!.CreateService<PublishedFile>(); }

    /// <summary>Every subscribed item of the app, or null when Steam didn't give a complete list (then nothing is
    /// removed).</summary>
    public async Task<List<WorkshopItem>?> Subscribed(uint app, ulong steamId)
    {
        var list = new List<WorkshopItem>();
        for (uint page = 1; page <= 50; page++)
        {
            var r = await pub.GetUserFiles(new CPublishedFile_GetUserFiles_Request { steamid = steamId, appid = app, type = "mysubscriptions", page = page, numperpage = 100 });
            if (r.Result != EResult.OK) return null;
            var got = r.Body.publishedfiledetails;
            foreach (var d in got)
            {
                if (d.result != (uint)EResult.OK || d.publishedfileid == 0) continue;
                list.Add(new WorkshopItem(d.publishedfileid, d.title ?? "", d.hcontent_file, d.file_size, d.time_updated, d.file_url ?? "", d.filename ?? ""));
            }
            if (got.Count == 0 || list.Count >= r.Body.total) return list;
        }
        return null;
    }

    public async Task Unsubscribe(uint app, ulong item)
    {
        var r = await pub.Unsubscribe(new CPublishedFile_Unsubscribe_Request { publishedfileid = item, list_type = 1, appid = (int)app, notify_client = true });
        if (r.Result != EResult.OK) throw new EngineException(Codes.SteamError, $"Steam didn't unsubscribe it ({r.Result}).");
    }
}
