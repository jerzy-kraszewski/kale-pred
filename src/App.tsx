import { Layout } from "@stellar/design-system";
import "./App.module.css";
import ConnectAccount from "./components/ConnectAccount.tsx";
import { useWallet } from "./hooks/useWallet.ts";
import MainPanel from "./components/MainPanel.tsx";

function App() {
  const { address } = useWallet();
  return (
    <main>
      <Layout.Header
        projectId="My App"
        projectTitle="My App"
        contentRight={<ConnectAccount />}
      />
      <Layout.Content>
        <Layout.Inset>
          <h1>Welcome to your app!</h1>
          {address ? <MainPanel /> : <h2>Please connect a wallet</h2>}
        </Layout.Inset>
      </Layout.Content>
      <Layout.Footer>
        <span>
          © {new Date().getFullYear()} My App. Licensed under the{" "}
          <a
            href="http://www.apache.org/licenses/LICENSE-2.0"
            target="_blank"
            rel="noopener noreferrer"
          >
            Apache License, Version 2.0
          </a>
          .
        </span>
      </Layout.Footer>
    </main>
  );
}

export default App;
