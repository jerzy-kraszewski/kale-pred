import { useState } from "react";
import { Card, Button, Input, Heading, Text } from "@stellar/design-system";
import { Round, Side } from "../../hooks/useKaleRounds";

/**
 * mode = "ADMIN"  → show resolve-form (when eligible)
 * mode = "USER"   → show bet / claim UI
 */
interface RoundCardProps {
  round: Round;
  mode: "ADMIN" | "USER";
}

/* ------------------------------------------------------------------ */
/* Component                                                          */
/* ------------------------------------------------------------------ */

export default function RoundCard({ round, mode }: RoundCardProps) {
  /* -------------------------- local state ------------------------- */
  const [actual, setActual] = useState<string>("");
  const [amount, setAmount] = useState<string>("");
  const [side, setSide] = useState<Side>("LOWER");
  const [isSubmitting] = useState(false);

  /* ------------------------ helper fns ---------------------------- */
  const resolve = () => {
    console.log("resolve");
  };

  const placeBet = () => {
    console.log("Bet placed");
  };

  const claim = () => {
    console.log("Claim submitted");
  };

  /* --------------------------- render ----------------------------- */
  const totalPool = BigInt(round.high_pool) + BigInt(round.low_pool);
  const deadlinePassed = Date.now() / 1000 > round.deadline;
  const readyToResolve = !round.resolved && Date.now() / 1000 > round.finality;

  return (
    <Card variant="primary" borderRadiusSize="md">
      {/* header */}
      <Heading as="h4" size="md" style={{ margin: 0 }}>
        Round #{round.id}
      </Heading>
      <Text as="p" size="sm" style={{ margin: "4px 0 12px" }}>
        Prediction: <strong>{round.predicted}</strong> • Total pot:{" "}
        <strong>{totalPool.toString()}</strong>
      </Text>

      {/* status & interaction */}
      {round.resolved ? (
        <Text as="p" size="sm" style={{ marginTop: 12 }}>
          ✅ Resolved – winner: <strong>{round.winning_side}</strong> (
          {round.actual})
        </Text>
      ) : (
        <>
          <Text as="p" size="sm" style={{ marginTop: 12 }}>
            {deadlinePassed ? "⏱️ Betting closed" : "🪙 Open for bets"}
          </Text>

          {/* --------------------------------------------------------- */}
          {/* ADMIN MODE – resolve form                                */}
          {/* --------------------------------------------------------- */}
          {mode === "ADMIN" && readyToResolve && (
            <div style={{ marginTop: 12, display: "flex", gap: 8 }}>
              <Input
                id={`actual-${round.id}`}
                placeholder="Actual count"
                value={actual}
                onChange={(e) =>
                  setActual((e.target as HTMLInputElement).value)
                }
                fieldSize="md"
              />
              <Button
                variant="primary"
                size="sm"
                disabled={!actual || isSubmitting}
                onClick={resolve}
              >
                Resolve
              </Button>
            </div>
          )}

          {/* --------------------------------------------------------- */}
          {/* USER MODE – bet / claim                                  */}
          {/* --------------------------------------------------------- */}
          {mode === "USER" && (
            <>
              {/* simplified: always allow claim button if resolved */}
              {round.resolved ? (
                <Button
                  variant="primary"
                  size="sm"
                  disabled={isSubmitting}
                  onClick={claim}
                  style={{ marginTop: 12 }}
                >
                  Claim winnings
                </Button>
              ) : (
                !deadlinePassed && (
                  <div
                    style={{
                      marginTop: 12,
                      display: "flex",
                      gap: 6,
                      alignItems: "center",
                    }}
                  >
                    <Input
                      id={`amt-${round.id}`}
                      placeholder="Amount"
                      value={amount}
                      onChange={(e) =>
                        setAmount((e.target as HTMLInputElement).value)
                      }
                      fieldSize="md"
                    />
                    <select
                      value={side}
                      onChange={(e) => setSide(e.target.value as Side)}
                    >
                      <option value="LOWER">Lower</option>
                      <option value="HIGHER">Higher</option>
                    </select>
                    <Button
                      variant="primary"
                      size="sm"
                      disabled={!amount || isSubmitting}
                      onClick={placeBet}
                    >
                      Bet
                    </Button>
                  </div>
                )
              )}
            </>
          )}
        </>
      )}
    </Card>
  );
}
