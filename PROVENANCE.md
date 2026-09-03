# Provenance

This repository is a spec-first cleanroom implementation. Record here what was consulted.

## Specifications used
- The templates repository's README ([public-software/templates](https://github.com/public-software/templates), `Apache-2.0 OR MIT`): the layout of the skeleton set (`crate/<kind>/`) and the placeholder contract (`{{NAME}}` replaced verbatim, non-text files copied byte for byte, the organization / repository / crate vocabularies) that `pub new` implements.
- The Cargo Book, [The Manifest Format](https://doc.rust-lang.org/cargo/reference/manifest.html) (`MIT OR Apache-2.0`): package name rules (alphanumeric, `-`, `_`; crates.io compares names case-insensitively), which the organization's stricter component rule (lowercase words joined by hyphens) sits inside.

## Behavioural references (cited, not copied)
- The bootstrap kit's `lib/common.sh` (`render`, `render_crate`; the organization's own, `Apache-2.0 OR MIT`) and the contributor workspace's `bin/ps-new-crate`: the reference rendering `pub new` reproduces byte for byte (trailing newlines collapsed to one, `.DS_Store` skipped, images by extension copied), verified by diffing the five kinds.
- `git clone --depth 1 --branch <ref>` ([git-clone documentation](https://git-scm.com/docs/git-clone), GPL-2.0 documentation of a tool invoked as a subprocess, not a source consulted): a shallow clone pins a tag or a branch, not a bare commit, which is why `--ref` names one of those.

## Copyleft sources
None consulted. Contributors who have studied GPL/AGPL implementations of this domain do not author the corresponding modules (two-team rule; see the Charter §09).

## AI assistance
Prompts point at the specifications and conformance suites above, never at copyleft source. Generated code is reviewed against this list before merge.
