
# Extension Platform — Current and Future

## Current

The repository currently uses first-party extensions linked behind private contracts.

Do not replace this with public Wasmtime/WIT merely because the technology is attractive.

## Security principles that apply now

Extensions should not receive raw:
- PTY ownership;
- GPU device access;
- credentials;
- arbitrary process execution;
- broad network authority.

Prefer typed semantic actions/plans and narrow host services.

## Future third-party ecosystem

Wasmtime + Component Model/WIT remains a credible candidate if Automexia later ships:
- third-party downloadable extensions;
- marketplace distribution;
- untrusted public extension code;
- a stable public ABI.

That decision must go through the real ecosystem ADR with:
- capability design;
- ABI/versioning;
- sandbox updates/SLA;
- packaging;
- extension signing/trust;
- performance/startup measurements.

No immediate adoption is implied.
