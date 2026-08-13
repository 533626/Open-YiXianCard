# Open-YiXianCard corpus policy

## V1 decision

Player-replay-derived corpus is private engineering evidence and is not
published in V1. This decision covers admitted fixtures, incoming and
quarantine fixtures, raw or decoded replay files, original replay filenames or
codes, and derived artifacts that could be linked back to a player or match.

The current development repository contains private replay-derived fixtures so
that exact TS/Rust regression gates can run. The repository must remain private
while those files are tracked. It must not be made public, mirrored to a public
Git host, or attached to a public source release. The private corpus is not
licensed for distribution under the project MIT License.

## Public source boundary

 is zero-fixture and local-only. It contains no fixture,
fixture index, replay payload, original replay identifier, player identifier,
local path, or network upload channel. Users may explicitly select their own
supported local JSON input; the browser processes it locally.

Repository archives are not public artifacts. `.gitattributes` marks the fixture
tree `export-ignore` as an additional local-archive safeguard, but that does not
make a public repository safe: tracked files remain visible in a public Git
repository.

## Privacy boundary

Corpus approved for any future public distribution must contain none of the
following:

- player, account, platform, Steam, device, or social identifiers;
- display names, avatars, chat, contact details, or profile metadata;
- raw replay filenames, share codes, match or record identifiers, local paths,
  or timestamps that permit linkage to a player or original match;
- undocumented fields whose privacy meaning has not been reviewed.

Game-state facts needed for deterministic simulation may be retained only
after the source-linkage and identity fields are removed and a dedicated
privacy audit passes. No curated, scrubbed, sample, or demo fixture is approved
for V1 publication.

## Third-party mirrored records

Server-recorded matches obtained from a third-party mirror rather than from a
local client are private engineering evidence under the same boundary. They are
kept in a separate fixture pool, are never admitted into the canonical replay
corpus, and are not redistributed from this repository regardless of the
upstream license. The identity fields above must be removed before a mirrored
record becomes a fixture; the upstream record identifier may be retained only as
a one-way digest. The mechanical gate is `bun run check:corpus-provenance`.

## Future change procedure

Publishing any corpus later requires a separate owner-approved policy change,
a mechanical privacy gate, review of the exact files to be distributed, and a
clean separation from the private evidence store. Absence of an obvious player
name is not sufficient approval.
