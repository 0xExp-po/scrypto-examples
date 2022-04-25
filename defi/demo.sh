#!/bin/bash

set -x
set -e

cd "$(dirname "$0")"

# cargo install --path ../../simulator

# reset database
resim reset

# new account
out=`resim new-account | tee /dev/tty | awk '/Account component address:|Public key:|Private key:/ {print $NF}'`
acc1_address=`echo $out | cut -d " " -f1`
acc1_pub_key=`echo $out | cut -d " " -f2`
acc1_priv_key=`echo $out | cut -d " " -f3`
out=`resim new-account | tee /dev/tty | awk '/Account component address:|Public key:|Private key:/ {print $NF}'`
acc2_address=`echo $out | cut -d " " -f1`
acc2_pub_key=`echo $out | cut -d " " -f2`
acc2_priv_key=`echo $out | cut -d " " -f3`
out=`resim new-account | tee /dev/tty | awk '/Account component address:|Public key:|Private key:/ {print $NF}'`
acc3_address=`echo $out | cut -d " " -f1`
acc3_pub_key=`echo $out | cut -d " " -f2`
acc3_priv_key=`echo $out | cut -d " " -f3`
out=`resim new-account | tee /dev/tty | awk '/Account component address:|Public key:|Private key:/ {print $NF}'`
acc4_address=`echo $out | cut -d " " -f1`
acc4_pub_key=`echo $out | cut -d " " -f2`
acc4_priv_key=`echo $out | cut -d " " -f3`

# generate acc1_mint_badge token
acc1_mint_badge=`resim new-badge-fixed --name MintBadge 1 | tee /dev/tty | awk '/Resource:/ {print $NF}'`

# mint btc
btc=`resim new-token-mutable $acc1_mint_badge --name Bitcoin --symbol BTC --description "Bitcoin is a decentralized digital currency, without a central bank or single administrator, that can be sent from user to user on the peer-to-peer bitcoin network without the need for intermediaries." | tee /dev/tty | awk '/Resource:/ {print $NF}'`
resim mint 18843462 $btc $acc1_mint_badge

# mint ethereum
eth=`resim new-token-mutable $acc1_mint_badge --name Ethereum --symbol ETH --description "Ethereum is a decentralized, open-source blockchain with smart contract functionality." | tee /dev/tty | awk '/Resource:/ {print $NF}'`
resim mint 117921786 $eth $acc1_mint_badge

# mint USD
usd=`resim new-token-mutable $acc1_mint_badge --name "US Dollar" --symbol USD --description "The United States dollar is the official currency of the United States and its territories." | tee /dev/tty | awk '/Resource:/ {print $NF}'`
resim mint 100000000000 $usd $acc1_mint_badge

# mint GBP
gbp=`resim new-token-mutable $acc1_mint_badge --name "Pound sterling" --symbol GBP --description "The pound sterling, known in some contexts simply as the pound or sterling, is the official currency of the United Kingdom, Jersey, Guernsey, the Isle of Man, Gibraltar, South Georgia and the South Sandwich Islands, the British Antarctic Territory, and Tristan da Cunha." | tee /dev/tty | awk '/Resource:/ {print $NF}'`
resim mint 100000000000 $gbp $acc1_mint_badge

# mint XRD
xrd=`resim new-token-mutable $acc1_mint_badge --name "Radix Token" --symbol XRD | tee /dev/tty | awk '/Resource:/ {print $NF}'`
resim mint 100000000000 $xrd $acc1_mint_badge

# mint SNX
snx=`resim new-token-mutable $acc1_mint_badge --name "Synthetics Token" --symbol SNX --description "A token which is used in the synthetics component for collateral" | tee /dev/tty | awk '/Resource:/ {print $NF}'`
resim mint 114841533.01 $snx $acc1_mint_badge

# publish PriceOracle
price_oracle_package=`resim publish ./price-oracle | tee /dev/tty | awk '/Package:/ {print $NF}'`
out=`resim call-function $price_oracle_package PriceOracle instantiate_oracle 1 | tee /dev/tty | awk '/Component:|Resource:/ {print $NF}'`
price_oracle_component=`echo $out | cut -d " " -f1`
price_oracle_update_auth=`echo $out | cut -d " " -f2`

# publish Radiswap
radiswap_package="0107467fe140289bec61d5ec6be68d6c7adc88ea5528d90c28b9f5"
resim publish ./radiswap --package-address $radiswap_package

# publish AutoLend
auto_lend_package=`resim publish ./auto-lend | tee /dev/tty | awk '/Package:/ {print $NF}'`
auto_lend_component=`resim call-function $auto_lend_package AutoLend instantiate_autolend $xrd | tee /dev/tty | awk '/Component:/ {print $NF}'`

# publish Synthetics
synthetics_package=`resim publish ./synthetics | tee /dev/tty | awk '/Package:/ {print $NF}'`
synthetics_component=`resim call-function $synthetics_package SyntheticPool instantiate_pool $price_oracle_component $snx $usd 4 | tee /dev/tty | awk '/Component:/ {print $NF}'`

# publish xPerpFutures
perpetual_futures_package=`resim publish ./x-perp-futures | tee /dev/tty | awk '/Package:/ {print $NF}'`
perpetual_futures_component=`resim call-function $perpetual_futures_package ClearingHouse instantiate_clearing_house $usd 100 45 | tee /dev/tty | awk '/Component:/ {print $NF}'`

# Set up swap pools
xrd_snx_radiswap_component=`resim call-function $radiswap_package Radiswap instantiate_pool 1000000,$xrd 38271,$snx 1000000 LPT LPToken https://www.example.com/ 0.001 | tee /dev/tty | awk '/Component:/ {print $NF}'`

# Update price
echo "CALL_METHOD ComponentAddress(\"$acc1_address\") \"create_proof\" ResourceAddress(\"$price_oracle_update_auth\");" > tx.rtm
echo "CALL_METHOD ComponentAddress(\"$price_oracle_component\") \"update_price\" ResourceAddress(\"$btc\") ResourceAddress(\"$usd\") Decimal(\"57523\");" >> tx.rtm
echo "CALL_METHOD ComponentAddress(\"$price_oracle_component\") \"update_price\" ResourceAddress(\"$eth\") ResourceAddress(\"$usd\") Decimal(\"3763\");" >> tx.rtm
echo "CALL_METHOD ComponentAddress(\"$price_oracle_component\") \"update_price\" ResourceAddress(\"$btc\") ResourceAddress(\"$gbp\") Decimal(\"41950\");" >> tx.rtm
echo "CALL_METHOD ComponentAddress(\"$price_oracle_component\") \"update_price\" ResourceAddress(\"$eth\") ResourceAddress(\"$gbp\") Decimal(\"2746\");" >> tx.rtm
echo "CALL_METHOD ComponentAddress(\"$price_oracle_component\") \"update_price\" ResourceAddress(\"$btc\") ResourceAddress(\"$eth\") Decimal(\"15\");" >> tx.rtm
echo "CALL_METHOD ComponentAddress(\"$price_oracle_component\") \"update_price\" ResourceAddress(\"$xrd\") ResourceAddress(\"$usd\") Decimal(\"0.4\");" >> tx.rtm
echo "CALL_METHOD ComponentAddress(\"$price_oracle_component\") \"update_price\" ResourceAddress(\"$snx\") ResourceAddress(\"$usd\") Decimal(\"10.40\");" >> tx.rtm
echo "CALL_METHOD ComponentAddress(\"$price_oracle_component\") \"update_price\" ResourceAddress(\"$btc\") ResourceAddress(\"$usd\") Decimal(\"66050.98\");" >> tx.rtm
resim run tx.rtm
rm tx.rtm
resim call-method $price_oracle_component get_price $btc $eth
resim call-method $price_oracle_component get_price $eth $btc

# Summary
set +x
echo "===================================================================================="
echo "Please assume a fixed number of decimal places for all resources: 18"
echo "Account 1 address: $acc1_address"
echo "Account 1 public key: $acc1_pub_key"
echo "Account 1 mint auth: $acc1_mint_badge"
echo "Account 2 address: $acc2_address"
echo "Account 2 public key: $acc2_pub_key"
echo "Account 3 address: $acc3_address"
echo "Account 3 public key: $acc3_pub_key"
echo "Account 4 address: $acc4_address"
echo "Account 4 public key: $acc4_pub_key"
echo "BTC: $btc"
echo "ETH: $eth"
echo "USD: $usd"
echo "GBP: $gbp"
echo "XRD: $xrd"
echo "SNX: $snx"
echo "Price Oracle blueprint: $price_oracle_package PriceOracle"
echo "Price Oracle component: $price_oracle_component"
echo "Price Oracle update auth: $price_oracle_update_auth"
echo "Radixswap blueprint: $radiswap_package Radiswap"
echo "AutoLend blueprint: $auto_lend_package AutoLend"
echo "AutoLend component: $auto_lend_component"
echo "Synthetics blueprint: $synthetics_package SyntheticPool"
echo "Synthetics component: $synthetics_component"
echo "xPerpFutures blueprint: $perpetual_futures_package ClearingHouse"
echo "xPerpFutures component: $perpetual_futures_component"
echo "XRD/SNX swap: $xrd_snx_radiswap_component"
echo "===================================================================================="
set -x

#====================
# Test mutual farm
#====================

# Create TESLA resource with no supply
tesla=`resim new-token-fixed --name "Tesla Token" --symbol "TESLA" 0 | tee /dev/tty | awk '/Resource:/ {print $NF}'`
echo "CALL_METHOD ComponentAddress(\"$acc1_address\") \"create_proof\" ResourceAddress(\"$price_oracle_update_auth\");" > tx.rtm
echo "CALL_METHOD ComponentAddress(\"$price_oracle_component\") \"update_price\" ResourceAddress(\"$tesla\") ResourceAddress(\"$usd\") Decimal(\"1162.00\");" >> tx.rtm
echo "CALL_METHOD ComponentAddress(\"$price_oracle_component\") \"update_price\" ResourceAddress(\"$xrd\") ResourceAddress(\"$snx\") Decimal(\"0.03901819\");" >> tx.rtm
resim run tx.rtm
rm tx.rtm

# Publish mutual farm package
mutual_farm_package=`resim publish mutual-farm | tee /dev/tty | awk '/Package:/ {print $NF}'`

# Instantiate mutual farm
out=`resim call-function $mutual_farm_package MutualFarm instantiate_farm $price_oracle_component $xrd_snx_radiswap_component $synthetics_component "TESLA" $tesla 1000 1000000,$xrd $snx $usd | tee /dev/tty | awk '/Component:/ {print $NF}'`
mutual_farm_component=`echo $out | cut -d " " -f1`
resim show $mutual_farm_component
resim show $acc1_address

# Deposit another 1,000,000 XRD
resim call-method $mutual_farm_component deposit 1000000,$xrd
resim show $acc1_address