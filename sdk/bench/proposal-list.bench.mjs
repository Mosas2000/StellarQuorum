// Issue #170: load test for the proposal list read path.
//
// Measures the wall time of reading N proposals (N = 50 / 200 / 1000 unless
// --sizes says otherwise) through the same call pattern QuorumClient uses:
// one getProposalCount() round trip, then the per-proposal reads.
//
// Two modes:
//
//   --mode=stub (default)  In-process stand-in for the RPC. Every "request"
//                          costs --rtt milliseconds (with --jitter) and
//                          serialises a synthetic proposal of --payload bytes,
//                          so the shape of the read path — round trips,
//                          concurrency, JSON cost — can be compared without a
//                          funded account or a live network. The RTT is
//                          simulated; the CPU cost of serialising the payload
//                          is real.
//   --mode=rpc             Runs the built SDK against a real RPC endpoint.
//                          getProposal()/getProposalCount() are still stubs
//                          (issues #110 / #111), so this mode fails fast with
//                          an explanation until they land.
//
// Scenarios per size:
//   all          what getAllProposals() does today: count, then every
//                proposal in parallel (bounded by --concurrency, which models
//                how many requests the RPC answers at once)
//   page-first   what a paginated list screen needs: count + one page
//   page-all     every page in parallel, for callers that want everything
//
// Usage:
//   node sdk/bench/proposal-list.bench.mjs
//   node sdk/bench/proposal-list.bench.mjs --mode=stub --sizes=50,200,1000 \
//     --page-sizes=10,25,50,100 --rtt=40 --concurrency=16 --runs=5 --json
//   node sdk/bench/proposal-list.bench.mjs --mode=rpc --rpc-url=https://soroban-testnet.stellar.org

import { createRequire } from "node:module";

const DEFAULTS = {
  mode: "stub",
  sizes: "50,200,1000",
  pageSizes: "10,25,50,100",
  rtt: 40,
  jitter: 0.1,
  concurrency: 16,
  payload: 4096,
  runs: 5,
  rpcUrl: "https://soroban-testnet.stellar.org",
  json: false,
  help: false,
};

const USAGE =
  "Usage: node sdk/bench/proposal-list.bench.mjs [--mode=stub|rpc] " +
  "[--sizes=50,200,1000] [--page-sizes=10,25,50,100] [--rtt=40] [--jitter=0.1] " +
  "[--concurrency=16] [--payload=4096] [--runs=5] [--rpc-url=URL] [--json] [--help]\n";

function parseArgs(argv) {
  const options = { ...DEFAULTS };
  for (const arg of argv) {
    if (!arg.startsWith("--")) {
      throw new Error(`Unexpected argument: ${arg}\n${USAGE}`);
    }
    const [rawKey, rawValue] = arg.slice(2).split("=");
    const key = rawKey.replace(/-([a-z])/g, (_, letter) => letter.toUpperCase());
    if (!Object.prototype.hasOwnProperty.call(DEFAULTS, key)) {
      throw new Error(`Unknown option: --${rawKey}\n${USAGE}`);
    }
    if (rawValue === undefined) {
      options[key] = true;
      continue;
    }
    if (typeof DEFAULTS[key] === "number") {
      const parsed = Number(rawValue);
      if (Number.isNaN(parsed)) {
        throw new Error(`--${rawKey} expects a number, got "${rawValue}"`);
      }
      options[key] = parsed;
    } else {
      options[key] = rawValue;
    }
  }
  return options;
}

function parseList(value) {
  const list = String(value)
    .split(",")
    .map((entry) => Number(entry.trim()))
    .filter((entry) => Number.isFinite(entry) && entry > 0);
  if (list.length === 0) {
    throw new Error(`Expected a comma-separated list of positive numbers, got "${value}"`);
  }
  return list;
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

/** A bounded worker pool: at most `concurrency` calls are in flight. */
function createLimiter(concurrency) {
  let active = 0;
  const queue = [];
  const runNext = () => {
    while (active < concurrency && queue.length > 0) {
      const { task, resolve, reject } = queue.shift();
      active += 1;
      task()
        .then(resolve, reject)
        .finally(() => {
          active -= 1;
          runNext();
        });
    }
  };
  return (task) =>
    new Promise((resolve, reject) => {
      queue.push({ task, resolve, reject });
      runNext();
    });
}

function makeProposal(id, payloadBytes) {
  return {
    id,
    title: `Proposal ${id}`,
    description: "x".repeat(Math.max(0, payloadBytes)),
    proposer: "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
    status: "active",
    startTime: "2026-05-25T00:00:00Z",
    endTime: "2026-06-06T00:00:00Z",
    forVotes: 1000,
    againstVotes: 250,
    abstainVotes: 100,
    quorumRequired: 500,
    category: "Financial",
    actions: [],
    votes: [],
  };
}

function stats(samples) {
  const sorted = [...samples].sort((a, b) => a - b);
  const sum = sorted.reduce((total, value) => total + value, 0);
  const pick = (quantile) => sorted[Math.min(sorted.length - 1, Math.floor(quantile * sorted.length))];
  return {
    runs: sorted.length,
    mean: sum / sorted.length,
    min: sorted[0],
    p50: pick(0.5),
    p95: pick(0.95),
    max: sorted[sorted.length - 1],
  };
}

function round(value, digits = 1) {
  const factor = 10 ** digits;
  return Math.round(value * factor) / factor;
}

function formatKibibytes(bytes) {
  return bytes === null || bytes === undefined ? "-" : `${round(bytes / 1024, 0)} KiB`;
}

// ─── Stub mode ─────────────────────────────────────────────────────────────

function createStubTransport({ rtt, jitter, payload }) {
  const delay = () => sleep(jitter > 0 ? rtt * (1 + (Math.random() * 2 - 1) * jitter) : rtt);
  return {
    async getProposalCount(count) {
      await delay();
      return count;
    },
    async getProposal(id) {
      await delay();
      // Round trip through JSON exactly like an RPC response would.
      return JSON.parse(JSON.stringify(makeProposal(id, payload)));
    },
    async getProposalPage(startId, pageSize) {
      await delay();
      const page = Array.from({ length: pageSize }, (_, index) => makeProposal(startId + index, payload));
      return JSON.parse(JSON.stringify(page));
    },
  };
}

async function readAll(transport, count, concurrency) {
  const limit = createLimiter(concurrency);
  const total = await transport.getProposalCount(count);
  return Promise.all(
    Array.from({ length: total }, (_, index) => limit(() => transport.getProposal(index + 1))),
  );
}

async function readFirstPage(transport, count, pageSize, concurrency) {
  const limit = createLimiter(concurrency);
  const total = await transport.getProposalCount(count);
  return limit(() => transport.getProposalPage(1, Math.min(pageSize, total)));
}

async function readAllPages(transport, count, pageSize, concurrency) {
  const limit = createLimiter(concurrency);
  const total = await transport.getProposalCount(count);
  const requests = [];
  for (let start = 1; start <= total; start += pageSize) {
    const size = Math.min(pageSize, total - start + 1);
    requests.push(limit(() => transport.getProposalPage(start, size)));
  }
  return (await Promise.all(requests)).flat();
}

async function measure(runs, scenario) {
  const samples = [];
  for (let run = 0; run < runs; run += 1) {
    const started = performance.now();
    await scenario();
    samples.push(performance.now() - started);
  }
  return stats(samples);
}

// ─── RPC mode ──────────────────────────────────────────────────────────────

function loadBuiltSdk() {
  const require = createRequire(import.meta.url);
  try {
    return require("../dist/index.js");
  } catch (error) {
    throw new Error(
      `Could not load sdk/dist/index.js (${error.message}). Build the SDK first: cd sdk && npm run build`,
    );
  }
}

async function runRpcMode(options, sizes) {
  const { QuorumClient } = loadBuiltSdk();
  const client = new QuorumClient({
    rpcUrl: options.rpcUrl,
    networkPassphrase: "Test SDF Network ; September 2015",
    governanceContractId: `C${"A".repeat(55)}`,
  });

  let count;
  try {
    count = Number(await client.getProposalCount());
  } catch (error) {
    if (String(error.message).includes("Not implemented")) {
      throw new Error(
        "getProposalCount() is not implemented yet (issues #110 / #111), so the read path " +
          "cannot be measured against a live chain. Re-run with --mode=stub for the modelled " +
          "numbers, or wait for #110/#111/#118.",
      );
    }
    throw error;
  }

  const limit = createLimiter(options.concurrency);
  const results = [];
  for (const size of sizes) {
    const target = Math.min(size, count);
    if (target < size) {
      process.stderr.write(
        `warning: chain reports ${count} proposals; measuring ${target} for size ${size}\n`,
      );
    }
    const samples = [];
    for (let run = 0; run < options.runs; run += 1) {
      const started = performance.now();
      await Promise.all(
        Array.from({ length: target }, (_, index) => limit(() => client.getProposal(BigInt(index + 1)))),
      );
      samples.push(performance.now() - started);
    }
    results.push({
      size,
      strategy: "all (live)",
      ...stats(samples),
      rpcCalls: 1 + target,
      totalBytes: null,
      maxResponseBytes: null,
    });
  }
  return results;
}

// ─── Reporting ─────────────────────────────────────────────────────────────

function formatResults(results) {
  const header = ["size", "strategy", "calls", "mean ms", "p50", "p95", "max", "total", "largest"];
  const rows = results.map((result) => [
    String(result.size),
    result.strategy,
    result.rpcCalls === null ? "-" : String(result.rpcCalls),
    round(result.mean).toFixed(1),
    round(result.p50).toFixed(1),
    round(result.p95).toFixed(1),
    round(result.max).toFixed(1),
    formatKibibytes(result.totalBytes),
    formatKibibytes(result.maxResponseBytes),
  ]);
  const widths = header.map((title, column) =>
    Math.max(title.length, ...rows.map((row) => row[column].length)),
  );
  const line = (cells) => cells.map((cell, column) => cell.padEnd(widths[column])).join("  ");
  return [line(header), line(widths.map((width) => "-".repeat(width))), ...rows.map(line)].join("\n");
}

async function main() {
  const options = parseArgs(process.argv.slice(2));
  if (options.help) {
    process.stdout.write(USAGE);
    return;
  }

  const sizes = parseList(options.sizes);
  const pageSizes = parseList(options.pageSizes);
  const results = [];

  if (options.mode === "rpc") {
    results.push(...(await runRpcMode(options, sizes)));
  } else if (options.mode === "stub") {
    const transport = createStubTransport(options);
    const proposalBytes = JSON.stringify(makeProposal(1, options.payload)).length;
    const pageSize = pageSizes[Math.floor(pageSizes.length / 2)] ?? 50;

    for (const size of sizes) {
      const scenarios = [
        {
          strategy: "all (current)",
          rpcCalls: 1 + size,
          totalBytes: proposalBytes * size,
          maxResponseBytes: proposalBytes,
          run: () => readAll(transport, size, options.concurrency),
        },
        {
          strategy: `page-first (${pageSize})`,
          rpcCalls: 2,
          totalBytes: proposalBytes * Math.min(pageSize, size),
          maxResponseBytes: proposalBytes * Math.min(pageSize, size),
          run: () => readFirstPage(transport, size, pageSize, options.concurrency),
        },
        {
          strategy: `page-all (${pageSize})`,
          rpcCalls: 1 + Math.ceil(size / pageSize),
          totalBytes: proposalBytes * size,
          maxResponseBytes: proposalBytes * pageSize,
          run: () => readAllPages(transport, size, pageSize, options.concurrency),
        },
      ];
      for (const scenario of scenarios) {
        results.push({
          size,
          strategy: scenario.strategy,
          ...(await measure(options.runs, scenario.run)),
          rpcCalls: scenario.rpcCalls,
          totalBytes: scenario.totalBytes,
          maxResponseBytes: scenario.maxResponseBytes,
        });
      }
    }

    const largest = sizes[sizes.length - 1];
    for (const candidate of pageSizes) {
      results.push({
        size: largest,
        strategy: `page-all (${candidate})`,
        ...(await measure(options.runs, () =>
          readAllPages(transport, largest, candidate, options.concurrency),
        )),
        rpcCalls: 1 + Math.ceil(largest / candidate),
        totalBytes: proposalBytes * largest,
        maxResponseBytes: proposalBytes * candidate,
      });
    }
  } else {
    throw new Error(`Unknown --mode="${options.mode}" (expected stub or rpc)\n${USAGE}`);
  }

  if (options.json) {
    process.stdout.write(`${JSON.stringify({ options, results }, null, 2)}\n`);
    return;
  }

  process.stdout.write(
    `proposal list read path — mode=${options.mode} sizes=${options.sizes} ` +
      `rtt=${options.rtt}ms concurrency=${options.concurrency} payload=${options.payload}B ` +
      `runs=${options.runs}\n\n`,
  );
  process.stdout.write(`${formatResults(results)}\n`);
}

main().catch((error) => {
  process.stderr.write(`${error.message}\n`);
  process.exitCode = 1;
});
