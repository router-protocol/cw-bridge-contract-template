use cw_storage_plus::{Item, Map};

pub const DATA: Item<String> = Item::new("data_string");

pub const BRIDGE_CONTRACT: Item<String> = Item::new("bridge_contract");

pub const CHAIN_TYPE_MAPPING: Map<&str, u64> = Map::new("chain_type_mapping");
