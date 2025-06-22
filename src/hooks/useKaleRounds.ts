import { useState } from "react";
import { useSubscription } from "./useSubscription";
import kale_prediction from "../contracts/kale_prediction.ts";

/** Minimal in-memory round model used by the UI. */
export type Side = "HIGHER" | "LOWER";

export interface Round {
  id: number;
  predicted: number;
  deadline: number;
  finality: number;
  high_pool: bigint;
  low_pool: bigint;
  resolved: boolean;
  winning_side?: Side;
  actual?: number;
}

/**
 * Keep a local <id, Round> map by listening to contract events.
 * Frontend state source-of-truth = events → no extra view needed on-chain.
 */
export const useKaleRounds = () => {
  const [rounds] = useState<Map<number, Round>>(new Map());

  // --- listen to StartRound ------------------------------------------------
  useSubscription(
    kale_prediction.options.contractId, // contract ID
    "start_round",
    (ev) => {
      console.log(ev.value);
    },
  );

  // --- listen to Bet -------------------------------------------------------
  useSubscription(kale_prediction.options.contractId, "bet", (ev) => {
    console.log(ev.value);
  });

  // --- listen to ResolveRound ---------------------------------------------
  useSubscription(kale_prediction.options.contractId, "resolve_round", (ev) => {
    console.log(ev.value);
  });

  return Array.from(rounds.values());
};
