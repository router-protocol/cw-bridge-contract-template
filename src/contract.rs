use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, SudoMsg};
use crate::state::{BRIDGE_CONTRACT, CHAIN_TYPE_MAPPING, DATA};
#[cfg(not(feature = "library"))]
use cosmwasm_std::{entry_point, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use cosmwasm_std::{to_binary, Coin, Event, StdError, Uint128};
use cw2::{get_contract_version, set_contract_version};
use router_wasm_bindings::ethabi::{encode, Token};
use router_wasm_bindings::types::{AckType, ChainType, GasPriceResponse, RequestMetaData};
use router_wasm_bindings::utils::{
    convert_address_from_bytes_to_string, convert_address_from_string_to_bytes,
};
use router_wasm_bindings::{Bytes, RouterMsg, RouterQuerier, RouterQuery};

// version info for migration info
const CONTRACT_NAME: &str = "hello-router-contract";
const CONTRACT_VERSION: &str = "0.1.0";

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut<RouterQuery>,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    BRIDGE_CONTRACT.save(deps.storage, &msg.bridge_address)?;
    Ok(Response::new().add_attribute("action", "hello_router_init"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn sudo(deps: DepsMut<RouterQuery>, env: Env, msg: SudoMsg) -> StdResult<Response<RouterMsg>> {
    match msg {
        SudoMsg::HandleIReceive {
            request_sender,
            src_chain_id,
            request_identifier,
            payload,
        } => handle_sudo_request(
            deps,
            env,
            request_sender,
            src_chain_id,
            request_identifier,
            payload,
        ),
        SudoMsg::HandleIAck {
            request_identifier,
            exec_flag,
            exec_data,
            refund_amount,
        } => handle_sudo_ack(
            deps,
            env,
            request_identifier,
            exec_flag,
            exec_data,
            refund_amount,
        ),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut<RouterQuery>,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> StdResult<Response<RouterMsg>> {
    match msg {
        ExecuteMsg::UpdateBridgeContract { address, payload } => {
            update_bridge_contract(deps, address, payload.0)
        }
        ExecuteMsg::SetChainType {
            chain_id,
            chain_type,
        } => set_chain_type(deps, env, info, chain_id, chain_type),
    }
}

pub fn handle_sudo_request(
    deps: DepsMut<RouterQuery>,
    _env: Env,
    request_sender: Binary,
    src_chain_id: String,
    request_identifier: u64,
    payload: Binary,
) -> StdResult<Response<RouterMsg>> {
    let src_chain_type: u64 = CHAIN_TYPE_MAPPING
        .load(deps.storage, &src_chain_id)
        .unwrap();
    let sender: String =
        match convert_address_from_bytes_to_string(&request_sender.0, src_chain_type) {
            Ok(address) => address,
            Err(err) => return Err(err),
        };
    let payload_string: Vec<u8> = base64::decode(payload.to_string()).unwrap();
    let string: String = String::from_utf8(payload_string).unwrap();
    let reverse_string: String = string.chars().rev().collect::<String>();
    DATA.save(deps.storage, &reverse_string)?;

    let dest_chain_id: String = String::from("80001");
    let dest_chain_type: u64 = ChainType::ChainTypeEvm.get_chain_code();
    let event = Event::new("in_bound_request")
        .add_attribute("sender", sender.to_string())
        .add_attribute("src_chain_type", src_chain_type.to_string())
        .add_attribute("src_chain_id", src_chain_id.clone())
        .add_attribute("request_identifier", request_identifier.to_string())
        .add_attribute("payload", reverse_string.clone());

    let bridge_address: String = fetch_bridge_address(deps.as_ref())?;
    let payload: Bytes = encode(&[Token::String(reverse_string)]);
    let dest_contract_slice: Bytes =
        convert_address_from_string_to_bytes(bridge_address, dest_chain_type)?;

    let gas_price: u64 = match fetch_oracle_gas_price(deps.as_ref(), dest_chain_id.clone()) {
        Ok(res) => res.gas_price,
        _ => 0,
    };

    let request_packet: Bytes = encode(&[Token::Bytes(payload), Token::Bytes(dest_contract_slice)]);
    let request_metadata: RequestMetaData = RequestMetaData {
        dest_gas_limit: 300_000,
        dest_gas_price: gas_price,
        ack_gas_limit: 300_000,
        ack_gas_price: 10_000_000,
        relayer_fee: Uint128::zero(),
        ack_type: AckType::AckOnBoth,
        is_read_call: false,
        asm_address: vec![],
    };

    let i_send_request: RouterMsg = RouterMsg::CrosschainCall {
        version: 1,
        route_amount: Uint128::new(0u128),
        route_recipient: vec![],
        dest_chain_id,
        request_metadata: request_metadata.get_abi_encoded_bytes(),
        request_packet,
    };
    let res = Response::new()
        .add_message(i_send_request)
        .add_event(event)
        .add_attribute("sender", sender)
        .add_attribute("src_chain_type", src_chain_type.to_string())
        .add_attribute("src_chain_id", src_chain_id);
    Ok(res)
}

fn handle_sudo_ack(
    deps: DepsMut<RouterQuery>,
    _env: Env,
    request_identifier: u64,
    exec_flag: bool,
    exec_data: Binary,
    refund_amount: Coin,
) -> StdResult<Response<RouterMsg>> {
    let execution_msg: String = format!(
        "request_identifier {:?}, refund_amount {:?}, exec_flag {:?}, exec_data {:?}",
        request_identifier, refund_amount, exec_flag, exec_data
    );
    deps.api.debug(&execution_msg);
    let res = Response::new().add_attribute("request_identifier", request_identifier.to_string());
    Ok(res)
}

fn update_bridge_contract(
    deps: DepsMut<RouterQuery>,
    address: String,
    payload: Vec<u8>,
) -> StdResult<Response<RouterMsg>> {
    BRIDGE_CONTRACT.save(deps.storage, &address)?;

    let dest_chain_id: String = String::from("80001");
    let dest_chain_type: u64 = ChainType::ChainTypeEvm.get_chain_code();
    let dest_contract_slice: Bytes =
        convert_address_from_string_to_bytes(address.clone(), dest_chain_type)?;

    let gas_price: u64 = match fetch_oracle_gas_price(deps.as_ref(), dest_chain_id.clone()) {
        Ok(res) => res.gas_price,
        _ => 0,
    };

    let request_packet: Bytes = encode(&[Token::Bytes(payload), Token::Bytes(dest_contract_slice)]);
    let request_metadata: RequestMetaData = RequestMetaData {
        dest_gas_limit: 300_000,
        dest_gas_price: gas_price,
        ack_gas_limit: 300_000,
        ack_gas_price: 10_000_000,
        relayer_fee: Uint128::zero(),
        ack_type: AckType::AckOnBoth,
        is_read_call: false,
        asm_address: vec![],
    };

    let i_send_request: RouterMsg = RouterMsg::CrosschainCall {
        version: 1,
        route_amount: Uint128::new(0u128),
        route_recipient: vec![],
        dest_chain_id,
        request_metadata: request_metadata.get_abi_encoded_bytes(),
        request_packet,
    };

    let res = Response::new()
        .add_message(i_send_request)
        .add_attribute("bridge_address", address);
    Ok(res)
}

fn set_chain_type(
    deps: DepsMut<RouterQuery>,
    _env: Env,
    _info: MessageInfo,
    chain_id: String,
    chain_type: u64,
) -> StdResult<Response<RouterMsg>> {
    CHAIN_TYPE_MAPPING.save(deps.storage, &chain_id, &chain_type)?;
    let res = Response::new()
        .add_attribute("action", "SetChainType")
        .add_attribute("chain_id", chain_id)
        .add_attribute("chain_type", chain_type.to_string());
    Ok(res)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut<RouterQuery>, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
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
pub fn query(deps: Deps<RouterQuery>, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetContractVersion {} => to_binary(&get_contract_version(deps.storage)?),
        QueryMsg::FetchData {} => to_binary(&fetch_data(deps)?),
        QueryMsg::FetchBridgeAddress {} => to_binary(&get_contract_version(deps.storage)?),
        QueryMsg::FetchChainType { chain_id } => to_binary(&fetch_chain_type(deps, &chain_id)?),
    }
}

pub fn fetch_data(deps: Deps<RouterQuery>) -> StdResult<String> {
    return Ok(DATA.load(deps.storage)?);
}

pub fn fetch_bridge_address(deps: Deps<RouterQuery>) -> StdResult<String> {
    return Ok(BRIDGE_CONTRACT.load(deps.storage)?);
}

pub fn fetch_oracle_gas_price(
    deps: Deps<RouterQuery>,
    chain_id: String,
) -> StdResult<GasPriceResponse> {
    // let query_wrapper: QuerierWrapper = QuerierWrapper::new(&deps.querier);
    let router_querier: RouterQuerier = RouterQuerier::new(&deps.querier);
    router_querier.gas_price(chain_id)
}

pub fn fetch_chain_type(deps: Deps<RouterQuery>, chain_id: &str) -> StdResult<u64> {
    CHAIN_TYPE_MAPPING.load(deps.storage, chain_id)
}
