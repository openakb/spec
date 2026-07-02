# Security Policy

## Reporting a vulnerability

Please report suspected vulnerabilities privately to **<security@openakb.org>**. Do not open a
public issue for a security report. We aim to acknowledge within 3 business days and will
coordinate disclosure with you — please give us reasonable time to release a fix before any
public write-up.

Report both classic software vulnerabilities (in a validator or CI script) and specification
or schema weaknesses that could put implementers or consumers at risk.

## Supported versions

OpenAKB is pre-1.0. Until 1.0, only the latest state of the default branch is supported. This
table is updated when release streams begin.

| Version | Supported |
| ------- | --------- |
| main    | ✅        |

## Threat model (summary, advisory)

OpenAKB descriptors are read by agents, and their URIs are resolved by tooling. Treat this as a
security-relevant surface:

- **All AKB content is untrusted until verified — first-party and linked alike.** Content fetched
  from any AKB is a prompt-injection surface; a cross-AKB link is no more and no less trusted than
  a first-party section.
- **Restrict URI schemes.** Resolve only expected schemes per field (`https`, and `file`/relative
  only for a local working copy). Treat `data:`, `javascript:`, `blob:`, and unexpected schemes as
  hostile and do not auto-fetch them. A resolver that fetches author-supplied URIs is an SSRF
  vector — sandbox it and restrict egress.
- **Bound every fetch.** Content is unbounded payload behind a URI; apply size, time, and
  redirect-count limits and stream-and-cap rather than buffering whole responses.
- **Integrity is not authenticity.** Content and provenance hashes attest that served bytes were
  not corrupted; they are not signatures and do not attest authorship. There is no descriptor
  signing primitive before 1.0; an attestation layer is a candidate for a post-1.0 proposal.

The normative rules that back this advice ship with the specification in a later phase.
