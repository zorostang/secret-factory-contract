import axios from "axios";
import { Wallet, SecretNetworkClient, fromUtf8, MsgExecuteContract, Msg } from "secretjs";
import fs from "fs";
import assert from "assert";
import { ContractInfo, ListMyOffspring, Filter, ListActiveOffspring, ListInactiveOffspring, CodeInfo } from "./classes";

const wallet = new Wallet(
  "grant rice replace explain federal release fix clever romance raise often wild taxi quarter soccer fiber love must tape steak together observe swap guitar",
);

const myAddress = wallet.address;

const grpcUrl = "http://localhost:9091";
const chainId = "secretdev-1";

// some universal constants:

const viewing_key = "test_key";

// Returns a client with which we can interact with secret network
const initializeClient = async (wallet: Wallet) => {
  const walletAddr = wallet.address;
  const client = await SecretNetworkClient.create({
    // Create a client to interact with the network
    grpcWebUrl: grpcUrl,
    chainId: chainId,
    wallet: wallet,
    walletAddress: walletAddr,
  });

  //console.log(`Initialized client with wallet address: ${walletAddr}`);
  return client;
};

// Stores and instantiaties a new contract in our network
const uploadContract = async (
  client: SecretNetworkClient,
  wasmPath: string
) => {
  const wasmCode = fs.readFileSync(wasmPath);
  console.log("Uploading contract");

  const uploadReceipt = await client.tx.compute.storeCode(
    {
      wasmByteCode: wasmCode,
      sender: client.address,
      source: "",
      builder: "",
    },
    {
      gasLimit: 5_000_000,
    }
  );

  if (uploadReceipt.code !== 0) {
    console.log(
      `Failed to get code id: ${JSON.stringify(uploadReceipt.rawLog)}`
    );
    throw new Error(`Failed to upload contract`);
  }

  const codeIdKv = uploadReceipt.jsonLog![0].events[0].attributes.find(
    (a: any) => {
      return a.key === "code_id";
    }
  );

  const codeId = Number(codeIdKv!.value);
  console.log(`${wasmPath} contract codeId: `, codeId);

  const codeHash = await client.query.compute.codeHash(codeId);
  console.log(`Contract hash: ${codeHash}`);

  let codeInfo = new CodeInfo(codeId, codeHash);
  return codeInfo;
};

const getFromFaucet = async (address: string) => {
  await axios.get(`http://localhost:5000/faucet?address=${address}`);
};

async function getScrtBalance(userCli: SecretNetworkClient): Promise<string> {
  let balanceResponse = await userCli.query.bank.balance({
    address: userCli.address,
    denom: "uscrt",
  });
  return balanceResponse.balance!.amount;
}

async function fillUpFromFaucet(
  client: SecretNetworkClient,
  targetBalance: Number
) {
  let balance = await getScrtBalance(client);
  while (Number(balance) < targetBalance) {
    try {
      await getFromFaucet(client.address);
    } catch (e) {
      console.error(`failed to get tokens from faucet: ${e}`);
    }
    balance = await getScrtBalance(client);
  }
  console.error(`got tokens from faucet: ${balance}`);
}

// Initialization procedure
async function initializeAndUploadFactory() {

  const client = await initializeClient(wallet);

  // upload codes

  const factoryCodeInfo = await uploadContract(client, "./compiled_wasm/factory.wasm.gz");
  const offspringCodeInfo = await uploadContract(client, "./compiled_wasm/offspring.wasm.gz");

  // initialize contracts

  // 1. init factory

  const factoryInitArgs = {
    sender: myAddress,
    codeId: factoryCodeInfo.codeId,
    codeHash: factoryCodeInfo.codeHash,
    initMsg: {
      entropy: "rndm_word",
      offspring_contract: {
        code_id: offspringCodeInfo.codeId,
        code_hash: offspringCodeInfo.codeHash
      },
    },
    label: "Test Factory Contract"
  };

  const factoryInitTx = await client.tx.compute.instantiateContract(factoryInitArgs, { gasLimit: 1_000_000 });

  const factoryAddress = factoryInitTx.arrayLog!.find(
    (log) => log.type === "message" && log.key === "contract_address",
  )!.value;

  console.log(`Factory Contract Address: ${factoryAddress}`);

  const factoryContractInfo = new ContractInfo(factoryCodeInfo.codeHash, factoryAddress);

  await factoryContractInfo.setViewingKey(client, viewing_key);

  let initInfo: [CodeInfo, CodeInfo, ContractInfo] = [factoryCodeInfo, offspringCodeInfo, factoryContractInfo]

  return initInfo;
}

async function createOffspring(
  factory: ContractInfo,
  label: string,
  entropy: string,
  owner: string,
  count: number,
) {

  const client = await initializeClient(wallet);

  const createOffspringMsg = {
    create_offspring: {
      label,
      entropy,
      owner,
      count,
    }
  };

  const createOffspringTx = await client.tx.compute.executeContract(
    {
      sender: client.address,
      contractAddress: factory.contractAddress,
      codeHash: factory.codeHash,
      msg: createOffspringMsg,
    },
    { gasLimit: 1_000_000 }
  );

  // console.log(createOffspringTx);

  const offspringAddress: string = createOffspringTx.arrayLog!.find(
    (a: any) => {
      return a.key === "offspring_address";
    }
  )!.value;

  console.log("Created offspring contract, at the address: ", offspringAddress);

  return offspringAddress;
}

async function createOffsprings(
  factory: ContractInfo,
  label: string,
  entropy: string,
  owner: string,
  count: number,
  number?: number,
) {
  let num = (number == undefined) ? 1 : number;

  const client = await initializeClient(wallet);

  var execMsgs: Msg[] = [];

  for (let index = 0; index < num; index++) {
    let createOffspringMsg = {
      create_offspring: {
        label: label + index,
        entropy,
        owner,
        count,
      }
    };
    let execMsg = new MsgExecuteContract({
      sender: client.address,
      contractAddress: factory.contractAddress,
      codeHash: factory.codeHash,
      msg: createOffspringMsg,
    });
    execMsgs.push(execMsg);
  }

  //console.log("List of messages: ", execMsgs);

  const createOffspringsTx = await client.tx.broadcast(execMsgs, {gasLimit: 2_000_000});

  return createOffspringsTx;
}

async function queryMyOffsprings(
  factoryInfo: ContractInfo,
  address: string,
  viewing_key: string,
  filter?: Filter,
  start_page?: number,
  page_size?: number,
  ) {
  const client = await initializeClient(wallet);

  let filterDefined = !(filter == undefined);
  let startPageDefined = !(start_page == undefined);
  let pageSizeDefined = !(page_size == undefined);

  const myOffspringMsg = {
    list_my_offspring: {
      address,
      viewing_key,
      ...(filterDefined && { filter }),
      ...(startPageDefined && { start_page }),
      ...(pageSizeDefined && { page_size })
    },
  };

  const queryTx = (await client.query.compute.queryContract(
    {
      contractAddress: factoryInfo.contractAddress,
      codeHash: factoryInfo.codeHash,
      query: myOffspringMsg,
    }
  )) as ListMyOffspring;

  if ('err"' in queryTx) {
    throw new Error(
      `My offspring query failed with the following err: ${JSON.stringify(queryTx)}`
    );
  }

  return queryTx;
}

async function queryActiveOffsprings(
  factoryInfo: ContractInfo,
  start_page?: number,
  page_size?: number,
  ) {
  const client = await initializeClient(wallet);

  let startPageDefined = !(start_page == undefined);
  let pageSizeDefined = !(page_size == undefined);

  const activeOffspringMsg = {
    list_active_offspring: {
      ...(startPageDefined && { start_page }),
      ...(pageSizeDefined && { page_size })
    },
  };

  const queryTx = (await client.query.compute.queryContract(
    {
      contractAddress: factoryInfo.contractAddress,
      codeHash: factoryInfo.codeHash,
      query: activeOffspringMsg,
    }
  )) as ListActiveOffspring;

  if ('err"' in queryTx) {
    throw new Error(
      `My offspring query failed with the following err: ${JSON.stringify(queryTx)}`
    );
  }

  return queryTx;
}

async function queryInactiveOffsprings(
  factoryInfo: ContractInfo,
  start_page?: number,
  page_size?: number,
  ) {
  const client = await initializeClient(wallet);

  let startPageDefined = !(start_page == undefined);
  let pageSizeDefined = !(page_size == undefined);

  const inactiveOffspringMsg = {
    list_inactive_offspring: {
      ...(startPageDefined && { start_page }),
      ...(pageSizeDefined && { page_size })
    },
  };

  const queryTx = (await client.query.compute.queryContract(
    {
      contractAddress: factoryInfo.contractAddress,
      codeHash: factoryInfo.codeHash,
      query: inactiveOffspringMsg,
    }
  )) as ListInactiveOffspring;

  if ('err"' in queryTx) {
    throw new Error(
      `My offspring query failed with the following err: ${JSON.stringify(queryTx)}`
    );
  }

  return queryTx;
}

async function deactivateOffspring(
  offspringInfo: ContractInfo,
  wallet: Wallet,
  ) {
  const client = await initializeClient(wallet);

  const deactivateMsg = {
    deactivate: {},
  };

  const deactivateTx = await client.tx.compute.executeContract(
    {
      sender: client.address,
      contractAddress: offspringInfo.contractAddress,
      codeHash: offspringInfo.codeHash,
      msg: deactivateMsg,
    },
    { gasLimit: 1_000_000 }
  );
  
  return deactivateTx;
}

async function deactivateOffsprings(
  offspringInfoList: ContractInfo[],
  wallet: Wallet,
  ) {
  const client = await initializeClient(wallet);

  const deactivateMsg = {
    deactivate: {},
  };

  var execMsgs: Msg[] = [];

  offspringInfoList.forEach(info => {
    let execMsg = new MsgExecuteContract({
      sender: client.address,
      contractAddress: info.contractAddress,
      codeHash: info.codeHash,
      msg: deactivateMsg,
    });
    execMsgs.push(execMsg);
  });

  const deactivateTx = await client.tx.broadcast(
    execMsgs,
    { gasLimit: 1_000_000 }
  );
  
  return deactivateTx;
}

(async () => {
  const [factoryCodeInfo, offspringCodeInfo, factoryContractInfo] = await initializeAndUploadFactory();

  const queryMy = await queryMyOffsprings(factoryContractInfo, myAddress, viewing_key);
  const queryActive = await queryActiveOffsprings(factoryContractInfo);
  const queryInactive = await queryInactiveOffsprings(factoryContractInfo);

  console.log("List my offspring: ", queryMy);
  console.log("List active offspring: ", queryActive);
  console.log("List inactive offspring: ", queryInactive);

  const test = await createOffsprings(factoryContractInfo, "offspring", "entropy", myAddress, 2, 20);

  //for (let index = 0; index < 7; index++) {
  //  await deactivateOffspring(offspringInfoList[index*3], wallet);
  //}

  const queryActivePage = await queryActiveOffsprings(factoryContractInfo);
  const offspringList = queryActivePage.list_active_offspring!.active;

  // deactivate multiples of 3
  let deactivateList: ContractInfo[] = [];
  for (let index = 0; index < (offspringList.length/3); index++) {
    let idx = index*3;
    let offspringInfo = offspringList[idx];
    let offspringAddr = offspringInfo.address;
    let contractInfo = new ContractInfo(offspringCodeInfo.codeHash, offspringAddr);
    deactivateList.push(contractInfo);
  }
  await deactivateOffsprings(deactivateList, wallet);

  console.log("Active offspring listed in pages:");
  for (let index = 0; index < 5; index++) {
    const queryActivePage = await queryActiveOffsprings(factoryContractInfo, index, 5);
    let display = (queryActivePage.not_found == undefined) ? queryActivePage.list_active_offspring?.active : queryActivePage;
    console.log(display);
  }

  const queryMyAll = await queryMyOffsprings(factoryContractInfo, myAddress, viewing_key, Filter.All);
  
  const queryInactiveAll = await queryInactiveOffsprings(factoryContractInfo);

  console.log("List my offspring: ", queryMyAll.list_my_offspring);
  console.log("List inactive offspring: ", queryInactiveAll.list_inactive_offspring?.inactive);

})();
