# Tributary

Payment splitting on Stellar.

A split is a routing rule stored on-chain: a list of recipient addresses and the share each one gets. Once a split exists, anyone can push a payment through it and every recipient gets paid in the same transaction.

Things you can do with one transfer:

- pay a whole team at once
- route a marketplace sale between seller, platform and referrer
- share donation income between project maintainers
- split royalties between collaborators

## How it works

The splitter contract keeps a registry of splits. Each split holds:

- `recipients`: the addresses that get paid
- `shares`: basis points per recipient, always summing to 10,000
- `controller`: optional. If set, that address can change the recipients and shares later. If left empty, the split is locked forever.

There are two ways money moves through a split:

- `pay` moves an amount of any Stellar asset from the payer straight to all recipients in a single call.
- `deposit` parks funds inside the contract, credited to the split. Anyone can later call `distribute` to pay the whole credited balance out to the recipients. This fits cases where money arrives over time and payouts happen on a schedule.

Per-recipient amounts are rounded down and the leftover dust goes to the last recipient, so the full amount always lands somewhere.

## Contract API

| Function | Description |
| --- | --- |
| `create_split(creator, recipients, shares, controller)` | Registers a split and returns its id |
| `pay(from, id, token, amount)` | Splits a payment across all recipients |
| `deposit(from, id, token, amount)` | Credits funds to the split without paying out |
| `distribute(id, token)` | Pays the credited balance out to all recipients |
| `balance(id, token)` | Credited amount waiting to be distributed |
| `update_split(id, recipients, shares)` | Controller only. Replaces the routing table |
| `get_split(id)` | Returns a split |
| `split_count()` | Number of splits created so far |

## Status

Early days. The core contract works, is tested and runs on testnet, but it is not audited. Do not put serious money through this yet.

## Deployments

| Network | Contract |
| --- | --- |
| Testnet | `CCUGN33DKXR36WAT7YOCMRC44XZFFHM6JNUZ7U7MDICQC22PCGY7ZJSS` |

## Development

You need stable Rust with the `wasm32v1-none` target (the checked-in `rust-toolchain.toml` sets this up automatically).

```
cargo test
cargo build --release --target wasm32v1-none -p tributary-splitter
```

## Layout

```
contracts/splitter   core splitting contract
sdk                  TypeScript client (planned)
app                  web dashboard (planned)
```

## Roadmap

- Nested splits, where a recipient is itself another split
- TypeScript SDK
- Web dashboard to create and inspect splits
- Testnet deployment, then mainnet

## Contributing

Issues and pull requests are welcome. See [CONTRIBUTING.md](CONTRIBUTING.md) for how to get set up and what a good change looks like.

## License

[Apache-2.0](LICENSE)
