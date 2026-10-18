# Hearth OS player counter

A tiny Cloudflare Worker (free plan) that counts how many Hearth OS and Ember installs are running:
installs send a random id, their version and edition about every 10 minutes; `/v1/stats` returns how many
were seen in the last 15 minutes, 24 hours and 30 days. Nothing else is stored (no IP addresses). People
can turn it off in Settings › System › "Count me in the player count".

Deployed by `.github/workflows/counter.yml` with the `CLOUDFLARE_API_TOKEN` and `CLOUDFLARE_ACCOUNT_ID`
repository secrets; it creates the D1 database the first time.
