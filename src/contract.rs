use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, SudoMsg};
use crate::state::{BRIDGE_CONTRACT, DATA};
#[cfg(not(feature = "library"))]
use cosmwasm_std::{entry_point, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use cosmwasm_std::{to_binary, Coin, Event, StdError, Uint128};
use cw2::{get_contract_version, set_contract_version};
use router_wasm_bindings::types::{ChainType, ContractCall, OutboundBatchRequest};
use router_wasm_bindings::RouterMsg;

// version info for migration info
const CONTRACT_NAME: &str = "hello-router-contract";
const CONTRACT_VERSION: &str = "0.1.0";

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    BRIDGE_CONTRACT.save(deps.storage, &msg.bridge_address)?;
    Ok(Response::new().add_attribute("action", "hello_router_init"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn sudo(deps: DepsMut, _env: Env, msg: SudoMsg) -> StdResult<Response<RouterMsg>> {
    match msg {
        SudoMsg::HandleInboundReq {
            sender,
            chain_type,
            source_chain_id,
            payload,
        } => handle_in_bound_request(deps, sender, chain_type, source_chain_id, payload),
        SudoMsg::HandleOutboundAck {
            outbound_tx_requested_by,
            destination_chain_type,
            destination_chain_id,
            outbound_batch_nonce,
            contract_ack_responses,
        } => handle_out_bound_ack_request(
            deps,
            outbound_tx_requested_by,
            destination_chain_type,
            destination_chain_id,
            outbound_batch_nonce,
            contract_ack_responses,
        ),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: ExecuteMsg,
) -> StdResult<Response<RouterMsg>> {
    match msg {
        ExecuteMsg::UpdateBridgeContract { address, payload } => {
            update_bridge_contract(deps, address, payload.0)
        }
    }
}

fn handle_in_bound_request(
    deps: DepsMut,
    sender: String,
    chain_type: u32,
    src_chain_id: String,
    payload: Binary,
) -> StdResult<Response<RouterMsg>> {
    let payload_string: Vec<u8> = base64::decode(payload.to_string()).unwrap();
    let string: String = String::from_utf8(payload_string).unwrap();
    let reverse_string: String = string.chars().rev().collect::<String>();
    DATA.save(deps.storage, &reverse_string)?;
    let event = Event::new("in_bound_request")
        .add_attribute("sender", sender.to_string())
        .add_attribute("chain_type", chain_type.to_string())
        .add_attribute("src_chain_id", src_chain_id.clone())
        .add_attribute("payload", reverse_string.clone());

    let bridge_address: String = fetch_bridge_address(deps.as_ref())?;
    let contract_call: ContractCall = ContractCall {
        destination_contract_address: bridge_address.into_bytes(),
        payload: reverse_string.into_bytes(),
    };
    let outbound_batch_req: OutboundBatchRequest = OutboundBatchRequest {
        destination_chain_type: ChainType::ChainTypeEvm.get_chain_code(),
        destination_chain_id: String::from("137"),
        contract_calls: vec![contract_call],
        relayer_fee: Coin {
            denom: String::from("router"),
            amount: Uint128::new(8u128),
        },
        outgoing_tx_fee: Coin {
            denom: String::from("router"),
            amount: Uint128::new(8u128),
        },
        is_atomic: false,
    };
    let outbound_batch_reqs: RouterMsg = RouterMsg::OutboundBatchRequests {
        outbound_batch_requests: vec![outbound_batch_req],
        is_sequenced: false,
    };

    let res = Response::new()
        .add_message(outbound_batch_reqs)
        .add_event(event)
        .add_attribute("sender", sender)
        .add_attribute("chain_type", chain_type.to_string())
        .add_attribute("src_chain_id", src_chain_id);
    Ok(res)
}

fn handle_out_bound_ack_request(
    _deps: DepsMut,
    sender: String,
    destination_chain_type: u64,
    destination_chain_id: String,
    outbound_batch_nonce: u64,
    contract_ack_responses: Binary,
) -> StdResult<Response<RouterMsg>> {
    // let mut ack_status_key: String = destination_chain_id.clone();
    // ack_status_key.push_str(&destination_chain_type.to_string());
    // ack_status_key.push_str(&outbound_batch_nonce.to_string());

    // ACK_STATUS.save(deps.storage, &ack_status_key, &contract_ack_responses.0)?;
    let res = Response::new()
        .add_attribute("sender", sender)
        .add_attribute("destination_chain_type", destination_chain_type.to_string())
        .add_attribute("destination_chain_id", destination_chain_id)
        .add_attribute("outbound_batch_nonce", outbound_batch_nonce.to_string())
        .add_attribute("contract_ack_responses", contract_ack_responses.to_string());
    Ok(res)
}

fn update_bridge_contract(
    deps: DepsMut,
    address: String,
    payload: Vec<u8>,
) -> StdResult<Response<RouterMsg>> {
    BRIDGE_CONTRACT.save(deps.storage, &address)?;

    let contract_call: ContractCall = ContractCall {
        destination_contract_address: address.clone().into_bytes(),
        payload,
    };
    let outbound_batch_req: OutboundBatchRequest = OutboundBatchRequest {
        destination_chain_type: ChainType::ChainTypeEvm.get_chain_code(),
        destination_chain_id: String::from("137"),
        contract_calls: vec![contract_call],
        relayer_fee: Coin {
            denom: String::from("router"),
            amount: Uint128::new(8u128),
        },
        outgoing_tx_fee: Coin {
            denom: String::from("router"),
            amount: Uint128::new(8u128),
        },
        is_atomic: false,
    };
    let outbound_batch_reqs: RouterMsg = RouterMsg::OutboundBatchRequests {
        outbound_batch_requests: vec![outbound_batch_req],
        is_sequenced: false,
    };

    let res = Response::new()
        .add_message(outbound_batch_reqs)
        .add_attribute("bridge_address", address);
    Ok(res)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    let ver = cw2::get_contract_version(deps.storage)?;
    // ensure we are migrating from an allowed contract
    if ver.contract != CONTRACT_NAME.to_string() {
        return Err(StdError::generic_err("Can only upgrade from same type").into());
    }
    // note: better to do proper semver compare, but string compare *usually* works
    if ver.version >= CONTRACT_VERSION.to_string() {
        return Err(StdError::generic_err("Cannot upgrade from a newer version").into());
    }

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetContractVersion {} => to_binary(&get_contract_version(deps.storage)?),
        QueryMsg::FetchData {} => to_binary(&fetch_data(deps)?),
        QueryMsg::FetchBridgeAddress {} => to_binary(&get_contract_version(deps.storage)?),
    }
}

pub fn fetch_data(deps: Deps) -> StdResult<String> {
    return Ok(DATA.load(deps.storage)?);
}

pub fn fetch_bridge_address(deps: Deps) -> StdResult<String> {
    return Ok(BRIDGE_CONTRACT.load(deps.storage)?);
}
