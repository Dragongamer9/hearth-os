# Hearth Ideas (approved Nov 22, 2026)

The owner, Nov 22: "Hearth Ideas app would be made for it we would need a way to quickly push the code for idea we
like", then "Mockups look great". On the owner's devices only (the Store's private list), his own GitHub sign-in and his
AI team (the Hearth AI Command Center). Its inbox is desktop-layouts board 10 (checklists/desktop-layouts.md § 10).

Mockups: hearth-ideas/ideas-1-idea-page.png, hearth-ideas/ideas-2-being-built.png, hearth-ideas/ideas-3-ship-it.png
Release: next

## The window
- [x] Sidebar: New (with its count), Later, Being built, Ready to ship, Shipped, Said no, each with a count (CI pictures ideas-0…3; needs testing on a device) — hosts/hearth-ui/apps/ideas/src/main.rs: "fn sidebar"
- [x] "On your devices only. Uses your own GitHub sign-in and your AI team (the Command Center). Nothing goes out without your version number." (CI pictures ideas-0…3; needs testing on a device) — hosts/hearth-ui/apps/ideas/src/main.rs: "On your devices only. Uses your own GitHub sign-in"

## 1. An idea's page
- [x] ‹ back to the list; the idea's title; its kind (Needs something new / built by people already / has a sketch); Later, No (CI pictures ideas-0…3; needs testing on a device) — hosts/hearth-ui/apps/ideas/src/main.rs: "fn back"
- [x] The part and app, how many asked, first and newest (CI pictures ideas-0…3; needs testing on a device) — hosts/hearth-ui/apps/ideas/src/main.rs: "asked · first {} · newest {}"
- [x] What people asked, in their words (the same idea once, the rest counted: "and 20 more · 2 with a sketch") (CI pictures ideas-0…3; needs testing on a device) — hosts/hearth-ui/apps/ideas/src/main.rs: "What people asked · the same idea, in their words"
- [x] What Describe it made instead, and how many kept it (CI pictures ideas-0…3; needs testing on a device) — hosts/hearth-ui/apps/ideas/src/main.rs: "Kept by {kept} of them"
- [x] Build it with your AI team: what the Command Center gets, the repo and its own branch, the lead and the checking AI (Change ▾), Draw a mockup first, Build it (CI pictures ideas-0…3; needs testing on a device) — hosts/hearth-ui/apps/ideas/src/main.rs: "Build it with your AI team"
- [x] Or put it on the roadmap: adds it to docs/ROADMAP.md with the people's words (CI pictures ideas-0…3; needs testing on a device) — hosts/hearth-ui/apps/ideas/src/main.rs: "fn roadmap_add"

## 2. Being built
- [x] Title, Building, Open in the Command Center, Stop (CI pictures ideas-0…3; needs testing on a device) — hosts/hearth-ui/apps/ideas/src/main.rs: "Open in the Command Center"
- [x] The steps, each ticked, running or waiting: sent to the AI team (who leads, who checks, when), branch made, built and tested in GitHub Actions (the tests), pictures being taken, pull request to bughunt, ready to ship (CI pictures ideas-0…3; needs testing on a device) — hosts/hearth-ui/apps/ideas/src/main.rs: "Built and tested in GitHub Actions"
- [x] Its pictures so far (CI's, with their names and times) (CI pictures ideas-0…3; needs testing on a device) — hosts/hearth-ui/apps/ideas/src/main.rs: "Its pictures so far"
- [x] What the AI team says about it (CI pictures ideas-0…3; needs testing on a device) — hosts/hearth-ui/apps/ideas/src/main.rs: "The AI team says"

## 3. Ship it
- [x] Ready to ship; CI green, pictures match the mockup, at rest (its CPU), the pull request looked at (CI pictures ideas-0…3; needs testing on a device) — hosts/hearth-ui/apps/ideas/src/main.rs: "Pictures match the mockup"
- [x] Goes out as: a quick update (Hearth Store) or in the next Hearth OS update (CI pictures ideas-0…3; needs testing on a device) — hosts/hearth-ui/apps/ideas/src/main.rs: "A quick update (Hearth Store)"
- [x] What updates; for which Hearth OS (CI pictures ideas-0…3; needs testing on a device) — hosts/hearth-ui/apps/ideas/src/main.rs: "What updates"
- [x] Version number: empty, "Hearth never picks it"; Ship it lights up only once it's filled in (CI pictures ideas-0…3; needs testing on a device) — hosts/hearth-ui/apps/ideas/src/main.rs: "Hearth never picks it"
- [x] What's new (filled in from the idea, can be changed) (CI pictures ideas-0…3; needs testing on a device) — hosts/hearth-ui/apps/ideas/src/main.rs: "What's new"
- [x] Tell who asked (the people who asked, in their Hearth Store) (CI pictures ideas-0…3; needs testing on a device) — hosts/hearth-ui/apps/ideas/src/main.rs: "in their Hearth Store"
- [x] What happens when you press Ship: into bughunt, the quick update built (Actions › Quick update), added to the Store's list, then Publish as a second press (CI pictures ideas-0…3; needs testing on a device) — hosts/hearth-ui/apps/ideas/src/main.rs: "What happens when you press Ship"
- [x] Not yet, Ship it (CI pictures ideas-0…3; needs testing on a device) — hosts/hearth-ui/apps/ideas/src/main.rs: "Not yet"
