import { useEffect, useState } from "react";
import {
  connectWallet,
  fetchSplits,
  shortAddress,
  SplitView,
  CONTRACT_ID,
  EXPLORER,
} from "./lib/tributary";
import CreateSplit from "./components/CreateSplit";
import PaySplit from "./components/PaySplit";
import EscrowCard from "./components/EscrowCard";
import SplitList from "./components/SplitList";
import Activity from "./components/Activity";

export default function App() {
  const [wallet, setWallet] = useState<string | null>(null);
  const [splits, setSplits] = useState<SplitView[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  async function refresh() {
    setLoading(true);
    try {
      setSplits(await fetchSplits());
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    refresh();
  }, []);

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
        </div>
        <nav>
          <a href="https://github.com/tributary-protocol/tributary">GitHub</a>
          {wallet ? (
            <span className="wallet">{shortAddress(wallet)}</span>
          ) : (
            <button onClick={onConnect}>Connect Freighter</button>
          )}
        </nav>
      </header>

      <main>
        <section className="intro">
          <h1>Split payments on Stellar</h1>
          <p>
            A split routes incoming funds to multiple recipients by fixed
            percentages, in one transaction. Running on testnet.
          </p>
        </section>

        {error && <div className="error">{error}</div>}

        <div className="columns">
          <CreateSplit wallet={wallet} onCreated={refresh} />
          <PaySplit wallet={wallet} splits={splits} onPaid={refresh} />
          <EscrowCard wallet={wallet} splits={splits} />
        </div>

        <div className="list-head">
          <SplitList splits={splits} loading={loading} />
          <button className="ghost" onClick={refresh} disabled={loading}>
            {loading ? "Refreshing…" : "Refresh"}
          </button>
        </div>

        <Activity />
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
