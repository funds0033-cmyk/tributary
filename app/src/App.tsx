import { useCallback, useEffect, useState } from "react";
import { motion } from "motion/react";
import {
  connectWallet,
  fetchSplits,
  fetchMineIds,
  fetchActivity,
  shortAddress,
  ActivityItem,
  SplitView,
  CONTRACT_ID,
  EXPLORER,
} from "./lib/tributary";
import ActionPanel from "./components/ActionPanel";
import SplitList from "./components/SplitList";
import Activity from "./components/Activity";

const REFRESH_MS = 30_000;

const rise = {
  initial: { opacity: 0, y: 14 },
  animate: { opacity: 1, y: 0 },
};

export default function App() {
  const [wallet, setWallet] = useState<string | null>(null);
  const [splits, setSplits] = useState<SplitView[]>([]);
  const [mine, setMine] = useState<Set<string>>(new Set());
  const [activity, setActivity] = useState<ActivityItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    try {
      const [nextSplits, nextActivity, nextMine] = await Promise.all([
        fetchSplits(),
        fetchActivity().catch(() => [] as ActivityItem[]),
        wallet
          ? fetchMineIds(wallet).catch(() => new Set<string>())
          : Promise.resolve(new Set<string>()),
      ]);
      setSplits(nextSplits);
      setActivity(nextActivity);
      setMine(nextMine);
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  }, [wallet]);

  useEffect(() => {
    refresh();
    const timer = setInterval(() => {
      if (!document.hidden) refresh();
    }, REFRESH_MS);
    const onVisible = () => {
      if (!document.hidden) refresh();
    };
    document.addEventListener("visibilitychange", onVisible);
    return () => {
      clearInterval(timer);
      document.removeEventListener("visibilitychange", onVisible);
    };
  }, [refresh]);

  async function onConnect() {
    try {
      setWallet(await connectWallet());
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  }

  return (
    <div className="page">
      <header>
        <div className="brand">
          <img src="/logo.svg" alt="" width="34" height="34" />
          <span>Tributary</span>
          <span className="badge net">Testnet</span>
        </div>
        <nav>
          <a href="https://github.com/tributary-protocol/tributary">GitHub</a>
          {wallet ? (
            <span className="wallet">{shortAddress(wallet)}</span>
          ) : (
            <motion.button whileTap={{ scale: 0.97 }} onClick={onConnect}>
              Connect Freighter
            </motion.button>
          )}
        </nav>
      </header>

      <main>
        <motion.section
          className="intro"
          {...rise}
          transition={{ duration: 0.5, ease: "easeOut" }}
        >
          <h1>Split payments on Stellar</h1>
          <p>
            One transaction in, every recipient paid by their share. Running on
            testnet.
          </p>
        </motion.section>

        {error && <div className="error">{error}</div>}

        <motion.div
          {...rise}
          transition={{ duration: 0.5, ease: "easeOut", delay: 0.08 }}
        >
          <ActionPanel
            wallet={wallet}
            splits={splits}
            onChanged={refresh}
          />
        </motion.div>

        <motion.div
          {...rise}
          transition={{ duration: 0.5, ease: "easeOut", delay: 0.16 }}
        >
          <SplitList splits={splits} loading={loading} mine={mine} />
        </motion.div>

        <Activity items={activity} />
      </main>

      <footer>
        <span>Apache-2.0</span>
        <a href={`${EXPLORER}/contract/${CONTRACT_ID}`}>Contract on testnet</a>
        <a href="https://github.com/tributary-protocol/tributary">
          tributary-protocol/tributary
        </a>
      </footer>
    </div>
  );
}
