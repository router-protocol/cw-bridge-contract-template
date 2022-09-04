use crate::contract::{execute, fetch_data, sudo};
use crate::msg::{ExecuteMsg, InstantiateMsg};
use crate::{contract::instantiate, msg::SudoMsg};
use cosmwasm_std::{
    testing::{mock_dependencies, mock_env, mock_info},
    DepsMut,
};
use cosmwasm_std::{Binary, CosmosMsg};
use router_wasm_bindings::types::OutboundBatchRequest;
use router_wasm_bindings::RouterMsg;

const INIT_ADDRESS: &str = "init_address";
const BRIDGE_ADDRESS: &str = "bridge_address";

fn do_instantiate(mut deps: DepsMut) {
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
    let mut deps = mock_dependencies();
    do_instantiate(deps.as_mut());
}

#[test]
fn test_sudo_function() {
    let mut deps = mock_dependencies();
    do_instantiate(deps.as_mut());
    let env = mock_env();
    let test_string: String = String::from("123");
    let encoded_string: String = base64::encode(test_string.clone());
    let msg: SudoMsg = SudoMsg::HandleInboundReq {
        sender: String::from(""),
        chain_type: 1,
        source_chain_id: String::from("323"),
        payload: Binary::from_base64(&encoded_string).unwrap(),
    };

    let response = sudo(deps.as_mut(), env, msg).unwrap();
    assert_eq!(response.messages.len(), 1);

    let data: String = fetch_data(deps.as_ref()).unwrap();
    assert_eq!(data, String::from("321"));

    let message = response.messages.get(0).unwrap();
    let router_msg = message.msg.clone();
    match router_msg {
        CosmosMsg::Custom(msg) => {
            println!("{:?}", msg);
            match msg {
                RouterMsg::OutboundBatchRequests {
                    outbound_batch_requests,
                    is_sequenced,
                } => {
                    assert_eq!(is_sequenced, false);
                    assert_eq!(outbound_batch_requests.len(), 1);
                    let request: OutboundBatchRequest = outbound_batch_requests[0].clone();
                    let contract: Vec<u8> = request.contract_calls[0]
                        .destination_contract_address
                        .clone();
                    assert_eq!(contract, String::from(BRIDGE_ADDRESS).into_bytes());
                }
            }
        }
        _ => {}
    }
}

#[test]
fn test_execute_update_bridge_address() {
    let mut deps = mock_dependencies();
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
        CosmosMsg::Custom(msg) => {
            println!("{:?}", msg);
            match msg {
                RouterMsg::OutboundBatchRequests {
                    outbound_batch_requests,
                    is_sequenced,
                } => {
                    assert_eq!(is_sequenced, false);
                    assert_eq!(outbound_batch_requests.len(), 1);
                    let request: OutboundBatchRequest = outbound_batch_requests[0].clone();
                    let contract: Vec<u8> = request.contract_calls[0]
                        .destination_contract_address
                        .clone();
                    assert_eq!(contract, String::from(BRIDGE_ADDRESS).into_bytes());
                }
            }
        }
        _ => {}
    }
}
