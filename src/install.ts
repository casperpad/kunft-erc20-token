import { config } from "dotenv";
// config({ path: ".env" });
config({ path: ".env.production.local" });

import { ERC20Client } from "casper-erc20-js-client";
import { utils } from "casper-js-client-helper";
import { getDeploy } from "./utils";

import { Keys, encodeBase16 } from "casper-js-sdk";

const {
  NODE_ADDRESS,
  CHAIN_NAME,
  ERC20_WASM_PATH,
  MASTER_KEY_PAIR_PATH,
  TOKEN_NAME,
  TOKEN_SYMBOL,
  TOKEN_DECIMALS,
  INSTALL_PAYMENT_AMOUNT,
} = process.env;

const KEYS = Keys.Ed25519.loadKeyPairFromPrivateFile(MASTER_KEY_PAIR_PATH!);

const deploy = async () => {
  const erc20 = new ERC20Client(NODE_ADDRESS!, CHAIN_NAME!);

  const installDeployHash = await erc20.install(
    KEYS,
    TOKEN_NAME!,
    TOKEN_SYMBOL!,
    TOKEN_DECIMALS!,
    "0",
    INSTALL_PAYMENT_AMOUNT!,
    ERC20_WASM_PATH!
  );

  console.log(`... Contract installation deployHash: ${installDeployHash}`);

  await getDeploy(NODE_ADDRESS!, installDeployHash);

  console.log(`... Contract installed successfully.`);

  const accountInfo = await utils.getAccountInfo(NODE_ADDRESS!, KEYS.publicKey);

  const contractHash = await utils.getAccountNamedKeyValue(
    accountInfo,
    `${TOKEN_NAME}_contract_hash`
  );

  console.log(`... Contract Hash: ${contractHash}`);
};

deploy();

// console.log(encodeBase16(KEYS.privateKey));
