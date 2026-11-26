# Releases

Use unprefixed UTC CalVer for new releases.
Use unpadded month and day fields, for example `2026.11.5`.
Add `-rc.N` for a release candidate, for example `2026.11.5-rc.1`.
Compare year, month, and day as numbers in that order.

Update the Cargo package version and lockfile in a pull request.
After CI passes and the pull request merges, tag the resulting commit on `main` with the exact Cargo version.
Push the tag to start the release workflow.
The workflow verifies the static Linux archive and checksum before attaching both to the GitHub release.
GitHub marks releases with the `-rc.N` suffix as prereleases.

The [0.1.0 release notes](v0.1.0.md) describe the original release format.
