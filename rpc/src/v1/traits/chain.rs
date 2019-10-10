// Copyright 2018-2019 Kodebox, Inc.
// This file is part of CodeChain.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use cjson::scheme::Params;
use cjson::uint::Uint;
use ckey::{NetworkId, PlatformAddress, Public};
use ctypes::{BlockNumber, ShardId};
use primitives::{Bytes as BytesArray, H160, H256};

use jsonrpc_core::Result;

use super::super::types::{AssetScheme, Block, BlockNumberAndHash, OwnedAsset, Text, Transaction, UnsignedTransaction};

#[rpc(server)]
pub trait Chain {
    /// Gets transaction with given hash.
    #[rpc(name = "chain_getTransaction")]
    fn get_transaction(&self, transaction_hash: H256) -> Result<Option<Transaction>>;

    /// Gets the signer of transaction with given hash.
    #[rpc(name = "chain_getTransactionSigner")]
    fn get_transaction_signer(&self, transaction_hash: H256) -> Result<Option<PlatformAddress>>;

    /// Query whether the chain has the transaction with given transaction hash.
    #[rpc(name = "chain_containsTransaction")]
    fn contains_transaction(&self, transaction_hash: H256) -> Result<bool>;

    #[rpc(name = "chain_containTransaction")]
    fn contain_transaction(&self, transaction_hash: H256) -> Result<bool>;

    /// Gets transaction with given transaction tracker.
    #[rpc(name = "chain_getTransactionByTracker")]
    fn get_transaction_by_tracker(&self, tracker: H256) -> Result<Option<Transaction>>;

    /// Gets asset scheme with given transaction tracker.
    #[rpc(name = "chain_getAssetSchemeByTracker")]
    fn get_asset_scheme_by_tracker(
        &self,
        tracker: H256,
        shard_id: ShardId,
        block_number: Option<u64>,
    ) -> Result<Option<AssetScheme>>;

    /// Gets asset scheme with given asset type.
    #[rpc(name = "chain_getAssetSchemeByType")]
    fn get_asset_scheme_by_type(
        &self,
        asset_type: H160,
        shard_id: ShardId,
        block_number: Option<u64>,
    ) -> Result<Option<AssetScheme>>;

    /// Gets text with given transaction hash.
    #[rpc(name = "chain_getText")]
    fn get_text(&self, transaction_hash: H256, block_number: Option<u64>) -> Result<Option<Text>>;

    /// Gets asset with given asset type.
    #[rpc(name = "chain_getAsset")]
    fn get_asset(
        &self,
        tracker: H256,
        index: usize,
        shard_id: ShardId,
        block_number: Option<u64>,
    ) -> Result<Option<OwnedAsset>>;

    /// Checks whether an asset is spent or not.
    #[rpc(name = "chain_isAssetSpent")]
    fn is_asset_spent(
        &self,
        transaction_hash: H256,
        index: usize,
        shard_id: ShardId,
        block_number: Option<u64>,
    ) -> Result<Option<bool>>;

    /// Gets seq with given account.
    #[rpc(name = "chain_getSeq")]
    fn get_seq(&self, address: PlatformAddress, block_number: Option<u64>) -> Result<Option<u64>>;

    /// Gets balance with given account.
    #[rpc(name = "chain_getBalance")]
    fn get_balance(&self, address: PlatformAddress, block_number: Option<u64>) -> Result<Option<Uint>>;

    /// Gets regular key with given account
    #[rpc(name = "chain_getRegularKey")]
    fn get_regular_key(&self, address: PlatformAddress, block_number: Option<u64>) -> Result<Option<Public>>;

    /// Gets the owner of given regular key.
    #[rpc(name = "chain_getRegularKeyOwner")]
    fn get_regular_key_owner(&self, public: Public, block_number: Option<u64>) -> Result<Option<PlatformAddress>>;

    /// Gets the genesis accounts
    #[rpc(name = "chain_getGenesisAccounts")]
    fn get_genesis_accounts(&self) -> Result<Vec<PlatformAddress>>;

    /// Gets the number of shards
    #[rpc(name = "chain_getNumberOfShards")]
    fn get_number_of_shards(&self, block_number: Option<u64>) -> Result<Option<ShardId>>;

    /// Gets shard id
    #[rpc(name = "chain_getShardIdByHash")]
    fn get_shard_id_by_hash(&self, create_shard_tx_hash: H256, block_number: Option<u64>) -> Result<Option<ShardId>>;

    /// Gets shard root
    #[rpc(name = "chain_getShardRoot")]
    fn get_shard_root(&self, shard_id: ShardId, block_number: Option<u64>) -> Result<Option<H256>>;

    /// Gets shard owners
    #[rpc(name = "chain_getShardOwners")]
    fn get_shard_owners(&self, shard_id: ShardId, block_number: Option<u64>) -> Result<Option<Vec<PlatformAddress>>>;

    /// Gets shard users
    #[rpc(name = "chain_getShardUsers")]
    fn get_shard_users(&self, shard_id: ShardId, block_number: Option<u64>) -> Result<Option<Vec<PlatformAddress>>>;

    /// Gets number of best block.
    #[rpc(name = "chain_getBestBlockNumber")]
    fn get_best_block_number(&self) -> Result<BlockNumber>;

    /// Gets the number and the hash of the best block.
    #[rpc(name = "chain_getBestBlockId")]
    fn get_best_block_id(&self) -> Result<BlockNumberAndHash>;

    /// Gets the hash of the block with given number.
    #[rpc(name = "chain_getBlockHash")]
    fn get_block_hash(&self, block_number: u64) -> Result<Option<H256>>;

    /// Gets block with given number.
    #[rpc(name = "chain_getBlockByNumber")]
    fn get_block_by_number(&self, block_number: u64) -> Result<Option<Block>>;

    /// Gets block with given hash.
    #[rpc(name = "chain_getBlockByHash")]
    fn get_block_by_hash(&self, block_hash: H256) -> Result<Option<Block>>;

    ///Gets the count of transactions in a block with given hash.
    #[rpc(name = "chain_getBlockTransactionCountByHash")]
    fn get_block_transaction_count_by_hash(&self, block_hash: H256) -> Result<Option<usize>>;

    ///Gets the minimum transaction fee of the given name.
    #[rpc(name = "chain_getMinTransactionFee")]
    fn get_min_transaction_fee(&self, action_type: String, block_number: Option<u64>) -> Result<Option<u64>>;

    /// Gets the mining given block number
    #[rpc(name = "chain_getMiningReward")]
    fn get_mining_reward(&self, block_number: u64) -> Result<Option<u64>>;

    /// Return the network id that is used in this chain.
    #[rpc(name = "chain_getNetworkId")]
    fn get_network_id(&self) -> Result<NetworkId>;

    /// Return common params at given block number
    #[rpc(name = "chain_getCommonParams")]
    fn get_common_params(&self, block_number: Option<u64>) -> Result<Option<Params>>;

    /// Return the current term id at given block number
    #[rpc(name = "chain_getTermMetadata")]
    fn get_term_metadata(&self, block_number: Option<u64>) -> Result<Option<(u64, u64)>>;

    /// Return the current metadata seq at given block number
    #[rpc(name = "chain_getMetadataSeq")]
    fn get_metadata_seq(&self, block_number: Option<u64>) -> Result<Option<u64>>;

    /// Return the valid block authors
    #[rpc(name = "chain_getPossibleAuthors")]
    fn get_possible_authors(&self, block_number: Option<u64>) -> Result<Option<Vec<PlatformAddress>>>;

    /// Execute Transactions
    #[rpc(name = "chain_executeTransaction")]
    fn execute_transaction(&self, tx: UnsignedTransaction, sender: PlatformAddress) -> Result<Option<String>>;

    /// Execute AssetTransfer transaction inputs in VM
    #[rpc(name = "chain_executeVM")]
    fn execute_vm(
        &self,
        tx: UnsignedTransaction,
        params: Vec<Vec<BytesArray>>,
        indices: Vec<usize>,
    ) -> Result<Vec<String>>;
}
