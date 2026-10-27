# First milestone: restore the experimental dashboard

The experimental prototype remains separate from the shipping interface. Its
server uses Python's standard library, a loopback listener and an ephemeral
request token. Mantel is still the existing labwc/Quickshell desktop, with only
an optional per-user desktop shortcut for this experiment. The Linux host bridge
and shared Game Mode UI remain unchanged.

The primary image definition is `os-bases/anatase/Containerfile`, based on
`i.anatase.org/anatase:stable`; `.github/workflows/base-anatase.yml` builds it.
It includes the existing session and desktop files and ends with `bootc container
lint`. This milestone does not change that image, the installer or boot handling.
No image was built or boot-tested.

## Findings and changes

- The dashboard had an inline script while the HTTP server allowed only
  same-origin scripts. Browser execution was blocked despite passing unit tests.
  Serve the unchanged dashboard logic as a separate same-origin script.
- Valid JSON arrays, null and non-string agent names could produce HTTP 500.
  Validate the object and agent name before dictionary membership checks.
- Add API regression coverage and a browser smoke test using the actual server.
  Keep the CSP strict and avoid invoking AI during validation.

## Validation in the cloud environment

- Python 3.12: 14 unit/API tests passed.
- Node 22 + Playwright 1.48.2 + system Chromium: the real dashboard loaded
  providers, telemetry and gaming-tool detection without browser errors.
- Existing Electron channel suite: 5 tests passed.
- Existing Linux host suite: 118 passed, 1 failed (`game.close` returned
  `{ok:false}`). This unrelated process-lifecycle failure remains unresolved;
  it is not evidence that game launching/stopping works in this environment.
- PR #3's Git head matched the experimental branch at initial inspection
  (`6a3d520c0cb4db6ca6a24754043ca292919890cc`). GitHub REST access returned
  HTTP 403, so current PR review/merge/check status was not verified.

## Next boundaries

Executable detection still does not establish authentication or provider
availability. Provider version/authentication probes, permission-controlled
execution, durable sessions, and native Mantel/host integration remain future
milestones. Current CLI execution inherits the user's environment and uses their
home directory; it is not a sandbox or a safe autonomous execution service.
Do not enable unattended agents based on this milestone.

This focused fix can be reviewed independently of native integration. Promote
only through a PR; leave production boot and release configuration unchanged.
