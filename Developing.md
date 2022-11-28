# Developing

If you have recently created a contract with this template, you probably could use some
help on how to build and test the contract, as well as prepare it for production. This
file attempts to provide a brief overview, assuming you have installed a recent
version of Rust already (eg. 1.58.1+).

## Prerequisites

Before starting, make sure you have [rustup](https://rustup.rs/) along with a
recent `rustc` and `cargo` version installed. Currently, we are testing on 1.58.1+.

And you need to have the `wasm32-unknown-unknown` target installed as well.

You can check that via:

```sh
rustc --version
cargo --version
rustup target list --installed
# if wasm32 is not listed above, run this
rustup target add wasm32-unknown-unknown
```

## Writing contracts that interact with Router

```sh
# add the following line in the cargo.toml [dependencies] section
router-wasm-bindings = "0.1.5"
```

To implement cross-chain interoperability, the contract needs to implement the following functionality
 - **SudoMsg** for handling incoming requests from the other chains
 - **RouterMsg** to send request to the other chains.

The Contract can write the intermediate business logic inbetween of the incoming request and outbound request.
While writing the intermediate business logic, the developer can convert single or multiple incoming request into single or multiple 
outbound request. 

Also, while creating request to other chain, the contract can be developed in such a way where mutiple request can be generated to
different chains.

You can find examples for different scenarios in the [cw-bridge-contracts](https://github.com/router-protocol/cw-bridge-contracts.git) repository.


## [SudoMsg]

The sudo function is one of the entry point for a router-bridge-contract.
It can be called internally by the chain only. It needs to implement to receive incoming request from the other chains.
```sh

# import router binding message
use router_wasm_bindings::{RouterMsg, SudoMsg};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn sudo(deps: DepsMut, _env: Env, msg: SudoMsg) -> StdResult<Response<RouterMsg>> {
    match msg {
        # Sudo msg to handle in coming request from other chains
        SudoMsg::HandleInboundReq {
            sender,
            chain_type,
            source_chain_id,
            payload,
        } => handle_in_bound_request(deps, sender, chain_type, source_chain_id, payload),
        # Sudo msg to handle outbound message acknowledgement
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

```

## [RouterMsg]

The RouterMsg is a enum type  of the entry point for a router-bridge-contract.
It can be called internally by the chain only. It needs to implement to receive incoming request from the other chains.
```sh

# import router binding message
use router_wasm_bindings::{RouterMsg, SudoMsg};

let address: String = String::from("destination_contract_address");
let payload: Vec<u8> = let payload: Vec<u8> = b"sample paylaod data".to_vec();
# Single Outbound request with single contract call
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
        amount: Uint128::new(100_000u128),
    },
    outgoing_tx_fee: Coin {
        denom: String::from("router"),
        amount: Uint128::new(100_000u128),
    },
    is_atomic: false,
    exp_timestamp: None,
};
let outbound_batch_reqs: RouterMsg = RouterMsg::OutboundBatchRequests {
    outbound_batch_requests: vec![outbound_batch_req],
    is_sequenced: false,
};

let res = Response::new()
    .add_message(outbound_batch_reqs);
Ok(res)

```

## Compiling and running tests

Now that you created your custom contract, make sure you can compile and run it before
making any changes. Go into the repository and do:

```sh
# this will produce a wasm build in ./target/wasm32-unknown-unknown/release/YOUR_NAME_HERE.wasm
cargo wasm

# this runs unit tests with helpful backtraces
RUST_BACKTRACE=1 cargo unit-test

# auto-generate json schema
cargo schema
```

### Understanding the tests

The main code is in `src/contract.rs` and the unit tests there run in pure rust,
which makes them very quick to execute and give nice output on failures, especially
if you do `RUST_BACKTRACE=1 cargo unit-test`.

We consider testing critical for anything on a blockchain, and recommend to always keep
the tests up to date.

## Generating JSON Schema

While the Wasm calls (`instantiate`, `execute`, `query`) accept JSON, this is not enough
information to use it. We need to expose the schema for the expected messages to the
clients. You can generate this schema by calling `cargo schema`, which will output
3 files in `./schema`, corresponding to the 3 message types the contract accepts.

These files are in standard json-schema format, which should be usable by various
client side tools, either to auto-generate codecs, or just to validate incoming
json wrt. the defined schema.

## Preparing the Wasm bytecode for production

Before we upload it to a chain, we need to ensure the smallest output size possible,
as this will be included in the body of a transaction. We also want to have a
reproducible build process, so third parties can verify that the uploaded Wasm
code did indeed come from the claimed rust code.

To solve both these issues, we have produced `rust-optimizer`, a docker image to
produce an extremely small build output in a consistent manner. The suggest way
to run it is this:

```sh
docker run --rm -v "$(pwd)":/code \
  --mount type=volume,source="$(basename "$(pwd)")_cache",target=/code/target \
  --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
  cosmwasm/rust-optimizer:0.12.6
```

Or, If you're on an arm64 machine, you should use a docker image built with arm64.
```sh
docker run --rm -v "$(pwd)":/code \
  --mount type=volume,source="$(basename "$(pwd)")_cache",target=/code/target \
  --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
  cosmwasm/rust-optimizer-arm64:0.12.6
```

We must mount the contract code to `/code`. You can use a absolute path instead
of `$(pwd)` if you don't want to `cd` to the directory first. The other two
volumes are nice for speedup. Mounting `/code/target` in particular is useful
to avoid docker overwriting your local dev files with root permissions.
Note the `/code/target` cache is unique for each contract being compiled to limit
interference, while the registry cache is global.

This is rather slow compared to local compilations, especially the first compile
of a given contract. The use of the two volume caches is very useful to speed up
following compiles of the same contract.

This produces an `artifacts` directory with a `PROJECT_NAME.wasm`, as well as
`checksums.txt`, containing the Sha256 hash of the wasm file.
The wasm file is compiled deterministically (anyone else running the same
docker on the same git commit should get the identical file with the same Sha256 hash).
It is also stripped and minimized for upload to a blockchain (we will also
gzip it in the uploading process to make it even smaller).
