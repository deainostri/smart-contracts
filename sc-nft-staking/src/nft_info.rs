elrond_wasm::imports!();
elrond_wasm::derive_imports!();

#[derive(
    TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode, PartialEq, ManagedVecItem, Clone,
)]
pub struct NftInfo<M: ManagedTypeApi> {
    //
    pub owner: ManagedAddress<M>,
    pub staked_at: u64,
}
