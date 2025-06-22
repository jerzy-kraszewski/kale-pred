import RoundCard from "./RoundCard";
import { useKaleRounds } from "../../hooks/useKaleRounds";
import { useWallet } from "../../hooks/useWallet";
import CreateRound from "./CreateRound";

/** Shows every round – active first, then earliest finality first. */
export default function AdminPanel() {
  const rounds = useKaleRounds();
  const { address } = useWallet();

  const sorted = [...rounds].sort((a, b) => {
    const aActive = a.resolved ? 1 : 0;
    const bActive = b.resolved ? 1 : 0;
    if (aActive !== bActive) return aActive - bActive; // unresolved first
    return a.finality - b.finality; // earliest finality first
  });

  if (!address) return <p>Connect wallet as admin…</p>;

  return (
    <>
      <h2>Admin panel</h2>
      <CreateRound />
      {sorted.map((r) => (
        <RoundCard key={r.id} round={r} mode="ADMIN" />
      ))}
    </>
  );
}
