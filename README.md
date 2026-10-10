<div align="center">
  <img alt="gray-twilio" src="assets/icon.svg" width="120" height="120" />
  <h1>gray-twilio</h1>
  <p><strong>SMS channel — send texts and poll the inbox through Twilio REST.</strong></p>
  <p>
    <a href="https://gray.alignment.id">Website</a> ·
    <a href="https://github.com/vstaln/gray-twilio">Source</a> ·
    <a href="https://github.com/vstaln/gray">gray</a>
  </p>
  <p>
    <a href="LICENSE"><img alt="License: MIT" src="https://img.shields.io/badge/license-MIT-1c1c20?style=flat-square&labelColor=0a0a0b" /></a>
    <a href="https://www.rust-lang.org"><img alt="Built with Rust" src="https://img.shields.io/badge/built%20with-rust-1c1c20?style=flat-square&labelColor=0a0a0b&logo=rust&logoColor=d4a373" /></a>
    <img alt="gray plugin" src="https://img.shields.io/badge/gray-plugin-1c1c20?style=flat-square&labelColor=0a0a0b&color=7aa2f7" />
  </p>
</div>

<br/>
```bash
gray plugin install twilio
```

`gray-twilio` gives the agent an SMS surface on a Twilio number.

## Setup

```sh
twilio setup --sid AC… --token <auth-token> --from +15551234567
twilio status
```

Config: `~/.gray/twilio/config.json` (`{"sid","token","from"}`, written
0600) or env `TWILIO_SID` / `TWILIO_TOKEN` / `TWILIO_FROM`.

## Tools

| Tool | What it does |
|---|---|
| `twilio_send` | Send an SMS — `to` (E.164), `body`. |
| `twilio_inbox` | Recent inbound SMS to the configured number — `limit`. |

Inbound is **polled**, not pushed: Twilio's REST API has no local push, so
`twilio_inbox` lists recent messages addressed to your number. A live
push channel would need a public webhook — point one at your own endpoint
and call `twilio_inbox` for history either way.

`TWILIO_API` overrides the REST base for testing/proxies.

## Build from source

```sh
cargo build --release
gray plugin check ./target/release/twilio
```

## License

MIT
