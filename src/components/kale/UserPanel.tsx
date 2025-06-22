import RoundCard from "./RoundCard";
import { useKaleRounds } from "../../hooks/useKaleRounds";

export default function UserPanel() {
  const rounds = useKaleRounds();

  const sorted = [...rounds].sort((a, b) => {
    const aActive = a.resolved ? 1 : 0;
    const bActive = b.resolved ? 1 : 0;
    if (aActive !== bActive) return aActive - bActive;
    return a.finality - b.finality;
  });

  return (
    <>
      <h2>Kale Prediction Market</h2>
      <p>Here you can bet on the amount of invocations of Kale contract.</p>
      {sorted.map((r) => (
        <RoundCard key={r.id} round={r} mode="USER" />
      ))}
    </>
  );
}
