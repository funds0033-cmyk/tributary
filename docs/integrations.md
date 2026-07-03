# Integrating Tributary

Patterns for using splits from another contract or application. All examples use the TypeScript sdk; the same calls work from any Soroban client.

## Marketplace fees

Create one split per seller at onboarding: seller 93%, platform 5%, referrer 2%. On each sale, route the buyer's payment through it:

```ts
await client.pay({ from: buyer, id: sellerSplit, token: USDC, amount });
```

One transaction, three balances updated, nothing to reconcile later.

## Team payroll pool

Create a mutable split with your multisig as controller and each member as a recipient. Revenue sources `deposit` into it whenever money arrives. Once per month anyone calls `distribute`. Adjusting shares when the team changes is one `update_split` from the controller.

## Referrer pools with nesting

Make the referrer share its own split: the marketplace split routes 2% to `Split(referrerPool)`, and the pool split divides that among active referrers. Updating the referrer roster never touches the marketplace split.

## Reading state

- `preview_payout(id, amount)` tells you the exact cut per recipient before sending, useful for checkout screens.
- `balance(id, token)` shows what is waiting in escrow.
- `splits_of(creator)` lists the split ids an address registered, so your app can find its own splits without an indexer.

## Events

Every state change emits an event topic-keyed by split id: `SplitCreated`, `SplitPaid`, `Deposited`, `Distributed`, `SplitUpdated`, `ControlTransferred`. Subscribe through RPC `getEvents` to build payment history or trigger webhooks.
