import { CasperClient, CLPublicKey } from "casper-js-sdk";
import _ from "lodash";

export const sleep = (ms: number) => {
  return new Promise((resolve) => setTimeout(resolve, ms));
};

export const getDeploy = async (NODE_URL: string, deployHash: string) => {
  const client = new CasperClient(NODE_URL);
  let i = 300;
  while (i != 0) {
    const [deploy, raw] = await client.getDeploy(deployHash);
    if (raw.execution_results.length !== 0) {
      // @ts-ignore
      if (raw.execution_results[0].result.Success) {
        return deploy;
      } else {
        // @ts-ignore
        throw Error(
          "Contract execution: " +
            // @ts-ignore
            raw.execution_results[0].result.Failure.error_message
        );
      }
    } else {
      i--;
      await sleep(1000);
      continue;
    }
  }
  throw Error("Timeout after " + i + "s. Something's wrong");
};

export const getAccountInfo = async (
  client: CasperClient,
  publicKey: CLPublicKey
) => {
  const accountHash = publicKey.toAccountHashStr();
  const stateRootHash = await client.nodeClient.getStateRootHash();
  const { Account: accountInfo } = await client.nodeClient.getBlockState(
    stateRootHash,
    accountHash,
    []
  );
  if (accountInfo === undefined) throw Error("Not found user");
  return accountInfo;
};

export const getAccountNamedKeyValue = async (
  client: CasperClient,
  publicKey: CLPublicKey,
  namedKey: string
) => {
  // Chain query: get account information.
  const accountInfo = await getAccountInfo(client, publicKey);
  const nameKey = _.find(accountInfo.namedKeys, (i) => {
    return i.name === namedKey;
  });
  if (nameKey === undefined) throw Error("Not found namedKey");
  return nameKey.key;
};
