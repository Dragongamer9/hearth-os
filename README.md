# The Hearth Store's live list

`catalog.json` here is the list of apps every Hearth device's Store shows (docs/HEARTH-STORE.md on main).
It's published on its own, without a Hearth OS release:

1. Change `catalog.json` here (add an app, change one) and raise its `serial` by one.
2. Actions › **Publish the Hearth Store list** › Run workflow (only on the owner's okay).
   It checks the list (`tools/store-check.py`), signs it with the owner's key and puts it on
   hearthos.fun/store/catalog.json. Devices pick it up the next time their Store refreshes.

A device uses this list only when it's signed with the owner's key and its serial is higher than the one it has;
otherwise it keeps the list built into Hearth OS. The built-in list (os-bases/anatase/.../store/catalog.json on main)
is brought up to this one with each Hearth OS release.
