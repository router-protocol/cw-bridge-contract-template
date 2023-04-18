use crate::contract::{execute, fetch_data, sudo};
use crate::msg::{ExecuteMsg, InstantiateMsg};
use crate::{contract::instantiate, msg::SudoMsg};
use cosmwasm_std::testing::{MockApi, MockQuerier, MockStorage};
use cosmwasm_std::{
    testing::{mock_env, mock_info},
    DepsMut,
};
use cosmwasm_std::{Binary, CosmosMsg, OwnedDeps, Uint128};
use router_wasm_bindings::types::ChainType;
use router_wasm_bindings::{RouterMsg, RouterQuery};
use std::marker::PhantomData;

const INIT_ADDRESS: &str = "router1apapk9zfz3rp4x87fsm6h0s3zd0wlmkz0fx8tx";
const BRIDGE_ADDRESS: &str = "0xeedb3ab68d567a6cd6d19fa819fe77b9f8ed1538";

fn get_mock_dependencies() -> OwnedDeps<MockStorage, MockApi, MockQuerier, RouterQuery> {
    OwnedDeps {
        storage: MockStorage::default(),
        api: MockApi::default(),
        querier: MockQuerier::default(),
        custom_query_type: PhantomData,
    }
}

fn do_instantiate(mut deps: DepsMut<RouterQuery>) {
    let instantiate_msg = InstantiateMsg {
        bridge_address: String::from(BRIDGE_ADDRESS),
    };
    let info = mock_info(INIT_ADDRESS, &[]);
    let env = mock_env();
    let res = instantiate(deps.branch(), env, info, instantiate_msg).unwrap();
    assert_eq!(0, res.messages.len());
}

#[test]
fn test_basic() {
    let mut deps = get_mock_dependencies();
    do_instantiate(deps.as_mut());
}

#[test]
fn test_sudo_function() {
    let mut deps = get_mock_dependencies();
    do_instantiate(deps.as_mut());
    let env = mock_env();
    let set_chain_type_msg: ExecuteMsg = ExecuteMsg::SetChainType {
        chain_id: "80001".to_string(),
        chain_type: ChainType::ChainTypeEvm.get_chain_code(),
    };
    let info = mock_info(INIT_ADDRESS, &[]);
    execute(deps.as_mut(), env.clone(), info.clone(), set_chain_type_msg).unwrap();

    let test_string: String = String::from("80001");
    let encoded_string: String = base64::encode(test_string.clone());
    let msg: SudoMsg = SudoMsg::HandleIReceive {
        request_sender: Binary::from_base64("97MRmF0DyXm9Sfa9szch6J9ie6U=").unwrap(),
        src_chain_id: String::from("80001"),
        request_identifier: 2,
        payload: Binary::from_base64(&encoded_string).unwrap(),
    };

    let response = sudo(deps.as_mut(), env, msg).unwrap();
    assert_eq!(response.messages.len(), 1);

    let data: String = fetch_data(deps.as_ref()).unwrap();
    assert_eq!(data, String::from("10008"));

    let message = response.messages.get(0).unwrap();
    let router_msg = message.msg.clone();
    match router_msg {
        CosmosMsg::Custom(msg) => match msg {
            RouterMsg::CrosschainCall {
                version,
                route_amount,
                route_recipient,
                dest_chain_id,
                request_metadata,
                request_packet,
            } => {
                assert_eq!(route_amount, Uint128::zero());
                assert_eq!(hex::encode(route_recipient), "");

                assert_eq!(dest_chain_id, "80001");
                assert_eq!(version, 1);
                assert_eq!(hex::encode(request_metadata), "000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000493e0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000493e0000000000000000000000000000000000000000000000000000000000098968000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000003000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001000000000000000000000000000000000000000000000000000000000000000000");
                assert_eq!(hex::encode(request_packet), "000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000000c000000000000000000000000000000000000000000000000000000000000000600000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000000531303030380000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000014eedb3ab68d567a6cd6d19fa819fe77b9f8ed1538000000000000000000000000");
            }
        },
        _ => {}
    }
}

#[test]
fn test_execute_update_bridge_address() {
    let mut deps = get_mock_dependencies();
    do_instantiate(deps.as_mut());
    let env = mock_env();
    let test_string: String = String::from("123");
    let encoded_string: String = base64::encode(test_string.clone());
    let msg: ExecuteMsg = ExecuteMsg::UpdateBridgeContract {
        address: String::from(BRIDGE_ADDRESS),
        payload: Binary::from_base64(&encoded_string).unwrap(),
    };
    let info = mock_info(INIT_ADDRESS, &[]);
    let response = execute(deps.as_mut(), env, info, msg).unwrap();
    assert_eq!(response.messages.len(), 1);

    let message = response.messages.get(0).unwrap();
    let router_msg = message.msg.clone();
    match router_msg {
        CosmosMsg::Custom(msg) => match msg {
            RouterMsg::CrosschainCall {
                version,
                route_amount,
                route_recipient,
                dest_chain_id,
                request_metadata,
                request_packet,
            } => {
                assert_eq!(route_amount, Uint128::zero());
                assert_eq!(hex::encode(route_recipient), "");

                assert_eq!(dest_chain_id, "80001");
                assert_eq!(version, 1);
                assert_eq!(hex::encode(request_metadata), "000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000493e0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000493e0000000000000000000000000000000000000000000000000000000000098968000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000003000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001000000000000000000000000000000000000000000000000000000000000000000");
                assert_eq!(hex::encode(request_packet), "00000000000000000000000000000000000000000000000000000000000000400000000000000000000000000000000000000000000000000000000000000080000000000000000000000000000000000000000000000000000000000000000331323300000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000014eedb3ab68d567a6cd6d19fa819fe77b9f8ed1538000000000000000000000000");
            }
        },
        _ => {}
    }
}
