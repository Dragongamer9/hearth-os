# os-bases/peppermint: Hearth OS on Debian (archived)

Archived Nov 19, 2026 (1.15, the owner: "Only Fedora, Archive Debian"): Hearth OS is Fedora-based now
(`os-bases/anatase`), and the Debian edition was merged into it. This folder is not moved to `archive/` because the
Fedora build copies `system_files/` from here (`os-bases/anatase/Containerfile`) and tests and CI scripts read
`build.sh`. `base-peppermint.yml` runs by hand only and doesn't publish unless ticked. See docs/ARCHIVED.md and
docs/BASE-PEPPERMINT.md.
