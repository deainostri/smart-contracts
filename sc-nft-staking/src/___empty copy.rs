#![no_std]
#![feature(generic_associated_types)]
const REQUIRED_NFT_AMOUNT: u32 = 1;

elrond_wasm::imports!();

pub mod wallet_info;
use wallet_info::*;

pub mod nft_info;
use nft_info::*;

const DAY_IN_SECONDS: u64 = 86400;

#[elrond_wasm::contract]
pub trait DeainostriNFTStaking {
    //

    #[init]
    fn init(&self) {}

    // -----------------------
    // action methods
    // -----------------------

    #[payable("*")]
    #[endpoint(stake)]
    fn stake(&self) {
        //
        let received_token = self.call_value().single_esdt();
        let nft_amount = received_token.amount;

        //
        require!(
            &received_token.token_identifier == &self.nft_token_id().get().unwrap_esdt(),
            "Invalid NFT received!"
        );

        //
        require!(
            nft_amount == REQUIRED_NFT_AMOUNT,
            "Invalid NFT amount received!"
        );

        // define vars
        let caller_address = self.blockchain().get_caller();
        let mut wallet_info;

        // resolve wallet info
        if self
            .stake_map()
            .contains_key(&self.blockchain().get_caller())
        {
            wallet_info = self
                .stake_map()
                .get(&self.blockchain().get_caller())
                .unwrap();
        } else {
            wallet_info = WalletInfo {
                address: self.blockchain().get_caller(),
                nfts: ManagedVec::new(),
                points: 0,
                claimed: BigUint::zero(),
                claimeable: BigUint::zero(),
            };
        }

        // marked nft as owned by caller
        self.nft_owner(&received_token.token_nonce)
            .set(caller_address);

        // mark timestamp when nft was staked
        self.nft_staked_at(&received_token.token_nonce)
            .set(self.blockchain().get_block_timestamp());

        // update wallet info
        wallet_info.nfts.push(received_token.token_nonce);

        // save wallet info
        self.stake_map()
            .insert(self.blockchain().get_caller(), wallet_info);
    }

    #[endpoint(unstake)]
    fn unstake(&self, nonce: u64) {
        //
        require!(self.is_nft_staked(nonce), "NFT is not staked!");

        require!(
            self.nft_owner(&nonce).get() == self.blockchain().get_caller(),
            "NFT is not staked by the caller!"
        );

        require!(
            self.stake_map()
                .contains_key(&self.blockchain().get_caller()),
            "NFT Owner never staked!"
        );

        // compute new points
        let points_to_add = self.get_new_points_by_nft(&nonce);
        let mut wallet_info = self
            .stake_map()
            .get(&self.blockchain().get_caller())
            .unwrap();

        // add unclaimable points to caller wallet
        wallet_info.points += points_to_add;

        // find index of nonce in wallet_info.nfts
        let index = wallet_info
            .nfts
            .iter()
            .position(|nft_nonce| nft_nonce == nonce)
            .unwrap();

        // remove nft from wallet_info.nfts
        wallet_info.nfts.remove(index);

        // remove nft from owner map
        self.nft_owner(&nonce).clear();

        // remove nft from timestamp map
        self.nft_staked_at(&nonce).clear();

        // send nft to owner
        self.send().direct(
            &self.blockchain().get_caller(),
            &self.nft_token_id().get(),
            nonce.into(),
            &BigUint::from(REQUIRED_NFT_AMOUNT),
            &[],
        );
    }

    #[only_owner]
    #[endpoint(commit)]
    fn compute_points(&self) {
        let total_points: u64 = 0;

        // for each stake nft in map
        for (address, wallet_info) in self.stake_map().iter() {
            // for each nft in wallet_info.nfts
            for nft_nonce in wallet_info.nfts.iter() {
                // compute new points
                let points_to_add = self.get_new_points_by_nft(nft_nonce);

                // add new points to wallet_info.points
                wallet_info.points += points_to_add;

                // add new points to total_points
                total_points += points_to_add;
            }
        }
    }

    fn get_new_points_by_nft(&self, nonce: &u64) -> u64 {
        //
        if self.nft_staked_at(nonce).is_empty() || self.nft_owner(nonce).is_empty() {
            return 0;
        }

        //
        let nft_staked_at = self.nft_staked_at(nonce).get();
        let nft_owner = self.nft_owner(nonce).get();

        //
        return (self.blockchain().get_block_timestamp() - nft_staked_at) / DAY_IN_SECONDS;
    }

    fn is_nft_staked(&self, nonce: u64) -> bool {
        return !self.nft_owner(&nonce).is_empty();
    }

    // -----------------------
    // setup methods
    // -----------------------

    #[only_owner]
    #[endpoint(setNftTokenId)]
    fn set_nft_token_id(&self, token_identifier: EgldOrEsdtTokenIdentifier) {
        //
        self.nft_token_id().set(token_identifier);
    }

    // -----------------------
    // storage
    // -----------------------

    // #[view(getNftTokenId)]
    #[storage_mapper("storage_nft_token_id")]
    fn nft_token_id(&self) -> SingleValueMapper<EgldOrEsdtTokenIdentifier>;

    #[storage_mapper("storage_staked_nfts_by_address")]
    fn stake_map(&self) -> MapMapper<ManagedAddress, WalletInfo<Self::Api>>;

    // #[storage_mapper("storage_staked_nfts_by_address")]
    // fn nft_map(&self) -> MapMapper<u64, NftInfo<Self::Api>>;

    #[storage_mapper("storage_nft_owner")]
    fn nft_owner(&self, nonce: &u64) -> SingleValueMapper<ManagedAddress>;

    #[storage_mapper("storage_nft_staked_at")]
    fn nft_staked_at(&self, nonce: &u64) -> SingleValueMapper<u64>;

    #[storage_mapper("storage_points_by_address")]
    fn points_by_address(&self, address: &ManagedAddress) -> SingleValueMapper<u64>;

    #[storage_mapper("storage_claimable_by_address")]
    fn claimable_by_address(&self, address: &ManagedAddress) -> SingleValueMapper<BigUint>;

    #[storage_mapper("storage_claimed_by_address")]
    fn claimed_by_address(&self, address: &ManagedAddress) -> SingleValueMapper<BigUint>;

    #[storage_mapper("storage_nfts_staked_by_address")]
    fn nfts_staked_by_address(&self, address: &ManagedAddress) -> UnorderedSetMapper<u64>;

    #[storage_mapper("storage_staked_adresses")]
    fn staked_adresses(&self) -> UnorderedSetMapper<ManagedAddress>;
}
