# 0046 — Release integrity without code-signing certificates

- **Status:** Accepted
- **Date:** 2026-10-12

## Context
Corifeu ships prebuilt binaries for six targets, installed by `curl | sh` or `irm | iex`
([ADR-0004](0004-rust-single-binary.md)). Two different problems get conflated here, and
separating them decides the question.

**Artefact integrity** — is this the binary the project built, unmodified? This is the real
threat in [SECURITY.md](../../SECURITY.md), and the one `curl | sh` is rightly criticised for.

**OS trust prompts** — Windows SmartScreen warns on binaries without an established
reputation; macOS Gatekeeper quarantines downloads. These are friction, not integrity.
Signing addresses the second and only incidentally the first.

Platform code-signing is no longer cheap. Since 2023 every Windows OV certificate requires
the private key on a hardware token or cloud HSM; file-based certificates were withdrawn.

| Option | Cost/year | Note |
|---|---|---|
| Unsigned | 0 | SmartScreen warning on first run |
| Azure Trusted Signing | ~$120 | Cheapest real signing; **requires 3+ years of verifiable business history** |
| OV certificate | $200–400 | Plus a USB token to manage and not lose |
| EV certificate | $400–600 | Immediate SmartScreen reputation |

## Decision
**Ship unsigned through 0.1.0.** Integrity is established by means that cost nothing and
address the actual threat:

1. **SHA-256 checksums** published with every release.
2. **Sigstore / cosign keyless signatures**, using GitHub OIDC. No key to manage, no
   certificate to buy, and the signature proves the artefact came from this repository's
   workflow — which is a stronger provenance claim than a code-signing certificate makes.
3. **The install scripts verify before placing anything**, and fail loudly if verification
   fails.
4. **A documented download-and-inspect path**, so `curl | sh` is a convenience rather than
   the only option.

Revisit when either becomes true: Corifeu ships a GUI installer, or the SmartScreen warning
is demonstrably costing adoption. Neither is true for a developer CLI.

## Alternatives considered
- **Azure Trusted Signing at ~$120/year.** The cheapest real signing, and the eligibility
  bar is a 3+ year verifiable business history, which a solo founder starting out does not
  clear. Revisit if that changes.
- **Buy an OV certificate anyway.** $200–400/year plus hardware-token custody, to remove a
  warning dialog for users who are already running `curl | sh` from a terminal. The money
  buys less safety than the checksum-plus-Sigstore path it would sit beside.
- **EV certificate for immediate reputation.** Same reasoning, more money.
- **Nothing at all — no checksums, no signatures.** This is what makes `curl | sh` genuinely
  indefensible, and it is cheap to avoid.

## Consequences
- **Windows users see a SmartScreen warning** if they run the binary by double-clicking.
  Documented plainly rather than hidden; users invoking it from a terminal after
  `install.ps1` are mostly unaffected.
- **macOS Gatekeeper is a smaller problem than it looks.** The quarantine attribute is set
  by browsers, not by `curl`, so a binary installed through the script is not quarantined.
  A user who downloads it manually in a browser will need to clear it, which the docs cover.
- Sigstore verification requires `cosign`, which most users will not have. Checksums remain
  the primary mechanism for them; Sigstore serves those who want provenance.
- This decision is **about money and timing, not security posture**. If the project ever
  takes sponsorship or revenue, signing is an early thing to spend it on.
