import { Button } from "@stellar/design-system";
import kale_prediction from "../../contracts/kale_prediction";
import { useWallet } from "../../hooks/useWallet";
import { wallet } from "../../util/wallet";

export default function CreateRound() {
  const { address, networkPassphrase } = useWallet();
  const createRound = () => {
    if (!address) {
      return;
    }
    kale_prediction
      .start_round(
        {
          admin: address,
          predicted_count: 0,
          deadline_ledger: 10,
          finality_ledger: 20,
        },
        { simulate: true },
      )
      .then((tx) => {
        console.log(tx.toXDR());
        tx.signAndSend({
          signTransaction: (xdr) =>
            wallet.signTransaction(xdr, { networkPassphrase, address }),
        })
          .then((sentTx) => {
            console.log(sentTx.sendTransactionResponse?.hash);
          })
          .catch((reason) =>
            console.log(`sign and send failed with reason: ${reason}`),
          );
      })
      .catch((reason) =>
        console.log(`assembling tx failed with reason: ${reason}`),
      );
  };

  return (
    <div>
      <Button variant="secondary" size="md" onClick={() => createRound()}>
        Create round
      </Button>
    </div>
  );
}
