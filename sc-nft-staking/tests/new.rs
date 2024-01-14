use sc_deainostri_nft_staking::*;

use elrond_wasm::{
    sc_error, sc_print,
    types::{Address, SCResult},
};

use elrond_wasm_debug::{
    managed_address, managed_biguint, managed_buffer, managed_token_id, managed_token_id_wrapped,
    rust_biguint, testing_framework::*, DebugApi,
};

const WASM_PATH: &'static str = "output/sc-deainostri-nft-staking.wasm";
const CF_TOKEN_ID: &[u8] = b"DEAN-123456";

struct SCTextContext<SCObjBuilder>
where
    SCObjBuilder: 'static + Copy + Fn() -> sc_deainostri_nft_staking::ContractObj<DebugApi>,
{
    pub blockchain_wrapper: BlockchainStateWrapper,
    pub owner_address: Address,

    pub alice: Address,
    pub bob: Address,
    pub charlie: Address,
    pub zeta: Address,

    pub cf_wrapper:
        ContractObjWrapper<sc_deainostri_nft_staking::ContractObj<DebugApi>, SCObjBuilder>,
}

fn setup_sc<SCObjBuilder>(cf_builder: SCObjBuilder) -> SCTextContext<SCObjBuilder>
where
    SCObjBuilder: 'static + Copy + Fn() -> sc_deainostri_nft_staking::ContractObj<DebugApi>,
{
    let rust_zero = rust_biguint!(0u64);
    let mut blockchain_wrapper = BlockchainStateWrapper::new();

    // -----------------------
    // create addresses
    // -----------------------

    let owner_address = blockchain_wrapper.create_user_account(&rust_biguint!(9000000000u64));
    let alice = blockchain_wrapper.create_user_account(&rust_zero);
    let bob = blockchain_wrapper.create_user_account(&rust_zero);
    let charlie = blockchain_wrapper.create_user_account(&rust_zero);
    let zeta = blockchain_wrapper.create_user_account(&rust_zero);

    // -----------------------
    // add nft tokens
    // -----------------------

    blockchain_wrapper.set_nft_balance(
        &alice,
        CF_TOKEN_ID,
        1,
        &rust_biguint!(1),
        &rust_biguint!(1),
    );

    blockchain_wrapper.set_nft_balance(&bob, CF_TOKEN_ID, 2, &rust_biguint!(1), &rust_biguint!(1));
    blockchain_wrapper.set_nft_balance(&bob, CF_TOKEN_ID, 20, &rust_biguint!(1), &rust_biguint!(1));

    blockchain_wrapper.set_nft_balance(
        &charlie,
        CF_TOKEN_ID,
        3,
        &rust_biguint!(1),
        &rust_biguint!(1),
    );

    blockchain_wrapper.set_nft_balance(&zeta, CF_TOKEN_ID, 4, &rust_biguint!(1), &rust_biguint!(1));

    // -----------------------
    // create sc wrapper
    // -----------------------

    let cf_wrapper = blockchain_wrapper.create_sc_account(
        &rust_zero,
        Some(&owner_address),
        cf_builder,
        WASM_PATH,
    );

    // -----------------------
    // init sc
    // -----------------------

    blockchain_wrapper
        .execute_tx(&owner_address, &cf_wrapper, &rust_zero, |sc| {
            // let target = managed_biguint!(2_000);
            // let token_id = managed_token_id!(CF_TOKEN_ID);

            sc.init();
        })
        .assert_ok();

    blockchain_wrapper
        .execute_tx(&owner_address, &cf_wrapper, &rust_zero, |sc| {
            sc.set_nft_token_id(managed_token_id_wrapped!(CF_TOKEN_ID));
        })
        .assert_ok();

    blockchain_wrapper.add_mandos_set_account(cf_wrapper.address_ref());

    SCTextContext {
        blockchain_wrapper,
        owner_address,
        cf_wrapper,
        alice,
        bob,
        charlie,
        zeta,
    }
}

#[test]
fn init_test() {
    let cf_setup = setup_sc(sc_deainostri_nft_staking::contract_obj);

    cf_setup
        .blockchain_wrapper
        .write_mandos_output("stake-function.scen.json");
}

#[test]
fn stake_test() {
    let mut cf_setup = setup_sc(sc_deainostri_nft_staking::contract_obj);
    let b_wrapper = &mut cf_setup.blockchain_wrapper;
    let sc = &mut cf_setup.cf_wrapper;

    let owner = &cf_setup.owner_address;
    let alice = &cf_setup.alice;
    let bob = &cf_setup.bob;
    let charlie = &cf_setup.charlie;
    let zeta = &cf_setup.zeta;

    let mut current_timestamp = 1654284522;
    let day_in_seconds = 86400;

    b_wrapper.set_block_timestamp(current_timestamp);
    print!("starting sc\n\n");

    // -----------------------
    // alice add to stake
    // -----------------------

    b_wrapper
        .execute_esdt_transfer(alice, &sc, CF_TOKEN_ID, 1, &rust_biguint!(1), |sc| {
            sc.stake();
            print!("alice staked 1 nft\n");
            assert_eq!(sc.nft_owner(&1u64).get(), managed_address!(&alice));
        })
        .assert_ok();

    // -----------------------
    // bob add to stake
    // -----------------------

    b_wrapper
        .execute_esdt_transfer(bob, &sc, CF_TOKEN_ID, 2, &rust_biguint!(1), |sc| {
            sc.stake();
            print!("bob staked 1 nft\n");
            assert_eq!(sc.nft_owner(&2u64).get(), managed_address!(&bob));
        })
        .assert_ok();

    // -----------------------
    // charlie add to stake
    // -----------------------

    b_wrapper
        .execute_esdt_transfer(charlie, &sc, CF_TOKEN_ID, 3, &rust_biguint!(1), |sc| {
            sc.stake();
            print!("charlie staked 1 nft\n");
            assert_eq!(sc.nft_owner(&3u64).get(), managed_address!(&charlie));
        })
        .assert_ok();

    // -----------------------
    // go forward 10 days
    // -----------------------

    current_timestamp = current_timestamp + 10 * day_in_seconds;
    b_wrapper.set_block_timestamp(current_timestamp);
    print!("\n");
    print!("fast-forwarding 10 days...\n\n");

    // -----------------------
    // compute points
    // -----------------------

    b_wrapper
        .execute_tx(&owner, &sc, &rust_biguint!(0u64), |sc| {
            //
            print!("computing all points...\n");
            sc.compute_all_points();

            print!("## after compute all_points\n");
            print!(
                "$alice_points: {:?}\n",
                sc.points_by_address(&managed_address!(alice)).get()
            );
            print!(
                "$bob_points: {:?}\n",
                sc.points_by_address(&managed_address!(bob)).get()
            );
            print!(
                "$charlie_points: {:?}\n",
                sc.points_by_address(&managed_address!(charlie)).get()
            );
            print!("$total_points: {:?}\n", sc.total_points().get());
        })
        .assert_ok();
    print!("\n");

    // -----------------------
    // compute percentages
    // -----------------------

    b_wrapper
        .execute_tx(&cf_setup.owner_address, &sc, &rust_biguint!(0u64), |sc| {
            //
            sc.compute_percentages();

            print!("## after compute percentages\n");
            print!(
                "$alice_percentage: {:?}\n",
                sc.points_by_address(&managed_address!(alice)).get()
            );
            print!(
                "$bob_percentage: {:?}\n",
                sc.points_by_address(&managed_address!(bob)).get()
            );
            print!(
                "$charlie_percentage: {:?}\n",
                sc.points_by_address(&managed_address!(charlie)).get()
            );
        })
        .assert_ok();
    print!("\n");

    // -----------------------
    // add stake reward
    // -----------------------

    b_wrapper
        .execute_tx(
            &cf_setup.owner_address,
            &sc,
            &rust_biguint!(3_000u64),
            |sc| {
                //
                sc.add_stake_reward();

                print!("## after reward set\n");
                print!(
                    "$alice_percentage: {:?}\n",
                    sc.points_by_address(&managed_address!(alice)).get()
                );
                print!(
                    "$bob_percentage: {:?}\n",
                    sc.points_by_address(&managed_address!(bob)).get()
                );
                print!(
                    "$charlie_percentage: {:?}\n",
                    sc.points_by_address(&managed_address!(charlie)).get()
                );
                print!(
                    "$alice_claimable: {:?}\n",
                    sc.claimable_by_address(&managed_address!(alice))
                        .get()
                        .to_u64()
                );
                print!(
                    "$bob_claimable: {:?}\n",
                    sc.claimable_by_address(&managed_address!(bob))
                        .get()
                        .to_u64()
                );
                print!(
                    "$charlie_claimable: {:?}\n",
                    sc.claimable_by_address(&managed_address!(charlie))
                        .get()
                        .to_u64()
                );
            },
        )
        .assert_ok();
    print!("\n");

    b_wrapper
        .execute_tx(alice, &sc, &rust_biguint!(0), |sc| {
            print!("alice unstaking...\n");
            sc.unstake(1u64)
        })
        .assert_ok();
    print!("\n");

    // -----------------------
    // bob add to stake 2nd nft
    // -----------------------

    b_wrapper
        .execute_esdt_transfer(bob, &sc, CF_TOKEN_ID, 20, &rust_biguint!(1), |sc| {
            sc.stake();
            print!("bob staked 2nd nft\n");
            assert_eq!(sc.nft_owner(&20u64).get(), managed_address!(&bob));
        })
        .assert_ok();

    current_timestamp = current_timestamp + 5 * day_in_seconds;
    b_wrapper.set_block_timestamp(current_timestamp);
    print!("fast-forwarding 5 days...\n\n");

    b_wrapper
        .execute_tx(bob, &sc, &rust_biguint!(0), |sc| {
            print!("bob unstaking...\n");
            sc.unstake(2u64);

            print!("## after bob unstake\n");
            print!(
                "$bob_percentage: {:?}\n",
                sc.points_by_address(&managed_address!(bob)).get()
            );
        })
        .assert_ok();
    print!("\n");

    b_wrapper
        .execute_tx(bob, &sc, &rust_biguint!(0), |sc| {
            print!("bob unstaking...\n");
            sc.unstake(20u64);

            print!("## after bob unstake\n");
            print!(
                "$bob_percentage: {:?}\n",
                sc.points_by_address(&managed_address!(bob)).get()
            );
        })
        .assert_ok();
    print!("\n");

    current_timestamp = current_timestamp + 5 * day_in_seconds;
    b_wrapper.set_block_timestamp(current_timestamp);
    print!("fast-forwarding 5 days...\n\n");

    // -----------------------
    // compute points
    // -----------------------

    b_wrapper
        .execute_tx(&owner, &sc, &rust_biguint!(0u64), |sc| {
            //
            print!("computing all points...\n");
            sc.compute_all_points();

            print!("## after compute all_points\n");
            print!(
                "$alice_points: {:?}\n",
                sc.points_by_address(&managed_address!(alice)).get()
            );
            print!(
                "$bob_points: {:?}\n",
                sc.points_by_address(&managed_address!(bob)).get()
            );
            print!(
                "$charlie_points: {:?}\n",
                sc.points_by_address(&managed_address!(charlie)).get()
            );
            print!("$total_points: {:?}\n", sc.total_points().get());
        })
        .assert_ok();
    print!("\n");

    // -----------------------
    // compute percentages
    // -----------------------

    b_wrapper
        .execute_tx(&cf_setup.owner_address, &sc, &rust_biguint!(0u64), |sc| {
            //
            sc.compute_percentages();

            print!("## after compute percentages\n");
            print!(
                "$alice_percentage: {:?}\n",
                sc.points_by_address(&managed_address!(alice)).get()
            );
            print!(
                "$bob_percentage: {:?}\n",
                sc.points_by_address(&managed_address!(bob)).get()
            );
            print!(
                "$charlie_percentage: {:?}\n",
                sc.points_by_address(&managed_address!(charlie)).get()
            );
        })
        .assert_ok();
    print!("\n");

    // -----------------------
    // add stake reward
    // -----------------------

    b_wrapper
        .execute_tx(
            &cf_setup.owner_address,
            &sc,
            &rust_biguint!(3_000u64),
            |sc| {
                //
                sc.add_stake_reward();

                print!("## after reward set\n");
                print!(
                    "$alice_percentage: {:?}\n",
                    sc.points_by_address(&managed_address!(alice)).get()
                );
                print!(
                    "$bob_percentage: {:?}\n",
                    sc.points_by_address(&managed_address!(bob)).get()
                );
                print!(
                    "$charlie_percentage: {:?}\n",
                    sc.points_by_address(&managed_address!(charlie)).get()
                );
                print!(
                    "$alice_claimable: {:?}\n",
                    sc.claimable_by_address(&managed_address!(alice))
                        .get()
                        .to_u64()
                );
                print!(
                    "$bob_claimable: {:?}\n",
                    sc.claimable_by_address(&managed_address!(bob))
                        .get()
                        .to_u64()
                );
                print!(
                    "$charlie_claimable: {:?}\n",
                    sc.claimable_by_address(&managed_address!(charlie))
                        .get()
                        .to_u64()
                );
            },
        )
        .assert_ok();
    print!("\n");

    // -----------------------
    // charlie claim
    // -----------------------

    b_wrapper
        .execute_tx(&charlie, &sc, &rust_biguint!(0u64), |sc| {
            //
            sc.claim();

            print!("## after charlie claim\n");
            print!(
                "$alice_claimable: {:?}\n",
                sc.claimable_by_address(&managed_address!(alice))
                    .get()
                    .to_u64()
            );
            print!(
                "$bob_claimable: {:?}\n",
                sc.claimable_by_address(&managed_address!(bob))
                    .get()
                    .to_u64()
            );
            print!(
                "$charlie_claimable: {:?}\n",
                sc.claimable_by_address(&managed_address!(charlie))
                    .get()
                    .to_u64()
            );
        })
        .assert_ok();
    print!("\n");
}
