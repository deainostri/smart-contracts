#!/bin/bash

erdpy config set chainID 1
erdpy config set proxy https://gateway.elrond.com

erdpy --verbose contract upgrade xxx \
    --project="sc-deainostri-nft-staking" \
    --metadata-payable \
    --pem="./wallets/deainostri.pem" \
    --gas-limit=46000000 \
    --proxy="https://gateway.elrond.com" \
    --recall-nonce \
    --send 