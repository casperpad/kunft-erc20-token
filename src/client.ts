import {
  CLKey,
  CLKeyParameters,
  CLPublicKey,
  CLValueBuilder,
  Contracts,
  Keys,
  RuntimeArgs,
} from "casper-js-sdk";
import { BigNumberish } from "@ethersproject/bignumber";
const { Contract } = Contracts;

export class MintableERC20Client extends Contract {
  mint(
    owner: CLKeyParameters,
    amount: BigNumberish,
    sender: CLPublicKey,
    paymentAmount: BigNumberish,
    chainName: string,
    signingKeys?: Keys.AsymmetricKey[]
  ) {
    const args = RuntimeArgs.fromMap({
      owner: new CLKey(owner),
      amount: CLValueBuilder.u256(amount),
    });
    return this.callEntrypoint(
      "mint",
      args,
      sender,
      chainName,
      paymentAmount.toString(),
      signingKeys
    );
  }
}
