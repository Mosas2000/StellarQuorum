# Proposal list read path — load test

Issue #170. Measures how long `getAllProposals()` takes to read 50, 200 and
1000 proposals, and uses that to recommend a page size for the list screen.

## What is being measured

`QuorumClient.getAllProposals()` ([sdk/src/client.ts](../sdk/src/client.ts)):

```ts
const count = await this.getProposalCount();          // 1 round trip
const proposals = await Promise.all(
  Array.from({ length: Number(count) }, (_, i) => this.getProposal(BigInt(i + 1)))
);                                                    // N round trips, unbounded
```

So the read path costs **1 + N RPC round trips**: one `getProposalCount()`,
then one `simulateTransaction` per proposal, all issued at once. The client
does not bound its concurrency, so the practical lower bound on wall time is
set by how many requests the RPC endpoint answers simultaneously.

The harness times the whole read path — count, per-proposal reads, and the
JSON round trip each response makes — from the caller's point of view.

## Harness

```
sdk/bench/proposal-list.bench.mjs
```

| Option | Default | Meaning |
| --- | --- | --- |
| `--mode` | `stub` | `stub` = in-process RPC stand-in, `rpc` = live endpoint |
| `--sizes` | `50,200,1000` | proposal counts to measure |
| `--page-sizes` | `10,25,50,100` | page sizes to sweep |
| `--rtt` | `40` | simulated round-trip time in ms (stub mode) |
| `--jitter` | `0.1` | ±10 % variation applied to each round trip |
| `--concurrency` | `16` | requests the RPC answers at once |
| `--payload` | `4096` | synthetic proposal size in bytes |
| `--runs` | `5` | repetitions per scenario (reports mean / p50 / p95 / max) |
| `--json` | off | machine-readable output for regenerating this table |

```bash
node sdk/bench/proposal-list.bench.mjs                    # modelled run
node sdk/bench/proposal-list.bench.mjs --json > bench.json
node sdk/bench/proposal-list.bench.mjs --mode=rpc         # needs cd sdk && npm run build
```

Three scenarios are timed per size:

- **all (current)** — what `getAllProposals()` does today: count, then every
  proposal in parallel.
- **page-first** — what a paginated list screen actually needs: count plus one
  page. Two round trips, regardless of how many proposals exist.
- **page-all** — every page in parallel, for callers that genuinely need the
  whole list (export, backfill).

### Why `--mode=stub`

`getProposal()` and `getProposalCount()` are still `throw new Error('Not
implemented')` (issues #110 and #111), and pagination does not exist yet
(issue #118), so nothing can be measured against a live chain today.
`--mode=rpc` therefore fails fast with that explanation rather than reporting
a number it did not take. Stub mode reproduces the call pattern exactly —
one count round trip, `ceil(N / concurrency)` waves of proposal reads, JSON
serialisation of every response — with the round-trip time simulated.

## Modelled results

Derived from that call pattern at the harness defaults (`rtt=40ms`,
`concurrency=16`, `payload=4KiB`), i.e. `wall ≈ rtt × (1 + ceil(wave))`.
These are **modelled**, not measured: the live path cannot run until #110 and
#111 land. Re-run the harness afterwards to replace them.

| Proposals | Strategy | RPC calls | Modelled wall time | Total payload | Largest response |
| ---: | --- | ---: | ---: | ---: | ---: |
| 50 | all (current) | 51 | 200 ms | 200 KiB | 4 KiB |
| 50 | page-first (50) | 2 | 80 ms | 200 KiB | 200 KiB |
| 50 | page-all (50) | 2 | 80 ms | 200 KiB | 200 KiB |
| 200 | all (current) | 201 | 560 ms | 800 KiB | 4 KiB |
| 200 | page-first (50) | 2 | 80 ms | 200 KiB | 200 KiB |
| 200 | page-all (50) | 5 | 80 ms | 800 KiB | 200 KiB |
| 1000 | all (current) | 1001 | 2560 ms | 3.9 MiB | 4 KiB |
| 1000 | page-first (50) | 2 | 80 ms | 200 KiB | 200 KiB |
| 1000 | page-all (50) | 21 | 120 ms | 3.9 MiB | 200 KiB |

At 1000 proposals the current shape issues **1001 simulations** and bursts
them all at once; even with 16-wide concurrency that is ~2.6 s of wall time,
and the client holds nothing back if the endpoint does not queue. Reading a
single page instead is two round trips — ~80 ms — at any list size.

Sensitivity of the current shape at N = 1000 to the round-trip time:

| RTT assumption | all (current) | page-all (50) | page-first (50) |
| --- | ---: | ---: | ---: |
| 15 ms (same region) | 960 ms | 45 ms | 30 ms |
| 40 ms (typical) | 2560 ms | 120 ms | 80 ms |
| 120 ms (cross-region / mobile) | 7680 ms | 360 ms | 240 ms |

### Page size sweep at 1000 proposals

| Page size | Pages | RPC calls | Modelled wall time | Largest response |
| ---: | ---: | ---: | ---: | ---: |
| 10 | 100 | 101 | 320 ms | 40 KiB |
| 25 | 40 | 41 | 160 ms | 100 KiB |
| 50 | 20 | 21 | 120 ms | 200 KiB |
| 100 | 10 | 11 | 80 ms | 400 KiB |

## Recommended page size: 50

- **One screenful.** The list renders a card per proposal; 50 matches what a
  user can scan before paging, so the UI never waits on rows it will not show.
- **Bounded response size.** 50 × 4 KiB ≈ 200 KiB per response, comfortably
  under the 1 MiB-class response caps public Soroban RPC endpoints impose,
  where 100 × 4 KiB = 400 KiB starts eating into that headroom.
- **Diminishing returns above 50.** Going from 50 to 100 saves one wave of a
  full sweep (120 ms → 80 ms at 1000 proposals) while doubling the largest
  response; going from 50 to 10 costs 200 ms extra per sweep for a quarter
  of the payload. Page-first reads — the path a user actually waits on —
  cost two round trips at every page size, so the choice is really about
  payload vs. request count, and 50 is the balanced point.
- **Fits the measured sizes.** 50 divides 200 and 1000 evenly, so pages stay
  full and the sweep at 50/200/1000 stays comparable.

Caveat: a page size only pays off once the read path can fetch a window
(`get_proposals(start, count)` or equivalent) instead of one proposal per
call — that is issue #118. Until then the recommendation is what the paginated
API should default to.

## Not measured here

- UI render time for 50 / 200 / 1000 cards (frontend, not the SDK read path).
- TLS setup, DNS and connection reuse on a cold client.
- Ledger contention or RPC queuing under load from other clients.
- Real proposal payloads — the harness uses `--payload` (default 4 KiB);
  pass a larger value if descriptions are typically longer.

## Regenerating the numbers

```bash
# modelled table (works today)
node sdk/bench/proposal-list.bench.mjs --json

# live table, once #110/#111/#118 are implemented
cd sdk && npm run build
node bench/proposal-list.bench.mjs --mode=rpc --sizes=50,200,1000
```
