use sc_deainostri_nft_staking::*;

use elrond_wasm::{
    sc_error,
    types::{Address, SCResult},
};

use elrond_wasm_debug::{
    managed_address, managed_biguint, managed_token_id, managed_token_id_wrapped, rust_biguint,
    testing_framework::*, DebugApi,
};

const WASM_PATH: &'static str = "output/sc-deainostri-nft-staking.wasm";

struct SCTextContext<SCObjBuilder>
where
    SCObjBuilder: 'static + Copy + Fn() -> sc_deainostri_nft_staking::ContractObj<DebugApi>,
{
    pub blockchain_wrapper: BlockchainStateWrapper,
    pub owner_address: Address,
    pub first_user_address: Address,
    pub cf_wrapper:
        ContractObjWrapper<sc_deainostri_nft_staking::ContractObj<DebugApi>, SCObjBuilder>,
}

fn setup_sc<SCObjBuilder>(cf_builder: SCObjBuilder) -> SCTextContext<SCObjBuilder>
where
    SCObjBuilder: 'static + Copy + Fn() -> sc_deainostri_nft_staking::ContractObj<DebugApi>,
{
    let rust_zero = rust_biguint!(0u64);

    let mut blockchain_wrapper = BlockchainStateWrapper::new();

    let owner_address = blockchain_wrapper.create_user_account(&rust_zero);
    let first_user_address = blockchain_wrapper.create_user_account(&rust_zero);
    let cf_wrapper = blockchain_wrapper.create_sc_account(
        &rust_zero,
        Some(&owner_address),
        cf_builder,
        WASM_PATH,
    );

    // blockchain_wrapper.set_esdt_balance(&first_user_address, CF_TOKEN_ID, &rust_biguint!(1_000));

    blockchain_wrapper
        .execute_tx(&owner_address, &cf_wrapper, &rust_zero, |sc| {
            // let target = managed_biguint!(2_000);
            // let token_id = managed_token_id!(CF_TOKEN_ID);

            sc.init();
        })
        .assert_ok();

    const CF_TOKEN_ID: &[u8] = b"DEAN-123456";

    blockchain_wrapper
        .execute_tx(&owner_address, &cf_wrapper, &rust_zero, |sc| {
            sc.set_nft_token_id(managed_token_id_wrapped!(CF_TOKEN_ID));
        })
        .assert_ok();

    blockchain_wrapper.add_mandos_set_account(cf_wrapper.address_ref());

    SCTextContext {
        blockchain_wrapper,
        owner_address,
        first_user_address,
        cf_wrapper,
    }
}

const CF_TOKEN_ID: &[u8] = b"DEAN-123456";

#[test]
fn stake_test() {
    let mut cf_setup = setup_sc(sc_deainostri_nft_staking::contract_obj);

    let b_wrapper = &mut cf_setup.blockchain_wrapper;
    let user_addr = &cf_setup.first_user_address;

    b_wrapper
        .execute_esdt_transfer(
            user_addr,
            &cf_setup.cf_wrapper,
            CF_TOKEN_ID,
            20,
            &rust_biguint!(1),
            |sc| {
                sc.stake();

                // let nonce: u64 = 20;
                // let sc_nft_owner = sc.nft_owner(&nonce).get();
                // assert_eq!(sc_nft_owner, managed_address!(&user_addr));

                // let user_deposit = sc.deposit(&managed_address!(user_addr)).get();
                // let expected_deposit = managed_biguint!(1_000);
                // assert_eq!(user_deposit, expected_deposit);
            },
        )
        .assert_ok();

    let mut sc_call = ScCallMandos::new(user_addr, cf_setup.cf_wrapper.address_ref(), "stake");
    sc_call.add_esdt_transfer(CF_TOKEN_ID, 0, &rust_biguint!(1));

    let expect = TxExpectMandos::new(0);
    b_wrapper.add_mandos_sc_call(sc_call, Some(expect));

    cf_setup
        .blockchain_wrapper
        .write_mandos_output("_generated_stake.scen.json");
}
