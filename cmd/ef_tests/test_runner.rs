use std::{collections::HashMap, path::Path};

use crate::types::{Account, TestUnit};
use ethereum_rust_core::{
    rlp::decode::RLPDecode,
    rlp::encode::RLPEncode,
    types::{Account as CoreAccount, Block as CoreBlock, Transaction as CoreTransaction},
    Address,
};
use ethereum_rust_evm::{apply_state_transitions, evm_state, execute_tx, EvmState, SpecId};
use ethereum_rust_storage::{EngineType, Store};

pub fn execute_test(test_key: &str, test: &TestUnit) {
    // Build pre state
    let mut evm_state = build_evm_state_from_prestate(&test.pre);
    let blocks = test.blocks.clone();
    // Execute all txs in the test unit
    for block in blocks.iter() {
        let block_header = block.block_header.clone().unwrap();
        let transactions = block.transactions.as_ref().unwrap();
        for transaction in transactions.iter() {
            assert_eq!(
                transaction.clone().sender,
                CoreTransaction::from(transaction.clone()).sender(),
                "Expected sender address differs from derived sender address on test: {}",
                test_key
            );
            assert!(
                execute_tx(
                    &transaction.clone().into(),
                    &block_header.clone().into(),
                    &mut evm_state,
                    SpecId::CANCUN,
                )
                .is_ok(), //TODO: Assert ExecutionResult depending on test case
                "Transaction execution failed on test: {}",
                test_key
            );
        }
    }
    // Apply state transitions
    apply_state_transitions(&mut evm_state).expect("Failed to update DB state");
    // Check post state
    // TODO
}

pub fn parse_test_file(path: &Path) -> HashMap<String, TestUnit> {
    let s: String = std::fs::read_to_string(path).expect("Unable to read file");
    let tests: HashMap<String, TestUnit> = serde_json::from_str(&s).expect("Unable to parse JSON");
    tests
}

//TODO: We shouldn't skip validating the tests with the field expect_exception.
//      We should run them and assert that those return the specified exception.
//      From the vectors/cancun tests, only tests in eip4844_blobs expect exceptions.
pub fn validate_test(test: &TestUnit) {
    // check that the decoded genesis block header matches the deserialized one
    let genesis_rlp = test.genesis_rlp.clone();
    let decoded_block = CoreBlock::decode(&genesis_rlp).unwrap();
    assert_eq!(
        decoded_block.header,
        test.genesis_block_header.clone().into()
    );

    // check that blocks can be decoded
    for block in &test.blocks {
        // skip the blocks with exceptions expected
        if block.expect_exception.is_some() {
            continue;
        }

        match CoreBlock::decode(block.rlp.as_ref()) {
            Ok(decoded_block) => {
                // check that the decoded block matches the deserialized one
                assert_eq!(decoded_block, (block.clone()).into());
                let mut rlp_block = Vec::new();
                // check that encoding the decoded block matches the rlp field
                decoded_block.encode(&mut rlp_block);
                assert_eq!(rlp_block, block.rlp.to_vec());
            }
            Err(_) => assert!(block.expect_exception.is_some()),
        }
    }
}

// Creates an in-memory DB for evm execution and loads the prestate accounts
pub fn build_evm_state_from_prestate(pre: &HashMap<Address, Account>) -> EvmState {
    let mut store =
        Store::new("store.db", EngineType::InMemory).expect("Failed to build DB for testing");
    for (address, account) in pre {
        let account: CoreAccount = account.clone().into();
        store
            .add_account(*address, account)
            .expect("Failed to write to test DB")
    }
    evm_state(store)
}