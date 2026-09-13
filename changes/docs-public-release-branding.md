# Public Linux release documentation and governance

- The public archive uses the supplied Automexia logo and version-pinned Linux
  package links, with installation, signature verification, quick-start,
  troubleshooting, removal and private security-reporting guidance.
- The repository owner explicitly extended the solo-maintainer policy to public
  metadata PRs. Required PRs, resolved discussions, signed linear history,
  immutable assets, protected tags and no-bypass rules remain enforced.
- Distribution regression tests reject missing, type-confused or drifting
  review settings. Runtime code, binaries and website activation are unchanged.
- A read-only public metadata gate checks the exact file set, unchanged logo,
  links and anchors, version-pinned downloads, trusted key and privacy markers.
  Its mutations run in the existing private CI suite; the archive stays passive.
- Public guide templates use the same branding and archive links. The portable
  instructions require a fresh extraction directory. Already published signed
  guide assets are not overwritten.
- The real public PR exposed a second approval requirement in classic branch
  protection. The read-only publication audit now checks both policy layers;
  missing evidence and fetch/argument bypasses are regression-tested.
