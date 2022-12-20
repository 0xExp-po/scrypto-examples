import WalletSdk, {
  requestBuilder,
  requestItem,
  ManifestBuilder,
  ResourceAddress,
  Bucket,
  Expression,
  Decimal,
} from '@radixdlt/wallet-sdk';
import { TransactionApi, StateApi, StatusApi, StreamApi } from "@radixdlt/babylon-gateway-api-sdk";

const transactionApi = new TransactionApi();
const stateApi = new StateApi();
const statusApi = new StatusApi();
const streamApi = new StreamApi();

const walletSdk = WalletSdk({ dAppId: 'Gumball', networkId: 0x0b })
console.log("walletSdk: ", walletSdk)

// Global states
let accountAddress: string // User account address
let componentAddress: string  // GumballMachine component address
let resourceAddress: string // GUM resource address
// package address package_tdx_b_1qyjhg0d0nvx48xuss8dnykpa0er8kfmvpql6v6awepnqhssqnf

// Fetch list of account Addresses on button click
document.getElementById('fetchAccountAddress').onclick = async function () {
  // Retrieve extension user account addresses
  console.log('getting account info')
  const result = await walletSdk.request(
    // the number passed as arg is the max number of addresses you wish to fetch
    requestBuilder(requestItem.oneTimeAccounts.withoutProofOfOwnership(1))
  )

  if (result.isErr()) {
    throw result.error
  }

  const { oneTimeAccounts } = result.value
  console.log("requestItem.oneTimeAccounts.withoutProofOfOwnership(1) ", result)
  if (!oneTimeAccounts) return

  document.getElementById('accountAddress').innerText = oneTimeAccounts[0].address
  accountAddress = oneTimeAccounts[0].address
}

// Instantiate component
document.getElementById('instantiateComponent').onclick = async function () {
  let packageAddress = document.getElementById("packageAddress").value;

  let manifest = new ManifestBuilder()
    .callFunction(packageAddress, "GumballMachine", "instantiate_gumball_machine", [Decimal("10")])
    .build()
    .toString();
  console.log("manifest: ", manifest)
  // Send manifest to extension for signing
  const result = await walletSdk
    .sendTransaction({
      transactionManifest: manifest,
      version: 1,
    })

  if (result.isErr()) throw result.error

  console.log("Intantiate Result: ", result.value)

  // Fetch the receipt from the Gateway API
  // const receipt = await transactionApi.transactionReceiptPost({
  //   v0CommittedTransactionRequest: { intent_hash: result.value },
  // })
  let response = await transactionApi.transactionStatus({
    transactionStatusRequest: {
      intent_hash_hex: result.value.transactionIntentHash
    }
  });
  console.log('response', response)

  // fetch component address from gateway api and set componentAddress variable 
  // componentAddress = receipt.committed.receipt.state_updates.new_global_entities[1].global_address
  // document.getElementById('componentAddress').innerText = componentAddress;

  // resourceAddress = receipt.committed.receipt.state_updates.new_global_entities[0].global_address
  // document.getElementById('gumAddress').innerText = resourceAddress;
}

document.getElementById('buyGumball').onclick = async function () {

  let manifest = new ManifestBuilder()
    .withdrawFromAccountByAmount(accountAddress, 10, "resource_tdx_b_1qzkcyv5dwq3r6kawy6pxpvcythx8rh8ntum6ws62p95s9hhz9x")
    .takeFromWorktopByAmount(10, "resource_tdx_b_1qzkcyv5dwq3r6kawy6pxpvcythx8rh8ntum6ws62p95s9hhz9x", "bucket1")
    .callMethod(componentAddress, "buy_gumball", ['Bucket("bucket1")'])
    .callMethod(accountAddress, "deposit_batch", ['Expression("ENTIRE_WORKTOP")'])
    .build()
    .toString();

  // Send manifest to extension for signing
  const result = await walletSdk
    .sendTransaction({
      transactionManifest: manifest,
      version: 1,
    })

  if (result.isErr()) throw result.error

  console.log("buyGumball result: ", result)

  // Fetch the receipt from the Gateway SDK
  // const receipt = await transactionApi.transactionReceiptPost({
  //   v0CommittedTransactionRequest: { intent_hash: hash.value },
  // })

  // Show the receipt on the DOM
  // document.getElementById('receipt').innerText = JSON.stringify(receipt.committed.receipt, null, 2);
};

// document.getElementById('checkBalance').onclick = async function () {

//   // Fetch the state of the account component
//   const account_state = await stateApi.stateComponentPost({
//     v0StateComponentRequest: { component_address: accountAddress }
//   })

//   let account_gum_vault = account_state.owned_vaults.find(vault => vault.resource_amount.resource_address == `${resourceAddress}`)

//   // Fetch the state of the machine component
//   const machine_state = await stateApi.stateComponentPost({
//     v0StateComponentRequest: { component_address: componentAddress }
//   })

//   let machine_gum_vault = machine_state.owned_vaults.find(vault => vault.resource_amount.resource_address == `${resourceAddress}`)

//   // Update the DOM
//   document.getElementById("userBalance").innerText = account_gum_vault.resource_amount.amount_attos / Math.pow(10, 18)
//   document.getElementById("machineBalance").innerText = machine_gum_vault.resource_amount.amount_attos / Math.pow(10, 18)
// };