# Kyra proto spike — Phase 0

## What this is

A standalone Rust binary that connects to a real Minecraft server as a
passthrough client-side stub, performs the login handshake in offline
mode, and decodes `player_info`, `player_remove`, and `teams` /
`scoreboard_team` packets in the play state, logging the decoded output.

It does not integrate with Tauri and does not forward traffic between a
real client and a real server — that is Phase 1.

## Status

62/62 unit tests passing, built and tested against every decoder bracket
using hand-constructed fixture bytes matched against `minecraft-data`
schemas. Never run against a live server: this sandbox cannot reach
Minecraft server ports.

## Build

Requires a current stable Rust toolchain (developed against crate
versions current as of September 2026; the code was iterated against
slightly older MSRV-compatible pins locally due to a sandbox toolchain
limitation — the RustCrypto `aes`/`cfb8` API used here has been stable
across recent minor versions, but run `cargo build` first and adjust
`Cargo.toml` pins if that assumption doesn't hold at whatever exact
version resolves).

```
cargo build --release
cargo test
```

## Run

```
./target/release/kyra_proto_spike --host play.pika-network.net --port 25565 --protocol-version 773 --username YourName
```

`--protocol-version` has to be one the packet-id table covers — see
`src/protocol/packet_ids.rs`. Protocol 776 (Minecraft 26.2, PikaNetwork's
current version) is deliberately not in the table yet; it will fail
fast with an explicit "not in the supported packet-id table" error
rather than guess. The closest confirmed bracket is 773–775 (1.21.10,
1.21.11, 26.1).

Logs go to stderr via `tracing`. Set `RUST_LOG=debug` for more detail if
needed later.

## Known gaps / next steps

- **Protocol 776 is unconfirmed.** `minecraft-data` doesn't have it
  yet. If Pika/Jartex reject connections at any other bracket, or if
  776 turns out to have changed `player_info`/`teams` again, that's the
  first thing to check.
- **Premium/online-mode auth is not implemented.** If the server sends
  `encryption_request`, this binary logs a warning and exits cleanly
  rather than completing the Microsoft auth handshake — that's Phase 4
  scope, not Phase 0. The AES-CFB8 primitive itself (`src/crypto.rs`)
  is fully implemented and tested against the `cfb8` crate's own
  known-answer vector; it just isn't wired into a live encrypted session
  yet, which is why `cargo build` reports a few unused-code warnings
  for it. If Pika/Jartex turns out to be online-mode-only for your
  account, this is where to pick up.
- **Compression threshold path is implemented but untested live** —
  covered by synthetic zlib fixtures in `src/framing.rs`'s tests, not
  by an actual server's `set_compression` packet yet.

## How to report results back

Run it against a real Bedwars game on Pika or Jartex and capture the
logged `player_info entry` / `teams decoded` lines, ideally alongside
what the existing TypeScript proxy logs for the same session. Mismatches
there — wrong names, garbled display names, wrong team colors, wrong
add/remove classification — point at exactly which bracket or field
needs a second look.
