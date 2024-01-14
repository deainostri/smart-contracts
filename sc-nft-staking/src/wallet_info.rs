elrond_wasm::imports!();
elrond_wasm::derive_imports!();

#[derive(
    TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode, PartialEq, ManagedVecItem, Clone,
)]
pub struct WalletInfo<M: ManagedTypeApi> {
    //
    pub address: ManagedAddress<M>,
    pub nfts: ManagedVec<M, u64>,

    pub points: u64,

    pub claimed: BigUint<M>,
    pub claimeable: BigUint<M>,
}
