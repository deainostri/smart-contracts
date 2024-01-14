#!/bin/bash

erdpy config set chainID D
erdpy config set proxy https://devnet-gateway.elrond.com

erdpy --verbose contract deploy \
    --project="sc-deainostri-nft-staking" \
    --metadata-payable \
    --pem="./wallets/walletKey-deainostri.pem" \
    --gas-limit=30000000 \
    --proxy="https://devnet-gateway.elrond.com" \
    --recall-nonce \
    --send
