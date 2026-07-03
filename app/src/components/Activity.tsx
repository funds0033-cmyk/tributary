import { useEffect, useState } from "react";
import { fetchActivity, fromStroops, ActivityItem, EXPLORER } from "../lib/tributary";

const LABELS: Record<string, string> = {
  split_created: "created",
  split_paid: "paid",
  split_updated: "updated",
  deposited: "deposit",
  distributed: "distributed",
  control_transferred: "control moved",
};

export default function Activity() {
  const [items, setItems] = useState<ActivityItem[]>([]);
  const [loaded, setLoaded] = useState(false);

  useEffect(() => {
    fetchActivity()
      .then(setItems)
      .finally(() => setLoaded(true));
  }, []);

  if (!loaded || items.length === 0) return null;

  return (
    <section className="activity">
      <h2>Recent activity</h2>
      <ul>
        {items.map((item, i) => (
          <li key={item.txHash + i}>
            <span className="badge">{LABELS[item.type] ?? item.type}</span>
            <span>
              {item.id !== undefined && `split #${String(item.id)}`}
              {item.amount !== undefined && ` · ${fromStroops(item.amount)} XLM`}
            </span>
            <a
              href={`${EXPLORER}/tx/${item.txHash}`}
              target="_blank"
              rel="noreferrer"
            >
              tx
            </a>
          </li>
        ))}
      </ul>
    </section>
  );
}
