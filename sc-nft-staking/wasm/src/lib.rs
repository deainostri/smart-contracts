////////////////////////////////////////////////////
////////////////// AUTO-GENERATED //////////////////
////////////////////////////////////////////////////

#![no_std]

elrond_wasm_node::wasm_endpoints! {
    sc_deainostri_nft_staking
    (
        addStakeReward
        claim
        computePercentages
        computePoints
        computeTotalPoints
        fixStakeReward
        getClaimableByAddress
        getClaimedByAddress
        getComputeIndex
        getIsLocked
        getNewPointsByAddress
        getPointsByAddress
        getStakedAddresses
        getStakedNfts
        getTickInterval
        getTotalPoints
        lock
        resetComputeIndex
        resolveStakeReward
        setNftStartStakeDate
        setNftTokenId
        setTickInterval
        stake
        unlock
        unstake
    )
}

elrond_wasm_node::wasm_empty_callback! {}
