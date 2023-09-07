import { RadixDappToolkit, DataRequestBuilder } from '@radixdlt/radix-dapp-toolkit'
// You can create a dApp definition in the dashboard at https://rcnet-v3-dashboard.radixdlt.com/dapp-metadata 
// then use that account for your dAppId
const dAppId = 'account_tdx_e_128y6f7ysmlvmn73zfjkjvrlvhqjac3gslep5xlaamg09pmcgdtrt7y'
// Instantiate DappToolkit
const rdt = RadixDappToolkit({
  dAppDefinitionAddress: dAppId,
  networkId: 14,
})
console.log("dApp Toolkit: ", rdt)

// ************ Fetch the user's account address ************
rdt.walletApi.setRequestData(DataRequestBuilder.accounts().atLeast(1))
// Subscribe to updates to the user's shared wallet data
rdt.walletApi.walletData$.subscribe((walletData) => {
  console.log("subscription wallet data: ", walletData)
  document.getElementById('accountName').innerText = walletData.accounts[0].label
  document.getElementById('accountAddress').innerText = walletData.accounts[0].address
  accountAddress = walletData.accounts[0].address
})


// Global states
let accountAddress // User account address
let componentAddress = "component_tdx_e_1crt0ndkk0x7rexswhczacj2405tgzh0zv3t0fr7czdxnd8y8vgjuyw" //GumballMachine component address
let gum_resourceAddress = "resource_tdx_e_1t5hrjudhk4yd5m4svujx5ytvtvjaj5ufuahxaauru72ed7vrce8q34" // RCV3 GUM resource address
let xrdAddress = "resource_tdx_e_1tknxxxxxxxxxradxrdxxxxxxxxx009923554798xxxxxxxxx8rpsmc" //RCnet v3 XRD resource address
// You receive this badge(your resource address will be different) when you instantiate the component
let admin_badge = "resource_tdx_e_1t5kmfczjh3g5vs7gj9jcarmgt0u29qfj50kngxqv2cnny9rr89fd94"
let owner_badge = "resource_tdx_e_1tk35c2pnzy4fr6ms5wpm8v4rsw76h7ptyydvqgs88jk58eqmaeqt45"
// You can use these addresses to skip package deployment steps
// RCNet v3.1 package_address = package_tdx_e_1p5xrp5rasany9nfa5ssp8skmhx4c4v2zlwmjnn7fu29yxhvhhra6l6


// ************ Instantiate component and fetch component and resource addresses *************
document.getElementById('instantiateComponent').onclick = async function () {
  let packageAddress = document.getElementById("packageAddress").value;
  let flavor = document.getElementById("flavor").value;
  let manifest = `
  CALL_FUNCTION
    Address("${packageAddress}")
    "GumballMachine"
    "instantiate_gumball_machine"
    Decimal("5")
    "${flavor}";
  CALL_METHOD
    Address("${accountAddress}")
    "deposit_batch"
    Expression("ENTIRE_WORKTOP");
    `
  console.log("Instantiate Manifest: ", manifest)

  // Send manifest to extension for signing
  const result = await rdt.walletApi
    .sendTransaction({
      transactionManifest: manifest,
      version: 1,
    })
  if (result.isErr()) throw result.error
  console.log("Intantiate WalletSDK Result: ", result.value)


  // ************ Fetch the transaction status from the Gateway API ************
  let transactionStatus = await rdt.gatewayApi.transaction.getStatus(result.value.transactionIntentHash)
  console.log('Instantiate TransactionApi transaction/status:', transactionStatus)


  // ************ Fetch component address from gateway api and set componentAddress variable **************
  let getCommitReceipt = await rdt.gatewayApi.transaction.getCommittedDetails(result.value.transactionIntentHash)
  console.log('Instantiate getCommittedDetails:', getCommitReceipt)

  // ****** Set componentAddress variable with gateway api getCommitReciept payload ******
  componentAddress = getCommitReceipt.transaction.affected_global_entities[2];
  document.getElementById('componentAddress').innerText = componentAddress;

  // ****** Set admin_badge variable with gateway api getCommitReciept payload ******
  admin_badge = getCommitReceipt.transaction.affected_global_entities[4];
  document.getElementById('admin_badge').innerText = admin_badge;

  // ****** Set owner_badge variable with gateway api getCommitReciept payload ******
  owner_badge = getCommitReceipt.transaction.affected_global_entities[3];
  document.getElementById('owner_badge').innerText = owner_badge;

  // ****** Set gum_resourceAddress variable with gateway api getCommitReciept payload ******
  gum_resourceAddress = getCommitReceipt.transaction.affected_global_entities[6];
  document.getElementById('gum_resourceAddress').innerText = gum_resourceAddress;
}


// *********** Buy Gumball ***********
document.getElementById('buyGumball').onclick = async function () {
  let manifest = `
  CALL_METHOD
    Address("${accountAddress}")
    "withdraw"    
    Address("${xrdAddress}")
    Decimal("33");
  TAKE_ALL_FROM_WORKTOP
    Address("${xrdAddress}")
    Bucket("xrd");
  CALL_METHOD
    Address("${componentAddress}")
    "buy_gumball"
    Bucket("xrd");
  CALL_METHOD
    Address("${accountAddress}")
    "deposit_batch"
    Expression("ENTIRE_WORKTOP");
    `
  console.log('buy_gumball manifest: ', manifest)

  // Send manifest to extension for signing
  const result = await rdt.walletApi
    .sendTransaction({
      transactionManifest: manifest,
      version: 1,
    })
  if (result.isErr()) throw result.error
  console.log("Buy Gumball sendTransaction Result: ", result.value)

  // Fetch the transaction status from the Gateway SDK
  let transactionStatus = await rdt.gatewayApi.transaction.getStatus(result.value.transactionIntentHash)
  console.log('Buy Gumball TransactionAPI transaction/status: ', transactionStatus)

  // fetch commit reciept from gateway api 
  let getCommitReceipt = await rdt.gatewayApi.transaction.getCommittedDetails(result.value.transactionIntentHash)
  console.log('Buy Gumball Committed Details Receipt', getCommitReceipt)

  // Show the receipt in the DOM
  document.getElementById('receipt').innerText = JSON.stringify(getCommitReceipt);
}


// *********** Get Price ***********
document.getElementById('getPrice').onclick = async function () {
  // Use gateway state api to fetch component details including price field
  let getPrice = await rdt.gatewayApi.state.getEntityDetailsVaultAggregated(componentAddress)
  console.log('getPrice', getPrice)

  // Show the price in the DOM
  document.getElementById('price').innerText = JSON.stringify(getPrice.details.state.fields[2].value);
}


// *********** Set Price ***********
document.getElementById('setPrice').onclick = async function () {
  let newPrice = document.getElementById('newPrice').value
  let manifest = `
  CALL_METHOD
    Address("${accountAddress}")
    "create_proof_of_amount"    
    Address("${admin_badge}")
    Decimal("1");
CALL_METHOD
    Address("${componentAddress}")
    "set_price"
    Decimal("${newPrice}");
  `
  console.log("Set Price manifest", manifest)

  // Send manifest to extension for signing
  const result = await rdt.walletApi
    .sendTransaction({
      transactionManifest: manifest,
      version: 1,
    })
  if (result.isErr()) throw result.error
  console.log("Set Price sendTransaction result: ", result.value)

  // Fetch the transaction status from the Gateway SDK
  let transactionStatus = await rdt.gatewayApi.transaction.getStatus(result.value.transactionIntentHash)
  console.log('Set Price transaction status', transactionStatus)
  let getPrice = await rdt.gatewayApi.state.getEntityDetailsVaultAggregated(componentAddress)
  console.log('Set Price new value', getPrice)

  // Show the New Price in the DOM
  document.getElementById('price').innerText = JSON.stringify(getPrice.details.state.fields[2].value);
}


// *********** Withdraw Earnings ***********
document.getElementById('withdrawEarnings').onclick = async function () {
  // TODO Replace with String Manifest
  let manifest = `
  CALL_METHOD
    Address("${accountAddress}")
    "create_proof_of_amount"    
    Address("${owner_badge}")
    Decimal("1");
  CALL_METHOD
    Address("${componentAddress}")
    "withdraw_earnings";
  CALL_METHOD
    Address("${accountAddress}")
    "deposit_batch"
    Expression("ENTIRE_WORKTOP");
    `
  console.log("Withdraw Earnings manifest", manifest)

  // Send manifest to extension for signing
  const result = await rdt.walletApi
    .sendTransaction({
      transactionManifest: manifest,
      version: 1,
    })
  if (result.isErr()) throw result.error
  console.log("Withdraw Earnings sendTransaction Result: ", result.value)

  // Fetch the transaction status from the Gateway SDK
  let transactionStatus = await rdt.gatewayApi.transaction.getStatus(result.value.transactionIntentHash)
  console.log('Withdraw Earnings status', transactionStatus)

  // fetch commit reciept from gateway api 
  let getCommitReceipt = await rdt.gatewayApi.transaction.getCommittedDetails(result.value.transactionIntentHash)
  console.log('Withdraw Earnings commitReceipt', getCommitReceipt)

  // Show the receipt on the DOM
  document.getElementById('withdraw').innerText = JSON.stringify(getCommitReceipt);
}
// *********** Mint NFT Staff Badge ***********
document.getElementById('mintStaffBadge').onclick = async function () {
  // TODO Replace with String Manifest
  let manifest = `
  CALL_METHOD
    Address("${accountAddress}")
    "create_proof_of_amount"    
    Address("${admin_badge}")
    Decimal("1");
CALL_METHOD
    Address("${componentAddress}")
    "mint_staff_badge"
    "Number2";
CALL_METHOD
    Address("${accountAddress}")
    "deposit_batch"
    Expression("ENTIRE_WORKTOP");
    `
  console.log("mintStaffBadge manifest", manifest)

  // Send manifest to extension for signing
  const result = await rdt.walletApi
    .sendTransaction({
      transactionManifest: manifest,
      version: 1,
    })
  if (result.isErr()) throw result.error

  console.log("mintStaffBadge sendTransaction Result: ", result.value)

  // Fetch the transaction status from the Gateway SDK
  let transactionStatus = await rdt.gatewayApi.transaction.getStatus(result.value.transactionIntentHash)
  console.log('mintStaffBadge status', transactionStatus)

  // fetch commit reciept from gateway api 
  let getCommitReceipt = await rdt.gatewayApi.transaction.getCommittedDetails(result.value.transactionIntentHash)
  console.log('mintStaffBadge commitReceipt', getCommitReceipt)

  // Show the receipt on the DOM
  document.getElementById('staffBadge').innerText = JSON.stringify(getCommitReceipt);
}