import exp from "constants";
import { type } from "os";
import { Wallet, SecretNetworkClient, fromUtf8, getMissingCodeHashWarning } from "secretjs";
import { Tx } from "secretjs/dist/protobuf_stuff/cosmos/tx/v1beta1/tx";

const decodeBase64 = (str: string):string => Buffer.from(str, 'base64').toString('binary');
const encodeBase64 = (str: string):string => Buffer.from(str, 'binary').toString('base64');

export class ContractInfo {
    codeHash: string;
    contractAddress: string;
  
    constructor(codeHash: string, contractAddress: string) {
      this.codeHash = codeHash;
      this.contractAddress = contractAddress;
    };

    async setViewingKey(client: SecretNetworkClient, key: string) {

      const setKeyTx = await client.tx.compute.executeContract(
        {
          sender: client.address,
          contractAddress: this.contractAddress,
          codeHash: this.codeHash,
          msg: {
            set_viewing_key: {
              key,
            }
          },
        },
        { gasLimit: 1_000_000 }
      );

      return setKeyTx;
    }
  };

  export class CodeInfo {
    codeId: number;
    codeHash: string;

    constructor(codeId: number, codeHash: string) {
      this.codeHash = codeHash;
      this.codeId = codeId;
    };
  };

  export enum Filter {
    Active = "active",
    Inactive = "inactive",
    All = "all",
  }

  type OffspringInfo = {
    address: string,
    label: string,
  };

  type InactiveOffspringInfo = {
    label: string,
    address: string,
  };

  export type ListMyOffspring = {
    list_my_offspring?: {
      active?: OffspringInfo[],
      inactive?: InactiveOffspringInfo[]
    },
    not_found?: { kind: string }
  };

  export type ListActiveOffspring = {
    list_active_offspring?: {
      active: OffspringInfo[]
    }
    not_found?: { kind: string }
  };

  export type ListInactiveOffspring = {
    list_inactive_offspring?: {
      inactive: InactiveOffspringInfo[]
    }
    not_found?: { kind: string }
  };