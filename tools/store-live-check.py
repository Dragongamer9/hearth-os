#!/usr/bin/env python3
"""Game Mode › Store: asks each store's public endpoints (the ones Hearth's Store view browses with, no sign-in)
for a small answer, saves what came back, and says which ones answered as expected.

Run by CI (rust.yml, job "store-live"), warn-only: a store changing or rate-limiting its endpoint shows as a warning,
never a failed build. The saved answers (store-live/*.json, kept as a download on the run) are what the committed
test samples in hosts/hearth-core/src/storefront/fixtures/ are trimmed from when a store changes its format.

    python3 tools/store-live-check.py [out-dir]
"""
import json
import os
import sys
import time
import urllib.error
import urllib.request

UA = "Hearth (store view; https://github.com/Dragongamer9/hearth-os)"
OUT = sys.argv[1] if len(sys.argv) > 1 else "store-live"

EPIC_SEARCH = ('{Catalog{searchStore(category:"games/edition/base|bundles/games|editors|software/edition/base",count:4,'
               'country:"US",locale:"en-US",sortBy:"currentPrice",sortDir:"DESC",keywords:"hades"){elements{title id namespace '
               'description effectiveDate keyImages{type url} seller{name} productSlug urlSlug url '
               'catalogNs{mappings(pageType:"productHome"){pageSlug pageType}} offerMappings{pageSlug pageType} '
               'price(country:"US"){totalPrice{discountPrice originalPrice currencyCode fmtPrice(locale:"en-US"){originalPrice discountPrice}}}}'
               ' paging{count total}}}}')

# name, url, keys the answer must have (dotted; a number picks a list item), method/body
CHECKS = [
    ("steam_featured", "https://store.steampowered.com/api/featuredcategories?cc=US&l=english", ["specials.items", "top_sellers.items"]),
    ("steam_search", "https://store.steampowered.com/api/storesearch/?term=hades&l=english&cc=US", ["items.0.id"]),
    ("steam_details", "https://store.steampowered.com/api/appdetails?appids=1145360&cc=US&l=english", ["1145360.data.name"]),
    ("steam_reviews", "https://store.steampowered.com/appreviews/1145360?json=1&language=all&purchase_type=all&num_per_page=0", ["query_summary.review_score_desc"]),
    ("gog_catalog", "https://catalog.gog.com/v1/catalog?limit=4&order=desc:trending&productType=in:game,pack&page=1&countryCode=US&locale=en-US&currencyCode=USD", ["products.0.title"]),
    ("gog_deals", "https://catalog.gog.com/v1/catalog?limit=4&order=desc:discount&discounted=eq:true&productType=in:game,pack&page=1&countryCode=US&locale=en-US&currencyCode=USD", ["products.0.price"]),
    ("gog_search", "https://catalog.gog.com/v1/catalog?limit=4&query=like:witcher&order=desc:score&productType=in:game,pack&countryCode=US&locale=en-US&currencyCode=USD", ["products.0.id"]),
    ("gog_product", "https://api.gog.com/products/1207664663?expand=description,screenshots,videos", ["title", "description.full"]),
    ("gog_v2", "https://api.gog.com/v2/games/1207664663?locale=en-US", ["_embedded.product.title"]),
    ("gog_prices", "https://api.gog.com/products/1207664663/prices?countryCode=US", ["_embedded.prices.0.finalPrice"]),
    ("epic_free", "https://store-site-backend-static-ipv4.ak.epicgames.com/freeGamesPromotions?locale=en-US&country=US&allowCountries=US", ["data.Catalog.searchStore.elements"]),
    ("epic_graphql", "https://graphql.epicgames.com/graphql", ["data.Catalog.searchStore.elements"], "POST", json.dumps({"query": EPIC_SEARCH})),
    ("epic_store_graphql", "https://store.epicgames.com/graphql", ["data.Catalog.searchStore.elements"], "POST", json.dumps({"query": EPIC_SEARCH})),
    ("epic_discover", "https://egs-platform-service.store.epicgames.com/api/v2/public/discover/home?count=10&country=US&locale=en&platform=android&start=0&store=EGS", []),
    ("epic_content", "https://store-content-ipv4.ak.epicgames.com/api/en-US/content/products/hades", ["productName"]),
    ("xbox_reco_toppaid", "https://reco-public.rec.mp.microsoft.com/channels/Reco/V8.0/Lists/Computed/TopPaid?Market=us&Language=en&ItemTypes=Game&deviceFamily=Windows.Desktop&count=5", ["Items.0.Id"]),
    ("xbox_reco_deal", "https://reco-public.rec.mp.microsoft.com/channels/Reco/V8.0/Lists/Computed/Deal?Market=us&Language=en&ItemTypes=Game&deviceFamily=Windows.Desktop&count=5", ["Items.0.Id"]),
    ("xbox_reco_new", "https://reco-public.rec.mp.microsoft.com/channels/Reco/V8.0/Lists/Computed/New?Market=us&Language=en&ItemTypes=Game&deviceFamily=Windows.Desktop&count=5", ["Items.0.Id"]),
    ("xbox_sigl_pc", "https://catalog.gamepass.com/sigls/v2?id=fdd9e2a7-0fee-49f6-ad69-4354098401ff&language=en-us&market=US", ["1.id"]),
    ("xbox_displaycatalog", "https://displaycatalog.mp.microsoft.com/v7.0/products?bigIds=9NBLGGH4R315,9P0J5C1LQ3SH&market=US&languages=en-us", ["Products.0.LocalizedProperties.0.ProductTitle"]),
    ("xbox_autosuggest", "https://displaycatalog.mp.microsoft.com/v7.0/productFamilies/autosuggest?market=US&languages=en-us&query=halo&productFamilyNames=Games", ["Results"]),
    ("xbox_storeedge_search", "https://storeedgefd.dsx.mp.microsoft.com/v9.0/search?market=US&locale=en-US&deviceFamily=Windows.Desktop&query=halo&mediaType=games", []),
    ("xbox_storeedge_searchpage", "https://storeedgefd.dsx.mp.microsoft.com/v9.0/pages/searchResults?market=US&locale=en-US&deviceFamily=Windows.Desktop&query=halo&mediaType=games", []),
    ("xbox_storeedge_toppaid", "https://storeedgefd.dsx.mp.microsoft.com/v9.0/collections/TopPaid?market=US&locale=en-US&deviceFamily=Windows.Desktop&mediaType=games", []),
    ("xbox_emerald_search", "https://emerald.xboxservices.com/xboxcomfd/search/games?locale=en-US", [], "POST", json.dumps({"Query": "halo", "Filters": "", "ReturnFilters": False, "ChannelKeyToBeUsedInResponse": "", "ChannelId": ""})),
    ("gog_catalog_ids", "https://catalog.gog.com/v1/catalog?limit=4&productId=in:1207664663&countryCode=US&locale=en-US&currencyCode=USD", ["products.0.price"]),
    ("gog_catalog_slug", "https://catalog.gog.com/v1/catalog?limit=4&slug=eq:the_witcher_3_wild_hunt&countryCode=US&locale=en-US&currencyCode=USD", ["products.0.price"]),
    ("gog_new", "https://catalog.gog.com/v1/catalog?limit=4&order=desc:storeReleaseDate&productType=in:game,pack&countryCode=US&locale=en-US&currencyCode=USD", ["products.0.price"]),
    ("gog_bestselling", "https://catalog.gog.com/v1/catalog?limit=4&order=desc:bestselling&productType=in:game,pack&countryCode=US&locale=en-US&currencyCode=USD", ["products.0.price"]),
    ("xbox_gamepass_products", "https://catalog.gamepass.com/v3/products?market=US&language=en-us&hydration=RemoteHighSapphire0", ["Products"], "POST", json.dumps({"Products": ["9NBLGGH4R315"]})),
]

EPIC_OFFER = ('{Catalog{catalogOffer(namespace:"36a21c249987423a93f8442cc40a0e33",id:"adbc94588a184ce3a6122a19aae940ae",locale:"en-US",country:"US"){'
              'title id namespace description longDescription developerDisplayName publisherDisplayName releaseDate effectiveDate '
              'keyImages{type url} productSlug urlSlug catalogNs{mappings(pageType:"productHome"){pageSlug}} tags{id name groupName} '
              'price(country:"US"){totalPrice{discountPrice originalPrice currencyCode fmtPrice(locale:"en-US"){originalPrice discountPrice}}}}}}')
EPIC_SALE = ('{Catalog{searchStore(category:"games/edition/base|bundles/games",count:4,country:"US",locale:"en-US",onSale:true,sortBy:"relevancy",sortDir:"DESC"){'
             'elements{title id namespace keyImages{type url} productSlug urlSlug catalogNs{mappings(pageType:"productHome"){pageSlug}} '
             'price(country:"US"){totalPrice{discountPrice originalPrice currencyCode fmtPrice(locale:"en-US"){originalPrice discountPrice}}}} paging{total}}}}')
EPIC_NEW = EPIC_SALE.replace('onSale:true,sortBy:"relevancy"', 'sortBy:"releaseDate",releaseDate:"[,2026-10-24T00:00:00.000Z]"')
EPIC_TOP = ('{Storefront{collectionLayout(slug:"top-sellers",locale:"en-US",country:"US"){collectionOffers{title id namespace keyImages{type url} productSlug urlSlug '
            'catalogNs{mappings(pageType:"productHome"){pageSlug}} price(country:"US"){totalPrice{discountPrice originalPrice currencyCode fmtPrice(locale:"en-US"){originalPrice discountPrice}}}}}}}')
EPIC_TOP2 = EPIC_TOP.replace('Storefront', 'Collection')
EPIC_RATING = '{RatingsPolls{getProductResult(sandboxId:"36a21c249987423a93f8442cc40a0e33",locale:"en-US"){averageRating}}}'
SIGLS = {"pc_popular": "a884932a-f02b-40c8-a903-a008c23b1df1", "pc_recent": "163cdff5-442e-4957-97f5-1050a3546511",
         "pc_coming": "4165f752-d702-49c8-886b-fb57936f6bae", "pc_leaving": "cc7fc951-d00f-410e-9e02-5e4628e04163",
         "popular": "eab7757c-ff70-45af-bfa6-79d3cfb2bf81", "recent": "f13cf6b4-57e6-4459-89df-6aec18cf0538",
         "coming": "095bda36-f5cd-43f2-9ee1-0a72f371fb96", "leaving": "393f05bf-e596-4ef6-9487-6d4fa0eab987", "eaplay": "b8900d09-a491-44cc-916e-32b5acae621b"}
EMERALD = {"x-ms-api-version": "1.1"}
CHECKS += [
    ("epic_offer", "https://store.epicgames.com/graphql", ["data.Catalog.catalogOffer.title"], "POST", json.dumps({"query": EPIC_OFFER})),
    ("epic_sale", "https://store.epicgames.com/graphql", ["data.Catalog.searchStore.elements.0"], "POST", json.dumps({"query": EPIC_SALE})),
    ("epic_new", "https://store.epicgames.com/graphql", ["data.Catalog.searchStore.elements.0"], "POST", json.dumps({"query": EPIC_NEW})),
    ("epic_top", "https://store.epicgames.com/graphql", ["data"], "POST", json.dumps({"query": EPIC_TOP})),
    ("epic_top2", "https://store.epicgames.com/graphql", ["data"], "POST", json.dumps({"query": EPIC_TOP2})),
    ("epic_rating", "https://store.epicgames.com/graphql", ["data.RatingsPolls.getProductResult.averageRating"], "POST", json.dumps({"query": EPIC_RATING})),
    ("epic_discover_windows", "https://egs-platform-service.store.epicgames.com/api/v2/public/discover/home?count=10&country=US&locale=en&platform=windows&start=0&store=EGS", ["data.0"]),
    ("xbox_apps_toppaid", "https://apps.microsoft.com/api/Reco/GetComputedProductsList?listName=TopPaid&pgNo=1&noItems=5&filteredCategories=PCGames&mediaType=games&market=US&hl=en-us&gl=US", []),
    ("xbox_apps_deals", "https://apps.microsoft.com/api/Reco/GetComputedProductsList?listName=Deal&pgNo=1&noItems=5&filteredCategories=PCGames&mediaType=games&market=US&hl=en-us&gl=US", []),
    ("xbox_apps_new", "https://apps.microsoft.com/api/Reco/GetComputedProductsList?listName=NewAndRising&pgNo=1&noItems=5&filteredCategories=PCGames&mediaType=games&market=US&hl=en-us&gl=US", []),
    ("xbox_storeedge_gaming", "https://storeedgefd.dsx.mp.microsoft.com/v9.0/pages/gaming?market=US&locale=en-US&deviceFamily=Windows.Desktop", []),
    ("xbox_storeedge_home", "https://storeedgefd.dsx.mp.microsoft.com/v9.0/pages/home?market=US&locale=en-US&deviceFamily=Windows.Desktop&mediaType=games", []),
    ("xbox_storeedge_pdp", "https://storeedgefd.dsx.mp.microsoft.com/v9.0/products/9NWQMVSB63N4?market=US&locale=en-US&deviceFamily=Windows.Desktop", []),
    ("xbox_emerald_search_v", "https://emerald.xboxservices.com/xboxcomfd/search/games?locale=en-US", [], "POST", json.dumps({"Query": "halo", "Filters": "", "ReturnFilters": False, "ChannelKeyToBeUsedInResponse": "", "ChannelId": ""}), EMERALD),
    ("xbox_emerald_browse", "https://emerald.xboxservices.com/xboxcomfd/browse?locale=en-US", [], "POST", json.dumps({"Filters": "e30=", "ReturnFilters": False, "ChannelKeyToBeUsedInResponse": "BROWSE_CHANNELID=_FILTERS=e30=", "EncodedFilters": "e30=", "ChannelId": ""}), EMERALD),
    ("protondb", "https://www.protondb.com/api/v1/reports/summaries/1145360.json", ["tier"]),
] + [("xbox_sigl_" + k, f"https://catalog.gamepass.com/sigls/v2?id={v}&language=en-us&market=US", ["0.title", "1.id"]) for k, v in SIGLS.items()]


def dig(v, path):
    for k in path.split("."):
        if isinstance(v, list):
            try: v = v[int(k)]
            except (ValueError, IndexError): return None
        elif isinstance(v, dict):
            if k not in v: return None
            v = v[k]
        else:
            return None
    return v


def chained():
    """Requests that need an id from an earlier answer (Xbox: ids from the PC Game Pass list)."""
    try: sigl = json.load(open(os.path.join(OUT, "xbox_sigl_pc.json"), encoding="utf-8"))
    except (OSError, ValueError): return []
    ids = [x["id"] for x in sigl if isinstance(x, dict) and "id" in x][:3]
    if not ids: return []
    return [("xbox_displaycatalog_gp", "https://displaycatalog.mp.microsoft.com/v7.0/products?bigIds=" + ",".join(ids) + "&market=US&languages=en-us", ["Products.0.LocalizedProperties.0.ProductTitle"]),
            ("xbox_gamepass_products_gp", "https://catalog.gamepass.com/v3/products?market=US&language=en-us&hydration=RemoteHighSapphire0", ["Products"], "POST", json.dumps({"Products": ids}))]


def main():
    os.makedirs(OUT, exist_ok=True)
    open(os.path.join(OUT, "status.txt"), "w").close()
    good = 0
    checks = list(CHECKS)
    i = 0
    while i < len(checks):
        c = checks[i]; i += 1
        if c[0] == "xbox_gamepass_products": checks[i:i] = chained()
        name, url, need = c[0], c[1], c[2]
        method = c[3] if len(c) > 3 else "GET"
        body = c[4].encode() if len(c) > 4 else None
        more = c[5] if len(c) > 5 else {}
        req = urllib.request.Request(url, data=body, method=method, headers={**more, "User-Agent": UA, "Accept": "application/json",
            **({"Content-Type": "application/json"} if body else {}), "ms-cv": "cv", "calling-app-name": "Hearth", "calling-app-version": "1"})
        status, text = 0, ""
        try:
            with urllib.request.urlopen(req, timeout=30) as r:
                status, text = r.status, r.read().decode("utf-8", "replace")
        except urllib.error.HTTPError as e:
            status, text = e.code, e.read().decode("utf-8", "replace")[:4000]
        except Exception as e:  # noqa: BLE001 (any network trouble is a warning)
            text = f"{type(e).__name__}: {e}"
        with open(os.path.join(OUT, name + ".json"), "w", encoding="utf-8") as f:
            f.write(text)
        try: data = json.loads(text)
        except ValueError: data = None
        missing = [k for k in need if dig(data, k) is None]
        with open(os.path.join(OUT, "status.txt"), "a", encoding="utf-8") as f:
            f.write(f"{name}\t{status}\t{len(text)}\t{'missing ' + ','.join(missing) if missing else 'ok' if data is not None else 'not json'}\t{url}\n")
        if status == 200 and data is not None and not missing:
            good += 1
            print(f"::notice title=Store endpoint {name}::OK ({len(text)} bytes)")
        else:
            why = f"HTTP {status}" + (f", missing {', '.join(missing)}" if missing else "") + ("" if data is not None else ", not JSON")
            print(f"::warning title=Store endpoint {name}::{why}: {text[:300]!r}")
        time.sleep(0.5)   # gently: one request at a time
    print(f"::notice title=Store endpoints::{good} of {len(checks)} answered as expected")


if __name__ == "__main__":
    main()
