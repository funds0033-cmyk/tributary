import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { createServer, cursorLedger, decode } from "./replay.mjs";
import { upsertEvents } from "./storage.mjs";

const RPC_URL = process.env.RPC_URL ?? "https://soroban-testnet.stellar.org";
const CONTRACT_ID =
  process.env.CONTRACT_ID ?? "CCZXVZUQIZT673QF6ZGLI5AJLEPWUFWVYOPIOJNLNIOO5NI27V4JGJUU";
const OUT = process.env.OUT ?? "events.ndjson";
const STATE = process.env.STATE ?? "state.json";
const POLL_MS = Number(process.env.POLL_MS ?? 10_000);

const server = createServer(RPC_URL);

function loadCursor() {
  if (!existsSync(STATE)) return null;
  return JSON.parse(readFileSync(STATE, "utf8")).cursor ?? null;
}

function saveCursor(cursor) {
  writeFileSync(STATE, JSON.stringify({ cursor }));
}

let isPolling = false;
let shutdownRequested = false;
let intervalId;

function handleShutdown(signal) {
  console.log(`Received ${signal}. Shutting down gracefully...`);
  shutdownRequested = true;
  if (intervalId) {
    clearInterval(intervalId);
  }
  if (!isPolling) {
    console.log("State flushed. Exiting cleanly.");
    process.exit(0);
  }
}

process.on("SIGINT", () => handleShutdown("SIGINT"));
process.on("SIGTERM", () => handleShutdown("SIGTERM"));

// getEvents scans at most ~10k ledgers per call, so one poll pages the
// cursor forward until it catches up with the chain head.
async function poll() {
  if (shutdownRequested) return;
  isPolling = true;
  let cursor = loadCursor();
  const filters = [{ type: "contract", contractIds: [CONTRACT_ID] }];
  let total = 0;

  try {
    for (;;) {
      if (shutdownRequested) break;
      const request = cursor
        ? { cursor, filters, limit: 100 }
        : {
            startLedger: Math.max(
              1,
              (await server.getLatestLedger()).sequence - 100_000,
            ),
            filters,
            limit: 100,
          };

      const res = await server.getEvents(request);
      upsertEvents(OUT, res.events.map(decode));
      total += res.events.length;

      if (!res.cursor || res.cursor === cursor) break;
      cursor = res.cursor;
      saveCursor(cursor);
      if (shutdownRequested) break;
      if (res.events.length < 100 && cursorLedger(cursor) >= res.latestLedger) {
        break;
      }
    }
  } finally {
    isPolling = false;
    if (total > 0) console.log(`indexed ${total} events`);
    if (shutdownRequested) {
      console.log("State flushed. Exiting cleanly.");
      process.exit(0);
    }
  }
}

console.log(`indexing ${CONTRACT_ID} from ${RPC_URL} every ${POLL_MS}ms`);
await poll();
intervalId = setInterval(() => poll().catch((e) => console.error(e.message ?? e)), POLL_MS);
