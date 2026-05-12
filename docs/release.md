# Release readiness

## Version source
- Release version comes from the Git tag being pushed (for example, `v1.2.3`) and is the source of record for published artifacts.
- CI and release workflows consume this tag as the canonical version input.

## Changelog and evidence
- Release notes are published with each tagged release and include noteworthy behavior changes, migration notes, and verification status.
- Link evidence in each release entry to the exact `rtk just` proof command output used for that release.

## Release process
- Prerequisites: run required proof lanes and record exit status in the release notes.
- Publishing is triggered through the repository release workflow after proofs and tag validation pass.
- Artifacts are attached to the release entry with immutable references.

## Integrity and provenance
- Artifacts are produced only from CI pipeline runs that match the release workflow configuration and repository commit SHA.
- Integrity is preserved by immutable tags and exact workflow versions.

## Rollback guidance
- If a release is found faulty, cut a patch release for the corrected change and deprecate the affected version in release notes.
- For urgent rollback, stop deployment, publish a replacement release, and post a postmortem with remediation evidence.
