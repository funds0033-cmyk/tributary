import { useEffect, useState } from "react";
import { walletClient, readClient, SplitView, Recipient } from "../lib/tributary";
import RecipientEditor, {
  Row,
  rowsError,
  toRecipient,
  toShares,
} from "./RecipientEditor";

function toRows(split: SplitView): Row[] {
  return split.recipients.map((r: Recipient, i: number) => ({
    kind: r.tag === "Account" ? ("address" as const) : ("split" as const),
    value: String(r.values[0]),
    percent: String(split.shares[i] / 100),
  }));
}

export default function ManageSplit({
  wallet,
  splits,
  onChanged,
}: {
  wallet: string | null;
  splits: SplitView[];
  onChanged: () => void;
}) {
  const [splitId, setSplitId] = useState("");
  const [rows, setRows] = useState<Row[]>([]);
  const [transferTo, setTransferTo] = useState("");
  const [confirmLock, setConfirmLock] = useState(false);
  const [pendingAddr, setPendingAddr] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);

  const mine = splits.filter((s) => s.controller === wallet);
  if (!wallet || mine.length === 0) return null;

  useEffect(() => {
    if (!splitId) {
      setPendingAddr(null);
      return;
    }
    let active = true;
    readClient()
      .pending_controller({ id: BigInt(splitId) })
      .then(({ result }) => {
        if (active) setPendingAddr(result ?? null);
      })
      .catch(() => {});
    return () => { active = false; };
  }, [splitId]);

  function select(id: string) {
    setSplitId(id);
    setConfirmLock(false);
    setTransferTo("");
    setMessage(null);
    const split = mine.find((s) => String(s.id) === id);
    setRows(split ? toRows(split) : []);
  }

  async function run(action: () => Promise<string>) {
    setBusy(true);
    setMessage(null);
    try {
      setMessage(await action());
      onChanged();
    } catch (e) {
      setMessage(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  }

  async function update() {
    const invalid = rowsError(rows);
    if (invalid) {
      setMessage(invalid);
      return;
    }
    await run(async () => {
      const tx = await walletClient(wallet!).update_split({
        id: BigInt(splitId),
        recipients: rows.map(toRecipient),
        shares: toShares(rows),
      });
      const { result } = await tx.signAndSend();
      return result.isOk() ? "Split updated." : "Update rejected.";
    });
  }

  async function proposeTransfer() {
    if (!/^G[A-Z2-7]{55}$/.test(transferTo.trim())) {
      setMessage("Controller must be a G… account key.");
      return;
    }
    await run(async () => {
      const tx = await walletClient(wallet!).transfer_control({
        id: BigInt(splitId),
        new_controller: transferTo.trim(),
      });
      const { result } = await tx.signAndSend();
      return result.isOk()
        ? `Transfer proposed to ${transferTo.trim().slice(0, 4)}…${transferTo.trim().slice(-4)}. They must accept it.`
        : "Transfer proposal rejected.";
    });
  }

  async function acceptTransfer() {
    await run(async () => {
      const tx = await walletClient(wallet!).accept_control({
        id: BigInt(splitId),
      });
      const { result } = await tx.signAndSend();
      return result.isOk() ? "Control accepted. You are now the controller." : "Accept failed.";
    });
  }

  async function cancelTransfer() {
    await run(async () => {
      const tx = await walletClient(wallet!).cancel_transfer({
        id: BigInt(splitId),
      });
      const { result } = await tx.signAndSend();
      return result.isOk() ? "Pending transfer cancelled." : "Cancel failed.";
    });
  }

  async function lock() {
    if (!confirmLock) {
      setConfirmLock(true);
      setMessage("Locking is permanent. Press again to confirm.");
      return;
    }
    await run(async () => {
      const tx = await walletClient(wallet!).transfer_control({
        id: BigInt(splitId),
        new_controller: undefined,
      });
      const { result } = await tx.signAndSend();
      return result.isOk() ? "Split locked forever." : "Lock rejected.";
    });
  }

  const isPendingTarget = pendingAddr === wallet;

  return (
    <section className="card">
      <h2>Manage your splits</h2>
      <div className="row">
        <select value={splitId} onChange={(e) => select(e.target.value)}>
          <option value="">Choose split you control</option>
          {mine.map((s) => (
            <option key={String(s.id)} value={String(s.id)}>
              #{String(s.id)} · {s.recipients.length} recipients
            </option>
          ))}
        </select>
      </div>
      {splitId !== "" && (
        <>
          {pendingAddr && !isPendingTarget && (
            <p className="hint">
              Pending transfer to {pendingAddr.slice(0, 4)}…{pendingAddr.slice(-4)}.
            </p>
          )}
          {isPendingTarget && (
            <div className="row">
              <span className="hint">
                {pendingAddr.slice(0, 4)}…{pendingAddr.slice(-4)} is proposed as controller.
              </span>
              <button disabled={busy} onClick={acceptTransfer}>
                Accept control
              </button>
              <button className="ghost" disabled={busy} onClick={cancelTransfer}>
                Decline
              </button>
            </div>
          )}

          <RecipientEditor rows={rows} onChange={setRows} />
          <div className="row">
            <button disabled={busy} onClick={update}>
              Update split
            </button>
          </div>
          <div className="row">
            <input
              placeholder="G… new controller"
              value={transferTo}
              onChange={(e) => setTransferTo(e.target.value)}
            />
            <button className="ghost" disabled={busy || isPendingTarget} onClick={proposeTransfer}>
              Propose transfer
            </button>
            {pendingAddr && (
              <button className="ghost" disabled={busy} onClick={cancelTransfer}>
                Cancel transfer
              </button>
            )}
            <button className="ghost" disabled={busy} onClick={lock}>
              {confirmLock ? "Confirm lock" : "Lock forever"}
            </button>
          </div>
        </>
      )}
      {message && <p className="note">{message}</p>}
    </section>
  );
}
