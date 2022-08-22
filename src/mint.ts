import { config } from "dotenv";
config({ path: ".env" });

import { formatFixed } from "@ethersproject/bignumber";
import { ERC20Client } from "casper-erc20-js-client";
import { getAccountNamedKeyValue, getDeploy } from "./utils";

import { CasperClient, CLPublicKey, Keys } from "casper-js-sdk";
import { MintableERC20Client } from "./client";

const {
  NODE_ADDRESS,
  CHAIN_NAME,
  MASTER_KEY_PAIR_PATH,
  TOKEN_NAME,
  TOKEN_SUPPLY,
} = process.env;

const KEYS = Keys.Ed25519.loadKeyPairFromPrivateFile(MASTER_KEY_PAIR_PATH!);

const mint = async () => {
  const casperClient = new CasperClient(NODE_ADDRESS!);
  const mintableERC20Client = new MintableERC20Client(casperClient);
  const contractHash = await getAccountNamedKeyValue(
    casperClient,
    KEYS.publicKey,
    `${TOKEN_NAME}_contract_hash`
  );
  mintableERC20Client.setContractHash(contractHash);

  const owner = CLPublicKey.fromHex(
    "015b1b98c0293d90ec0bc7f0bae2c6ddc72b463e4ebad9c495c40f41bbeb2eba16"
  );

  const deploy = mintableERC20Client.mint(
    owner,
    TOKEN_SUPPLY!,
    KEYS.publicKey,
    "100000000",
    [KEYS]
  );

  const deployHash = await deploy.send(NODE_ADDRESS!);

  console.log({ deployHash });

  await getDeploy(NODE_ADDRESS!, deployHash);

  console.log(`...Minted successfully.`);
  const erc20 = new ERC20Client(NODE_ADDRESS!, CHAIN_NAME!);
  await erc20.setContractHash(contractHash.slice(5));
  const balance = await erc20.balanceOf(owner);
  const decimals = await erc20.decimals();
  const parsed = formatFixed(balance, decimals);
  console.log({ balance: parsed });
};

mint();
