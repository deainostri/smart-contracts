#![no_std]
#![feature(generic_associated_types)]
const REQUIRED_NFT_AMOUNT: u32 = 1;

elrond_wasm::imports!();

// pub mod wallet_info;
// use wallet_info::*;

// pub mod nft_info;
// use nft_info::*;

const DAY_IN_SECONDS: u64 = 86400;
const PERCENTAGE_PREC: u64 = 1000000;

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

        require!(self.is_locked().is_empty(), "SC is locked!");

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

        // marked nft as owned by caller
        self.nft_owner(&received_token.token_nonce)
            .set(self.blockchain().get_caller());

        // mark timestamp when nft was staked
        self.nft_staked_at(&received_token.token_nonce)
            .set(self.blockchain().get_block_timestamp());

        // insert nonce into staked nfts
        self.nfts_staked_by_address(&self.blockchain().get_caller())
            .insert(received_token.token_nonce);

        // insert address in to staked addresses
        self.staked_adresses()
            .insert(self.blockchain().get_caller());
    }

    #[endpoint(unstake)]
    fn unstake(&self, nonce: u64) {
        //
        require!(self.is_locked().is_empty(), "SC is locked!");

        //
        require!(
            self.is_nft_staked_by_caller(nonce),
            "NFT is not staked by you!"
        );

        // compute new points
        self.add_new_points_by_nft(&nonce);

        // remove nft from owner map
        self.nft_owner(&nonce).clear();

        // remove nft from timestamp map
        self.nft_staked_at(&nonce).clear();

        // remove nft from staked nfts map
        self.nfts_staked_by_address(&self.blockchain().get_caller())
            .swap_remove(&nonce);

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
    #[endpoint(computePoints)]
    fn compute_all_points(&self) {
        let current_index = if self.compute_index().is_empty() {
            0
        } else {
            self.compute_index().get()
        };
        let max_index = self.staked_adresses().len() as u64;
        // let mut total_points: u64 = self.total_points().get();
        let mut new_index = current_index;

        // iterate over all staked addresses
        for index in current_index..max_index {
            let address = self.staked_adresses().iter().nth(index as usize).unwrap();
            let mut points = self.points_by_address(&address).get();

            // iterate over all staked nfts by address
            for nft_nonce in self.nfts_staked_by_address(&address).iter() {
                // compute new points
                let points_to_add = self.get_new_points_by_nft(&nft_nonce);

                // reset nft staked at date
                self.nft_staked_at(&nft_nonce)
                    .set(self.blockchain().get_block_timestamp());

                // add new points
                points += points_to_add;
            }

            self.points_by_address(&address).set(points);

            // set new points
            // total_points += self.points_by_address(&address).get();

            new_index = index;

            // if not enough gas, intrerrupt
            if self.blockchain().get_gas_left() < 4400000 {
                break;
            }
        }

        self.compute_index().set(new_index);
        // self.total_points().set(total_points);
    }

    #[only_owner]
    #[endpoint(computeTotalPoints)]
    fn compute_total_points(&self) {
        let mut total_points: u64 = 0;

        // iterate over all staked addresses
        for address in self.staked_adresses().iter() {
            let points = self.points_by_address(&address).get();

            // add new points
            total_points += points;
        }

        self.total_points().set(total_points);
    }

    #[only_owner]
    #[endpoint(resetComputeIndex)]
    fn reset_compute_index(&self) {
        self.compute_index().set(0);
    }

    #[only_owner]
    #[endpoint(computePercentages)]
    fn compute_percentages(&self) {
        let total_points = self.total_points().get();
        let current_index = if self.compute_index().is_empty() {
            0
        } else {
            self.compute_index().get()
        };
        let max_index = self.staked_adresses().len() as u64;
        let mut new_index = current_index;

        // iterate over all staked addresses
        // for address in self.staked_adresses().iter() {
        for index in current_index..max_index {
            let address = self.staked_adresses().iter().nth(index as usize).unwrap();
            let points = self.points_by_address(&address).get();
            let percentage = (points * PERCENTAGE_PREC) / total_points;

            new_index = index;

            // set new percentage
            self.points_by_address(&address).set(percentage);

            // if not enough gas, intrerrupt
            if self.blockchain().get_gas_left() < 10000000 {
                break;
            }
        }

        self.compute_index().set(new_index);
        // self.total_points().set(0);
    }

    #[only_owner]
    #[payable("*")]
    #[endpoint(addStakeReward)]
    fn add_stake_reward(&self) {
        let reward_amount = self.call_value().egld_value();

        // iterate over all staked addresses
        for address in self.staked_adresses().iter() {
            let percentage = self.points_by_address(&address).get();
            let claimable = (&reward_amount * percentage) / PERCENTAGE_PREC;

            // set new percentage
            self.points_by_address(&address).set(0);
            self.claimable_by_address(&address)
                .set(claimable + self.claimable_by_address(&address).get());
        }
    }

    #[only_owner]
    #[endpoint(resolveStakeReward)]
    fn resolve_stake_reward(&self, reward_amount: BigUint) {
        let current_index = if self.compute_index().is_empty() {
            0
        } else {
            self.compute_index().get()
        };
        let max_index = self.staked_adresses().len() as u64;
        let mut new_index = current_index;

        // iterate over all staked addresses
        // for address in self.staked_adresses().iter() {
        for index in current_index..max_index {
            let address = self.staked_adresses().iter().nth(index as usize).unwrap();
            let percentage = self.points_by_address(&address).get();
            let claimable = (&reward_amount * percentage) / PERCENTAGE_PREC;

            new_index = index;

            // set new percentage
            self.points_by_address(&address).set(0);
            self.claimable_by_address(&address)
                .set(claimable + self.claimable_by_address(&address).get());

            // if not enough gas, intrerrupt
            if self.blockchain().get_gas_left() < 10000000 {
                break;
            }
        }

        self.compute_index().set(new_index);
    }

    #[endpoint(claim)]
    fn claim(&self) {
        //
        require!(self.is_locked().is_empty(), "SC is locked!");

        //
        let claimable = self
            .claimable_by_address(&self.blockchain().get_caller())
            .get();

        let claimed = &self.claimed_by_address(&self.blockchain().get_caller())
            .get();

        self.claimed_by_address(&self.blockchain().get_caller())
            .set(claimed + &claimable);

        self.claimable_by_address(&self.blockchain().get_caller())
            .set(&BigUint::zero());

        self.send()
            .direct_egld(&self.blockchain().get_caller(), &claimable, &[]);
    }

    fn add_new_points_by_nft(&self, nonce: &u64) {
        //
        let points = self.get_new_points_by_nft(nonce);
        let address = self.nft_owner(nonce).get();

        // add new points to points map
        self.points_by_address(&address)
            .set(points + self.points_by_address(&address).get());
    }

    fn get_new_points_by_nft(&self, nonce: &u64) -> u64 {
        //
        if self.nft_staked_at(nonce).is_empty() || self.nft_owner(nonce).is_empty() {
            return 0;
        }

        let interval = if self.tick_interval().is_empty() {
            DAY_IN_SECONDS
        } else {
            self.tick_interval().get()
        };

        //
        return (self.blockchain().get_block_timestamp() - self.nft_staked_at(nonce).get())
            / interval;
    }

    #[view(getNewPointsByAddress)]
    fn get_new_points_by_address(&self, address: &ManagedAddress) -> u64 {
        return self
            .nfts_staked_by_address(address)
            .iter()
            .fold(0, |acc, nft_nonce| {
                acc + self.get_new_points_by_nft(&nft_nonce)
            });
    }

    fn is_nft_staked(&self, nonce: u64) -> bool {
        return !self.nft_owner(&nonce).is_empty() && !self.nft_staked_at(&nonce).is_empty();
    }

    fn is_nft_staked_by_caller(&self, nonce: u64) -> bool {
        return self.is_nft_staked(nonce)
            && self.nft_owner(&nonce).get() == self.blockchain().get_caller();
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

    #[only_owner]
    #[endpoint(setNftStartStakeDate)]
    fn set_nft_start_stake_date(&self, nonce: u64, start_stake_date: u64) {
        //
        self.nft_staked_at(&nonce).set(start_stake_date);
    }

    #[only_owner]
    #[endpoint(lock)]
    fn lock(&self) {
        self.is_locked().set(true);
    }

    #[only_owner]
    #[endpoint(unlock)]
    fn unlock(&self) {
        self.is_locked().clear();
    }

    #[only_owner]
    #[endpoint(setTickInterval)]
    fn set_tick_interval(&self, tick_interval: u64) {
        self.tick_interval().set(tick_interval);
    }

    #[only_owner]
    #[endpoint(fixStakeReward)]
    fn fix_stake_reward(&self, address: ManagedAddress, amount: BigUint) {
        self.claimable_by_address(&address).set(&amount);
    }

    // -----------------------
    // util storage
    // -----------------------

    #[view(getComputeIndex)]
    #[storage_mapper("storage_compute_index")]
    fn compute_index(&self) -> SingleValueMapper<u64>;

    #[view(getTotalPoints)]
    #[storage_mapper("storage_total_points")]
    fn total_points(&self) -> SingleValueMapper<u64>;

    // -----------------------
    // storage
    // -----------------------

    #[view(getIsLocked)]
    #[storage_mapper("storage_is_locked")]
    fn is_locked(&self) -> SingleValueMapper<bool>;

    #[view(getTickInterval)]
    #[storage_mapper("storage_tick_interval")]
    fn tick_interval(&self) -> SingleValueMapper<u64>;

    // #[view(getNftTokenId)]
    #[storage_mapper("storage_nft_token_id")]
    fn nft_token_id(&self) -> SingleValueMapper<EgldOrEsdtTokenIdentifier>;

    // #[storage_mapper("storage_staked_nfts_by_address")]
    // fn stake_map(&self) -> MapMapper<ManagedAddress, WalletInfo<Self::Api>>;

    // #[storage_mapper("storage_staked_nfts_by_address")]
    // fn nft_map(&self) -> MapMapper<u64, NftInfo<Self::Api>>;

    #[storage_mapper("storage_nft_owner")]
    fn nft_owner(&self, nonce: &u64) -> SingleValueMapper<ManagedAddress>;

    #[storage_mapper("storage_nft_staked_at")]
    fn nft_staked_at(&self, nonce: &u64) -> SingleValueMapper<u64>;

    #[view(getPointsByAddress)]
    #[storage_mapper("storage_points_by_address")]
    fn points_by_address(&self, address: &ManagedAddress) -> SingleValueMapper<u64>;

    #[view(getClaimableByAddress)]
    #[storage_mapper("storage_claimable_by_address")]
    fn claimable_by_address(&self, address: &ManagedAddress) -> SingleValueMapper<BigUint>;

    #[view(getClaimedByAddress)]
    #[storage_mapper("storage_claimed_by_address")]
    fn claimed_by_address(&self, address: &ManagedAddress) -> SingleValueMapper<BigUint>;

    #[view(getStakedNfts)]
    #[storage_mapper("storage_nfts_staked_by_address")]
    fn nfts_staked_by_address(&self, address: &ManagedAddress) -> UnorderedSetMapper<u64>;

    #[view(getStakedAddresses)]
    #[storage_mapper("storage_staked_adresses")]
    fn staked_adresses(&self) -> SetMapper<ManagedAddress>;
}
