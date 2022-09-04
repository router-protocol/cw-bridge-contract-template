use cw_storage_plus::Item;

pub const DATA: Item<String> = Item::new("data_string");

pub const BRIDGE_CONTRACT: Item<String> = Item::new("bridge_contract");
