import { useState } from "react";
import { walletClient } from "../lib/tributary";
import RecipientEditor, {
  Row,
  rowsError,
  toRecipient,
  toShares,
} from "./RecipientEditor";

export default function CreateSplit({
  wallet,
  onCreated,
}: {
  wallet: string | null;
  onCreated: () => void;
}) {
  const [rows, setRows] = useState<Row[]>([
    { kind: "address", value: "", percent: "60" },
    { kind: "address", value: "", percent: "40" },
  ]);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);

  async function submit() {
    if (!wallet) {
      setMessage("Connect your wallet first.");
      return;
    }
    const invalid = rowsError(rows);
    if (invalid) {
      setMessage(invalid);
      return;
    }
    setBusy(true);
    setMessage(null);
    try {
      const client = walletClient(wallet);
      const tx = await client.create_split({
        creator: wallet,
        recipients: rows.map(toRecipient),
        shares: toShares(rows),
        controller: undefined,
      });
      const { result } = await tx.signAndSend();
      setMessage(
        result.isOk()
          ? `Split #${result.unwrap()} created.`
          : "Contract rejected the split.",
      );
      onCreated();
    } catch (e) {
      setMessage(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <section className="card">
      <h2>Create a split</h2>
      <RecipientEditor rows={rows} onChange={setRows} />
      <button disabled={busy} onClick={submit}>
        {busy ? "Waiting for signature…" : "Create split"}
      </button>
      {message && <p className="note">{message}</p>}
    </section>
  );
}
