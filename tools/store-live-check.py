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
    ("xbox_gamepass_products", "https://catalog.gamepass.com/v3/products?market=US&language=en-us&hydration=RemoteHighSapphire0", ["Products"], "POST", json.dumps({"Products": ["9NBLGGH4R315"]})),
]


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


def main():
    os.makedirs(OUT, exist_ok=True)
    good = 0
    for c in CHECKS:
        name, url, need = c[0], c[1], c[2]
        method = c[3] if len(c) > 3 else "GET"
        body = c[4].encode() if len(c) > 4 else None
        req = urllib.request.Request(url, data=body, method=method, headers={"User-Agent": UA, "Accept": "application/json",
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
        if status == 200 and data is not None and not missing:
            good += 1
            print(f"::notice title=Store endpoint {name}::OK ({len(text)} bytes)")
        else:
            why = f"HTTP {status}" + (f", missing {', '.join(missing)}" if missing else "") + ("" if data is not None else ", not JSON")
            print(f"::warning title=Store endpoint {name}::{why}: {text[:300]!r}")
        time.sleep(0.5)   # gently: one request at a time
    print(f"::notice title=Store endpoints::{good} of {len(CHECKS)} answered as expected")


if __name__ == "__main__":
    main()
