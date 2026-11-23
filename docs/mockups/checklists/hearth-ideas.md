# Hearth Ideas (approved Nov 22, 2026)

The owner, Nov 22: "Hearth Ideas app would be made for it we would need a way to quickly push the code for idea we
like", then "Mockups look great". On the owner's devices only (the Store's private list), his own GitHub sign-in and his
AI team (the Hearth AI Command Center). Its inbox is desktop-layouts board 10 (checklists/desktop-layouts.md § 10).

Mockups: hearth-ideas/ideas-1-idea-page.png, hearth-ideas/ideas-2-being-built.png, hearth-ideas/ideas-3-ship-it.png
Release: next

## The window
- [x] Sidebar: New (with its count), Later, Being built, Ready to ship, Shipped, Said no, each with a count — hosts/hearth-ui/apps/ideas/src/main.rs: "fn sidebar" (CI pictures ideas-0…3; needs testing on a device)
- [x] "On your devices only. Uses your own GitHub sign-in and your AI team (the Command Center). Nothing goes out without your version number." — hosts/hearth-ui/apps/ideas/src/main.rs: "On your devices only. Uses your own GitHub sign-in" (CI pictures ideas-0…3; needs testing on a device)

## 1. An idea's page
- [x] ‹ back to the list; the idea's title; its kind (Needs something new / built by people already / has a sketch); Later, No — hosts/hearth-ui/apps/ideas/src/main.rs: "fn back" (CI pictures ideas-0…3; needs testing on a device)
- [x] The part and app, how many asked, first and newest — hosts/hearth-ui/apps/ideas/src/main.rs: "people asked · first" (CI pictures ideas-0…3; needs testing on a device)
- [x] What people asked, in their words (the same idea once, the rest counted: "and 20 more · 2 with a sketch") — hosts/hearth-ui/apps/ideas/src/main.rs: "What people asked · the same idea, in their words" (CI pictures ideas-0…3; needs testing on a device)
- [x] What Describe it made instead, and how many kept it — hosts/hearth-ui/apps/ideas/src/main.rs: "Kept by {kept} of them" (CI pictures ideas-0…3; needs testing on a device)
- [x] Build it with your AI team: what the Command Center gets, the repo and its own branch, the lead and the checking AI (Change ▾), Draw a mockup first, Build it — hosts/hearth-ui/apps/ideas/src/main.rs: "Build it with your AI team" (hand-off: hearth-ai-command-center electron/ideas.cjs) (CI pictures ideas-0…3; needs testing on a device)
- [x] Or put it on the roadmap: adds it to docs/ROADMAP.md with the people's words — hosts/hearth-ui/apps/ideas/src/main.rs: "fn roadmap_add" (CI pictures ideas-0…3; needs testing on a device)

## 2. Being built
- [x] Title, Building, Open in the Command Center, Stop — hosts/hearth-ui/apps/ideas/src/main.rs: "Open in the Command Center" (CI pictures ideas-0…3; needs testing on a device)
- [x] The steps, each ticked, running or waiting: sent to the AI team (who leads, who checks, when), branch made, built and tested in GitHub Actions (the tests), pictures being taken, pull request to bughunt, ready to ship — hosts/hearth-ui/apps/ideas/src/main.rs: "Built and tested in GitHub Actions" (CI pictures ideas-0…3; needs testing on a device)
- [x] Its pictures so far (CI's, with their names and times) — hosts/hearth-ui/apps/ideas/src/main.rs: "Its pictures so far" (CI pictures ideas-0…3; needs testing on a device)
- [x] What the AI team says about it — hosts/hearth-ui/apps/ideas/src/main.rs: "The AI team says" (CI pictures ideas-0…3; needs testing on a device)

## 3. Ship it
- [x] Ready to ship; CI green, pictures match the mockup, at rest (its CPU), the pull request looked at — hosts/hearth-ui/apps/ideas/src/main.rs: "Pictures match the mockup" (CI pictures ideas-0…3; needs testing on a device)
- [x] Goes out as: a quick update (Hearth Store) or in the next Hearth OS update — hosts/hearth-ui/apps/ideas/src/main.rs: "A quick update (Hearth Store)" (CI pictures ideas-0…3; needs testing on a device)
- [x] What updates; for which Hearth OS — hosts/hearth-ui/apps/ideas/src/main.rs: "What updates" (CI pictures ideas-0…3; needs testing on a device)
- [x] Version number: empty, "Hearth never picks it"; Ship it lights up only once it's filled in — hosts/hearth-ui/apps/ideas/src/main.rs: "Hearth never picks it" (CI pictures ideas-0…3; needs testing on a device)
- [x] What's new (filled in from the idea, can be changed) — hosts/hearth-ui/apps/ideas/src/main.rs: "What's new" (CI pictures ideas-0…3; needs testing on a device)
- [x] Tell who asked (the people who asked, in their Hearth Store) — hosts/hearth-ui/apps/ideas/src/main.rs: "in their Hearth Store" (CI pictures ideas-0…3; needs testing on a device)
- [x] What happens when you press Ship: into bughunt, the quick update built (Actions › Quick update), added to the Store's list, then Publish as a second press — hosts/hearth-ui/apps/ideas/src/main.rs: "What happens when you press Ship" (CI pictures ideas-0…3; needs testing on a device)
- [x] Not yet, Ship it — hosts/hearth-ui/apps/ideas/src/main.rs: "Not yet" (CI pictures ideas-0…3; needs testing on a device)
