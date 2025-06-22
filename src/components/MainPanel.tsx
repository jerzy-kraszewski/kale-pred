import { useEffect, useState } from "react";
import kale_prediction from "../contracts/kale_prediction";
import { useWallet } from "../hooks/useWallet";
import AdminPanel from "./kale/AdminPanel";
import UserPanel from "./kale/UserPanel";

export default function MainPanel() {
  const { address } = useWallet();
  const [admin, setAdmin] = useState("");
  useEffect(() => {
    kale_prediction
      .get_admin()
      .then((result) => setAdmin(result.result))
      .catch((reason) => console.log(reason));
  });
  return <div>{address == admin ? <AdminPanel /> : <UserPanel />}</div>;
}
