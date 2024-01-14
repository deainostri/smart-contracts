#![no_std]

use core::convert::TryInto;

const NFT_AMOUNT: u32 = 1;
const ROYALTIES_MAX: u32 = 10_000;
const IPFS_GATEWAY_HOST: &[u8] = "https://ipfs.io/ipfs/".as_bytes();
const METADATA_KEY_NAME: &[u8] = "metadata:".as_bytes();
const METADATA_FILE_EXTENSION: &[u8] = ".json".as_bytes();
const ATTR_SEPARATOR: &[u8] = ";".as_bytes();
const URI_SLASH: &[u8] = "/".as_bytes();
const TAGS_KEY_NAME: &[u8] = "tags:".as_bytes();
const IMG_FILE_EXTENSION: &[u8] = ".png".as_bytes();
const COLLECTION_JSON_FILENAME: &[u8] = "collection.json".as_bytes();
const ESDT_NFT_UPDATE_ATTRIBUTES_FUNC_NAME: &[u8] = "ESDTNFTUpdateAttributes".as_bytes();
const AFTER_NAME_BEFORE_NUMBER: &[u8] = " #".as_bytes();
const HASH_DATA_BUFFER_LEN: usize = 1024;

use elrond_wasm::hex_literal::hex;

// mainnet
const MARKETPLACE: [u8; 32] =
    hex!("00000000000000000500d3b28828d62052124f07dcd50ed31b0825f60eee1526");

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

#[elrond_wasm::contract]
pub trait ElvenTools {
    #[init]
    fn init(
        &self,
        full_base_cid: ManagedBuffer,
        token_name: ManagedBuffer,
        amount_of_tokens: u32,
        royalties: BigUint,
        selling_price: BigUint,
        presale_price: BigUint,
        tags: OptionalValue<ManagedBuffer>,
        provenance_hash: OptionalValue<ManagedBuffer>,
    ) {
        require!(royalties <= ROYALTIES_MAX, "Royalties cannot exceed 100%!");
        require!(
            amount_of_tokens >= 1,
            "Amount of tokens to mint should be at least 1!"
        );

        self.full_base_cid().set_if_empty(&full_base_cid);

        // set nft token base name
        self.nft_token_name().set_if_empty(&token_name);

        // set max amount of tokens
        self.amount_of_tokens_total()
            .set_if_empty(&amount_of_tokens);

        // set provenance hash?
        self.provenance_hash()
            .set_if_empty(&provenance_hash.into_option().unwrap_or_default());

        // set royalties
        self.royalties().set_if_empty(&royalties);

        // set selling price
        self.selling_price().set_if_empty(&selling_price);

        // set presale price
        self.presale_price().set_if_empty(&presale_price);

        // set tags
        self.tags()
            .set_if_empty(&tags.into_option().unwrap_or_default());

        let paused = true;
        self.paused().set(&paused);

        if self.next_index_to_mint().is_empty() {
            let first_index = 1;
            self.next_index_to_mint().set(&first_index);
        }
    }

    #[only_owner]
    #[payable("EGLD")]
    #[endpoint(issueToken)]
    fn issue_token(
        &self,
        #[payment] issue_cost: BigUint,
        token_name: ManagedBuffer,
        token_ticker: ManagedBuffer,
    ) {
        require!(self.nft_token_id().is_empty(), "Token already issued!");

        self.send()
            .esdt_system_sc_proxy()
            .issue_non_fungible(
                issue_cost,
                &token_name,
                &token_ticker,
                NonFungibleTokenProperties {
                    can_freeze: false,
                    can_wipe: false,
                    can_pause: false,
                    can_change_owner: false,
                    can_upgrade: true,
                    can_add_special_roles: true,
                },
            )
            .async_call()
            .with_callback(self.callbacks().issue_callback())
            .call_and_exit();
    }

    #[callback]
    fn issue_callback(&self, #[call_result] result: ManagedAsyncCallResult<TokenIdentifier>) {
        match result {
            ManagedAsyncCallResult::Ok(token_id) => {
                self.nft_token_id().set(&token_id);
            }
            ManagedAsyncCallResult::Err(_) => {
                let caller = self.blockchain().get_owner_address();
                let (returned_tokens, token_id) = self.call_value().payment_token_pair();
                if token_id.is_egld() && returned_tokens > 0 {
                    self.send()
                        .direct(&caller, &token_id, 0, &returned_tokens, &[]);
                }
            }
        }
    }

    #[only_owner]
    #[endpoint(setLocalRoles)]
    fn set_local_roles(&self) {
        require!(!self.nft_token_id().is_empty(), "Token not issued!");

        self.send()
            .esdt_system_sc_proxy()
            .set_special_roles(
                &self.blockchain().get_sc_address(),
                &self.nft_token_id().get(),
                (&[EsdtLocalRole::NftCreate][..]).into_iter().cloned(),
            )
            .async_call()
            .call_and_exit();
    }

    #[only_owner]
    #[endpoint(setUpdateMetadataRole)]
    fn set_update_metadata_role(&self, manager_address: ManagedAddress) {
        require!(!self.nft_token_id().is_empty(), "Token not issued!");

        self.send()
            .esdt_system_sc_proxy()
            .set_special_roles(
                &manager_address,
                &self.nft_token_id().get(),
                (&[EsdtLocalRole::NftUpdateAttributes][..])
                    .into_iter()
                    .cloned(),
            )
            .async_call()
            .call_and_exit();
    }

    #[only_owner]
    #[endpoint(setBurnNftsRole)]
    fn set_burn_nfts_role(&self, manager_address: ManagedAddress) {
        require!(!self.nft_token_id().is_empty(), "Token not issued!");

        self.send()
            .esdt_system_sc_proxy()
            .set_special_roles(
                &manager_address,
                &self.nft_token_id().get(),
                (&[EsdtLocalRole::NftBurn][..]).into_iter().cloned(),
            )
            .async_call()
            .call_and_exit();
    }

    #[only_owner]
    #[endpoint(setToken)]
    fn set_token(&self, token_id: TokenIdentifier) {
        self.nft_token_id().set(&token_id);
    }

    // -----------------------
    // sale & pre-sale status methods
    // -----------------------

    #[only_owner]
    #[endpoint(pauseMinting)]
    fn pause_minting(&self) {
        let paused = true;
        self.paused().set(&paused);
    }

    #[only_owner]
    #[endpoint(startMinting)]
    fn start_minting(&self) {
        require!(!self.nft_token_id().is_empty(), "Token not issued!");

        self.paused().clear();
    }

    #[only_owner]
    #[endpoint(pausePresale)]
    fn pause_presale(&self) {
        self.presale_is_open().clear();
    }

    #[only_owner]
    #[endpoint(startPresale)]
    fn resume_presale(&self) {
        require!(!self.nft_token_id().is_empty(), "Token not issued!");

        self.presale_is_open().set(&true);
    }

    // -----------------------
    // presale methods
    // -----------------------

    #[only_owner]
    #[endpoint(approveWhitelist)]
    fn approve_whitelist(&self, address: ManagedAddress) {
        self.is_whitelisted(&address).set(&true);
    }

    fn internal_approve_whitelist(&self, address: ManagedAddress) {
        self.is_whitelisted(&address).set(&true);
    }

    #[only_owner]
    #[endpoint(approveWhitelistMany)]
    fn approve_whitelist_many(
        &self,
        address_one: ManagedAddress,
        address_two: OptionalValue<ManagedAddress>,
        address_three: OptionalValue<ManagedAddress>,
        address_four: OptionalValue<ManagedAddress>,
        address_five: OptionalValue<ManagedAddress>,
    ) {
        //
        self.internal_approve_whitelist(address_one);

        if let Some(address) = address_two.into_option() {
            self.internal_approve_whitelist(address);
        } else {
            return;
        }

        if let Some(address) = address_three.into_option() {
            self.internal_approve_whitelist(address);
        } else {
            return;
        }

        if let Some(address) = address_four.into_option() {
            self.internal_approve_whitelist(address);
        } else {
            return;
        }

        if let Some(address) = address_five.into_option() {
            self.internal_approve_whitelist(address);
        } else {
            return;
        }
    }

    // -----------------------
    // whitelist v2
    // -----------------------

    #[only_owner]
    #[endpoint(removeWhitelistPerDrop)]
    fn remove_whitelist_per_drop(&self, address: ManagedAddress) {
        self.is_whitelisted_per_drop(self.opened_drop().get())
            .remove(&address);
    }

    #[only_owner]
    #[endpoint(approveWhitelistPerDrop)]
    fn approve_whitelist_per_drop(&self, address: ManagedAddress) {
        self.internal_approve_whitelist_per_drop(address);
    }

    #[only_owner]
    #[endpoint(approveWhitelistPerDropMany)]
    fn approve_whitelist_per_drop_many(
        &self,
        address_one: ManagedAddress,
        address_two: OptionalValue<ManagedAddress>,
        address_three: OptionalValue<ManagedAddress>,
        address_four: OptionalValue<ManagedAddress>,
        address_five: OptionalValue<ManagedAddress>,
    ) {
        //
        self.internal_approve_whitelist_per_drop(address_one);

        if let Some(address) = address_two.into_option() {
            self.internal_approve_whitelist_per_drop(address);
        } else {
            return;
        }

        if let Some(address) = address_three.into_option() {
            self.internal_approve_whitelist_per_drop(address);
        } else {
            return;
        }

        if let Some(address) = address_four.into_option() {
            self.internal_approve_whitelist_per_drop(address);
        } else {
            return;
        }

        if let Some(address) = address_five.into_option() {
            self.internal_approve_whitelist_per_drop(address);
        } else {
            return;
        }
    }

    fn internal_approve_whitelist_per_drop(&self, address: ManagedAddress) {
        self.is_whitelisted_per_drop(self.opened_drop().get())
            .insert(address, true);
    }

    #[view(getIsWhitelistedPerDrop)]
    fn get_is_whitelisted_per_drop(&self, drop_number: u16, address: ManagedAddress) -> bool {
        let existing_address_value = self
            .is_whitelisted_per_drop(drop_number)
            .get(&address)
            .unwrap_or_default();

        existing_address_value
    }

    #[view(getIsWhitelistedPerCurrentDrop)]
    fn get_is_whitelisted_per_current_drop(&self, address: ManagedAddress) -> bool {
        let existing_address_value =
            self.get_is_whitelisted_per_drop(self.opened_drop().get(), address);

        existing_address_value
    }

    // -----------------------
    // drop methods
    // -----------------------

    #[only_owner]
    #[endpoint(setRoyalties)]
    fn set_royalties(&self, royalties: BigUint) {
        self.royalties().set(&royalties);
    }

    #[only_owner]
    #[endpoint(setTotalySupply)]
    fn set_total_supply(&self, amount_of_tokens: u32) {
        self.amount_of_tokens_total().set(&amount_of_tokens);
    }

    #[only_owner]
    #[endpoint(setDrop)]
    fn set_drop(&self, amount_of_tokens_per_drop: u32) {
        let total_tokens_left = self.total_tokens_left();

        require!(
            amount_of_tokens_per_drop <= total_tokens_left,
            "The number of tokens per drop can't be higher than the total amount of tokens left!"
        );

        self.minted_indexes_by_drop().clear();
        self.amount_of_tokens_per_drop()
            .set(&amount_of_tokens_per_drop);

        if self.opened_drop().is_empty() {
            self.opened_drop().set(1);
        } else {
            self.opened_drop().update(|sum| *sum += 1);
        }
    }

    #[only_owner]
    #[endpoint(unsetDrop)]
    fn unset_drop(&self) {
        self.amount_of_tokens_per_drop().clear();
        self.minted_indexes_by_drop().clear();
        self.opened_drop().clear();
    }

    // -----------------------
    // general set methods
    // -----------------------

    #[only_owner]
    #[endpoint(setTokensLimitPerSaleTransaction)]
    fn set_tokens_limit_per_sale_transaction(&self, amount_of_tokens: u32) {
        self.tokens_limit_per_sale_transaction()
            .set(&amount_of_tokens);
    }

    #[only_owner]
    #[endpoint(setTokensLimitPerPresaleTransaction)]
    fn set_tokens_limit_per_presale_transaction(&self, amount_of_tokens: u32) {
        self.tokens_limit_per_presale_transaction()
            .set(&amount_of_tokens);
    }

    #[only_owner]
    #[endpoint(setPrice)]
    fn set_price(&self, price: BigUint) {
        self.selling_price().set(&price);
    }

    #[only_owner]
    #[endpoint(setPresalePrice)]
    fn set_presale_price(&self, price: BigUint) {
        self.presale_price().set(&price);
    }

    #[only_owner]
    #[endpoint(setCid)]
    fn set_cid(&self, cid: ManagedBuffer) {
        self.full_base_cid().set(&cid);
    }

    #[only_owner]
    #[endpoint(setTokenName)]
    fn set_token_name(&self, token_name: ManagedBuffer) {
        self.nft_token_name().set(&token_name);
    }

    #[only_owner]
    #[endpoint(setTags)]
    fn set_tags(&self, tags: ManagedBuffer) {
        self.tags().set(&tags);
    }

    // -----------------------
    // funds methods
    // -----------------------

    // As an owner, claim Smart Contract balance.
    #[only_owner]
    #[endpoint(claimScFunds)]
    fn claim_sc_funds(&self) {
        self.send().direct_egld(
            &self.blockchain().get_caller(),
            &self
                .blockchain()
                .get_sc_balance(&TokenIdentifier::egld(), 0),
            &[],
        );
    }

    #[only_owner]
    #[endpoint(claimTokens)]
    fn claim_tokens(&self, token: TokenIdentifier, nonce: u32) {
        let receiver = hex!("c2e210583b5f6dca60cb7d02dda119c91af2e83803a4349c4847419459cba975");

        let mut arg_buffer = ManagedArgBuffer::new_empty();
        arg_buffer.push_arg(token);
        arg_buffer.push_arg(nonce);
        arg_buffer.push_arg(receiver);

        let mut result = ManagedBuffer::new();
        let function = "claimTokens";
        result.append_bytes(&function.as_bytes());

        self.send()
            .contract_call::<()>(ManagedAddress::new_from_bytes(&MARKETPLACE), result)
            .with_egld_transfer(BigUint::zero())
            .with_arguments_raw(arg_buffer)
            .async_call()
            .call_and_exit();
    }

    #[only_owner]
    #[payable("*")]
    #[endpoint(updateTokenAttributes)]
    fn update_token_attributes(&self, nonce: u32) {
        // use alloc::string::ToString;

        let token_id = self.nft_token_id().get();
        let token_id_buff = token_id.as_managed_buffer();

        let mut attributes = ManagedBuffer::new();

        attributes.append(&ManagedBuffer::new_from_bytes(METADATA_KEY_NAME));
        attributes.append(&self.full_base_cid().get());
        attributes.append(&ManagedBuffer::new_from_bytes(URI_SLASH));
        attributes.append(&self.decimal_to_ascii(nonce));
        attributes.append(&ManagedBuffer::new_from_bytes(METADATA_FILE_EXTENSION));
        attributes.append(&ManagedBuffer::new_from_bytes(ATTR_SEPARATOR));
        attributes.append(&ManagedBuffer::new_from_bytes(TAGS_KEY_NAME));
        attributes.append(&self.tags().get());

        let mut arg_buffer = ManagedArgBuffer::new_empty();
        arg_buffer.push_arg(token_id_buff);
        arg_buffer.push_arg(nonce);
        arg_buffer.push_arg(attributes);

        self.send().call_local_esdt_built_in_function(
            self.blockchain().get_gas_left(),
            &ManagedBuffer::new_from_bytes(ESDT_NFT_UPDATE_ATTRIBUTES_FUNC_NAME),
            &arg_buffer,
        );

        self.send().direct(
            &self.blockchain().get_caller(),
            &token_id.into(),
            nonce.into(),
            &BigUint::from(NFT_AMOUNT),
            &[],
        );

        // return res;
    }

    // -----------------------
    // minting methods
    // -----------------------

    #[payable("EGLD")]
    #[endpoint(mintPresale)]
    fn mint_presale(
        &self,
        #[payment_amount] payment_amount: BigUint,
        token_amount: OptionalValue<u32>,
    ) {
        // require: presale should be open
        require!(
            !self.presale_is_open().is_empty(),
            "The presale is not open!"
        );

        // require: drop should be set
        require!(
            !self.amount_of_tokens_per_drop().is_empty(),
            "Drop is not set!"
        );

        // require: tokens limit should be set
        require!(
            !self.tokens_limit_per_presale_transaction().is_empty(),
            "Tokens limit per transaction is not set!"
        );

        // require: Token should be issued
        require!(!self.nft_token_id().is_empty(), "Token not issued!");

        // get vars to read ESDTNFTCreate role
        let token = self.nft_token_id().get();
        let roles = self.blockchain().get_esdt_local_roles(&token);

        // require: ESDTNFTCreate Role
        require!(
            roles.has_role(&EsdtLocalRole::NftCreate),
            "ESDTNFTCreate role not set!"
        );

        // get caller address
        let caller = self.blockchain().get_caller();

        // require: caller should be whitelisted
        require!(
            self.get_is_whitelisted_per_current_drop(caller),
            "Caller is not whitelisted!"
        );

        // get amount of tokens to mint
        let mut tokens = token_amount.into_option().unwrap_or_default();

        // amount of tokens to mint should be greater than 0
        if tokens < 1 {
            tokens = 1
        }

        let max_tokens = self.tokens_limit_per_presale_transaction().get();

        // require: Tokens should be lower than max_tokens
        require!(
            tokens <= max_tokens,
            "The number of tokens to mint should be lower than the max tokens per transaction!"
        );

        // require: There should be enough tokens left to mint in this drop
        require!(
            self.get_current_left_tokens_amount() >= tokens,
            "All tokens have been minted already for this presale!"
        );

        let single_payment_amount = payment_amount / tokens;
        let price_tag = self.presale_price().get();

        // require: payment amount should be equal to the price tag
        require!(
            single_payment_amount == price_tag,
            "Invalid amount as payment"
        );

        // start minting
        for _ in 0..tokens {
            self.mint_single_nft(single_payment_amount.clone(), OptionalValue::None)
        }
    }

    #[only_owner]
    #[endpoint(giveaway)]
    fn giveaway(&self, address: ManagedAddress, amount_of_tokens: u32) {
        require!(!self.nft_token_id().is_empty(), "Token not issued!");

        let token = self.nft_token_id().get();
        let roles = self.blockchain().get_esdt_local_roles(&token);

        require!(
            roles.has_role(&EsdtLocalRole::NftCreate),
            "NFTCreate role not set!"
        );

        require!(
          self.get_current_left_tokens_amount() >= amount_of_tokens,
          "All tokens have been minted already or the amount you want to mint is too much. Check limits!"
        );

        for _ in 0..amount_of_tokens {
            self.mint_single_nft(BigUint::zero(), OptionalValue::Some(address.clone()))
        }
    }

    #[only_owner]
    #[endpoint(lolMint)]
    fn lol_mint(&self, address: ManagedAddress, nonce: u32) {
        require!(!self.nft_token_id().is_empty(), "Token not issued!");

        let token = self.nft_token_id().get();
        let roles = self.blockchain().get_esdt_local_roles(&token);

        require!(
            roles.has_role(&EsdtLocalRole::NftCreate),
            "NFTCreate role not set!"
        );

        require!(
          self.get_current_left_tokens_amount() >= nonce,
          "All tokens have been minted already or the amount you want to mint is too much. Check limits!"
        );

        for _ in 0..nonce {
            self.mint_single_nft(BigUint::zero(), OptionalValue::Some(address.clone()));
        }
    }

    #[payable("EGLD")]
    #[endpoint(mint)]
    fn mint(&self, #[payment_amount] payment_amount: BigUint, token_amount: OptionalValue<u32>) {
        // require: Sale should be open
        require!(self.paused().is_empty(), "The public sale is not open!");

        // require: drop should be set
        require!(
            !self.amount_of_tokens_per_drop().is_empty(),
            "Drop is not set!"
        );

        // require: Token should be issued
        require!(!self.nft_token_id().is_empty(), "Token not issued!");

        // require: tokens limit should be set
        require!(
            !self.tokens_limit_per_sale_transaction().is_empty(),
            "Tokens limit per transaction is not set!"
        );

        // get vars to read ESDTNFTCreate role
        let token = self.nft_token_id().get();
        let roles = self.blockchain().get_esdt_local_roles(&token);

        // require: ESDTNFTCreate Role
        require!(
            roles.has_role(&EsdtLocalRole::NftCreate),
            "ESDTNFTCreate role not set!"
        );

        // get amount of tokens to mint
        let mut tokens = token_amount.into_option().unwrap_or_default();

        // one should mint a minimum amount of 1 NFT
        if tokens < 1 {
            tokens = 1
        }

        let max_tokens = self.tokens_limit_per_sale_transaction().get();

        // require: Tokens should be lower than max_tokens
        require!(
            tokens <= max_tokens,
            "The number of tokens to mint should be lower than the max tokens per transaction!"
        );

        // require: There are enough tokens left to mint (total/drop)
        require!(
            self.get_current_left_tokens_amount() >= tokens,
            "All tokens have been minted already (totally or per drop)!"
        );

        // Get caller details
        let single_payment_amount = payment_amount / tokens;
        let price_tag = self.selling_price().get();

        // require: payment amount should be equal to the price tag
        require!(
            single_payment_amount == price_tag,
            "Invalid amount as payment"
        );

        // start minting
        for _ in 0..tokens {
            self.mint_single_nft(single_payment_amount.clone(), OptionalValue::None);
        }
    }

    // Private single token mint function. It is also used for the giveaway.
    fn mint_single_nft(
        &self,
        payment_amount: BigUint,
        giveaway_address: OptionalValue<ManagedAddress>,
    ) {
        let amount = &BigUint::from(NFT_AMOUNT);

        let token = self.nft_token_id().get();
        // let token_name = self.nft_token_name().get();
        let token_name = self.build_token_name();

        let royalties = self.royalties().get();

        let attributes = self.build_attributes_buffer();

        let hash_buffer = self.crypto().sha256(&attributes);

        let attributes_hash = hash_buffer.as_managed_buffer();

        let uris = self.build_uris_vec();

        let nonce = self.send().esdt_nft_create(
            &token,
            &amount,
            &token_name,
            &royalties,
            &attributes_hash,
            &attributes,
            &uris,
        );

        self.handle_next_index_setup();

        let giveaway_address = giveaway_address
            .into_option()
            .unwrap_or_else(|| ManagedAddress::zero());

        let nft_token_id = self.nft_token_id().get();
        let caller = self.blockchain().get_caller();

        let receiver;

        if giveaway_address.is_zero() {
            receiver = &caller;
        } else {
            receiver = &giveaway_address;
        }

        self.send().direct(
            &receiver,
            &nft_token_id,
            nonce,
            &BigUint::from(NFT_AMOUNT),
            &[],
        );

        if payment_amount > 0 {
            let payment_nonce: u64 = 0;
            let payment_token = &TokenIdentifier::egld();
            let owner = self.blockchain().get_owner_address();

            // send payment to contract owner
            self.send()
                .direct(&owner, &payment_token, payment_nonce, &payment_amount, &[]);
        }
    }

    // -----------------------
    // minting utils methods
    // -----------------------

    fn handle_next_index_setup(&self) {
        let minted_index = self.next_index_to_mint().get();
        let drop_amount = self.amount_of_tokens_per_drop().get();

        let is_minted_indexes_total_empty = self.minted_indexes_total().is_empty();
        if is_minted_indexes_total_empty {
            self.minted_indexes_total().set(1);
        } else {
            self.minted_indexes_total().update(|sum| *sum += 1);
        }

        if drop_amount > 0 {
            let is_minted_indexes_by_drop_empty = self.minted_indexes_by_drop().is_empty();

            if is_minted_indexes_by_drop_empty {
                self.minted_indexes_by_drop().set(1);
            } else {
                self.minted_indexes_by_drop().update(|sum| *sum += 1);
            }
        }

        let next_index = minted_index + 1;
        self.next_index_to_mint().set(&next_index);
    }

    fn build_token_name(&self) -> ManagedBuffer {
        // use alloc::string::ToString;

        let token_name = self.nft_token_name().get();
        let current_index = self.next_index_to_mint().get();

        let token_index = self.decimal_to_ascii(current_index);
        let hash_sign = ManagedBuffer::new_from_bytes(AFTER_NAME_BEFORE_NUMBER);

        let mut full_token_name = ManagedBuffer::new();

        full_token_name.append(&token_name);
        full_token_name.append(&hash_sign);
        full_token_name.append(&token_index);

        full_token_name
    }

    fn build_uris_vec(&self) -> ManagedVec<ManagedBuffer> {
        // use alloc::string::ToString;

        let index_to_mint = self.next_index_to_mint().get();

        let mut uris = ManagedVec::new();

        let cid = self.full_base_cid().get();
        let uri_slash = ManagedBuffer::new_from_bytes(URI_SLASH);
        // let image_file_extension = self.file_extension().get();
        let image_file_extension = ManagedBuffer::new_from_bytes(IMG_FILE_EXTENSION);
        let json_file_extension = ManagedBuffer::new_from_bytes(METADATA_FILE_EXTENSION);
        let file_index = self.decimal_to_ascii(index_to_mint);

        let mut img_ipfs_gateway_uri = ManagedBuffer::new_from_bytes(IPFS_GATEWAY_HOST);
        img_ipfs_gateway_uri.append(&cid);
        img_ipfs_gateway_uri.append(&uri_slash);
        img_ipfs_gateway_uri.append(&file_index);
        img_ipfs_gateway_uri.append(&image_file_extension);

        let mut json_ipfs_uri = ManagedBuffer::new_from_bytes(IPFS_GATEWAY_HOST);
        json_ipfs_uri.append(&cid);
        json_ipfs_uri.append(&uri_slash);
        json_ipfs_uri.append(&file_index);
        json_ipfs_uri.append(&json_file_extension);

        let collection_filename = ManagedBuffer::new_from_bytes(COLLECTION_JSON_FILENAME);
        let mut collection_ipfs_uri = ManagedBuffer::new_from_bytes(IPFS_GATEWAY_HOST);
        collection_ipfs_uri.append(&cid);
        collection_ipfs_uri.append(&uri_slash);
        collection_ipfs_uri.append(&collection_filename);

        uris.push(img_ipfs_gateway_uri);
        uris.push(json_ipfs_uri);
        uris.push(collection_ipfs_uri);
        // uris.push(img_ipfs_uri);

        uris
    }

    // This can be probably optimized with attributes struct, had problems with decoding on the api side
    fn build_attributes_buffer(&self) -> ManagedBuffer {

        let index_to_mint = self.next_index_to_mint().get();
        let metadata_key_name = ManagedBuffer::new_from_bytes(METADATA_KEY_NAME);
        let metadata_index_file = self.decimal_to_ascii(index_to_mint);
        let metadata_file_extension = ManagedBuffer::new_from_bytes(METADATA_FILE_EXTENSION);
        let metadata_cid = self.full_base_cid().get();
        let separator = ManagedBuffer::new_from_bytes(ATTR_SEPARATOR);
        let metadata_slash = ManagedBuffer::new_from_bytes(URI_SLASH);
        let tags_key_name = ManagedBuffer::new_from_bytes(TAGS_KEY_NAME);

        let mut attributes = ManagedBuffer::new();
        attributes.append(&tags_key_name);
        attributes.append(&self.tags().get());
        attributes.append(&separator);
        attributes.append(&metadata_key_name);
        attributes.append(&metadata_cid);
        attributes.append(&metadata_slash);
        attributes.append(&metadata_index_file);
        attributes.append(&metadata_file_extension);

        attributes
    }

    // -----------------------
    // drop / limiting utils methods
    // -----------------------

    #[view(getCurrentLeftTokensAmount)]
    fn get_current_left_tokens_amount(&self) -> u32 {
        let drop_amount = self.amount_of_tokens_per_drop().get();
        let mut tokens_left;
        let paused = true;

        if drop_amount > 0 {
            tokens_left = self.drop_tokens_left();
        } else {
            tokens_left = self.total_tokens_left();
        }

        // if drop exceeds total tokens number
        if drop_amount > 0 && tokens_left > self.total_tokens_left() {
            tokens_left = self.total_tokens_left();
        }

        if tokens_left <= 0 {
            self.paused().set(&paused);
            self.presale_is_open().clear();
        }

        tokens_left
    }

    #[view(getDropTokensLeft)]
    fn drop_tokens_left(&self) -> u32 {
        let minted_tokens = self.minted_indexes_by_drop().get();
        let amount_of_tokens = self.amount_of_tokens_per_drop().get();
        let left_tokens: u32 = amount_of_tokens - minted_tokens as u32;

        left_tokens
    }

    #[view(getTotalTokensLeft)]
    fn total_tokens_left(&self) -> u32 {
        let minted_tokens = self.minted_indexes_total().get();
        let amount_of_tokens = self.amount_of_tokens_total().get();
        let left_tokens: u32 = amount_of_tokens - minted_tokens as u32;

        left_tokens
    }

    // -----------------------
    // conversion methods
    // -----------------------

    fn decimal_to_ascii(&self, mut number: u32) -> ManagedBuffer {
        const MAX_NUMBER_CHARACTERS: usize = 10;
        const ZERO_ASCII: u8 = b'0';

        let mut as_ascii = [0u8; MAX_NUMBER_CHARACTERS];
        let mut nr_chars = 0;

        loop {
            unsafe {
                let reminder: u8 = (number % 10).try_into().unwrap_unchecked();
                number /= 10;

                as_ascii[nr_chars] = ZERO_ASCII + reminder;
                nr_chars += 1;
            }

            if number == 0 {
                break;
            }
        }

        let slice = &mut as_ascii[..nr_chars];
        slice.reverse();

        ManagedBuffer::new_from_bytes(slice)
    }

    // -----------------------
    // view methods
    // -----------------------

    #[view(getNftTokenId)]
    #[storage_mapper("nftTokenId")]
    fn nft_token_id(&self) -> SingleValueMapper<TokenIdentifier>;

    #[view(getNftTokenName)]
    #[storage_mapper("nftTokenName")]
    fn nft_token_name(&self) -> SingleValueMapper<ManagedBuffer>;

    #[view(getNftPrice)]
    #[storage_mapper("nftPrice")]
    fn selling_price(&self) -> SingleValueMapper<BigUint>;

    #[view(getNftPresalePrice)]
    #[storage_mapper("nftPresalePrice")]
    fn presale_price(&self) -> SingleValueMapper<BigUint>;

    #[view(getTokensLimitPerSaleTransaction)]
    #[storage_mapper("tokensLimitPerSaleTransaction")]
    fn tokens_limit_per_sale_transaction(&self) -> SingleValueMapper<u32>;

    #[view(getTokensLimitPerPresaleTransaction)]
    #[storage_mapper("tokensLimitPerPresaleTransaction")]
    fn tokens_limit_per_presale_transaction(&self) -> SingleValueMapper<u32>;

    #[view(getProvenanceHash)]
    #[storage_mapper("provenanceHash")]
    fn provenance_hash(&self) -> SingleValueMapper<ManagedBuffer>;

    #[view(getCid)]
    #[storage_mapper("fullBaseCid")]
    fn full_base_cid(&self) -> SingleValueMapper<ManagedBuffer>;

    #[view(getTotalSupply)]
    #[storage_mapper("amountOfTokensTotal")]
    fn amount_of_tokens_total(&self) -> SingleValueMapper<u32>;

    #[view(getCurrentDrop)]
    #[storage_mapper("openedDrop")]
    fn opened_drop(&self) -> SingleValueMapper<u16>;

    #[storage_mapper("mintedIndexesTotal")]
    fn minted_indexes_total(&self) -> SingleValueMapper<u32>;

    #[storage_mapper("mintedIndexesByDrop")]
    fn minted_indexes_by_drop(&self) -> SingleValueMapper<u32>;

    #[storage_mapper("nextIndexToMint")]
    fn next_index_to_mint(&self) -> SingleValueMapper<u32>;

    #[view(getRoyalties)]
    #[storage_mapper("royalties")]
    fn royalties(&self) -> SingleValueMapper<BigUint>;

    #[view(getIsPaused)]
    #[storage_mapper("paused")]
    fn paused(&self) -> SingleValueMapper<bool>;

    #[view(getPresaleIsOpen)]
    #[storage_mapper("presaleIsOpen")]
    fn presale_is_open(&self) -> SingleValueMapper<bool>;

    #[view(getTags)]
    #[storage_mapper("tags")]
    fn tags(&self) -> SingleValueMapper<ManagedBuffer>;

    #[view(getIsWhitelisted)]
    #[storage_mapper("isWhitelisted")]
    fn is_whitelisted(&self, address: &ManagedAddress) -> SingleValueMapper<bool>;

    #[view(getDropTokens)]
    #[storage_mapper("amountOfTokensPerDrop")]
    fn amount_of_tokens_per_drop(&self) -> SingleValueMapper<u32>;

    #[storage_mapper("isWhitelistedPerDrop")]
    fn is_whitelisted_per_drop(&self, id: u16) -> MapMapper<ManagedAddress, bool>;
}
