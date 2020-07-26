// #![allow(dead_code, unused_imports)]

use super::*;
use std::collections::HashMap;
use ckb_testtool::{builtin::ALWAYS_SUCCESS, context::Context};
use ckb_tool::{ckb_error::assert_error_eq, ckb_script::ScriptError};
use ckb_tool::ckb_types::{bytes::Bytes, packed::*, prelude::*};
use ckb_tool::ckb_types::core::{Capacity, TransactionBuilder};

// Constants
const MAX_CYCLES: u64 = 10_000_000;

// Error Codes
const ERROR_ARGS_LEN: i8 = 100;
const ERROR_AMOUNT_CKBYTES: i8 = 101;
const ERROR_AMOUNT_SUDT: i8 = 102;
const ERROR_EXCHANGE_RATE: i8 = 103;
const ERROR_COST: i8 = 104;
const ERROR_STRUCTURE: i8 = 105;

// 
struct LocalResources
{
	binaries: HashMap<String, Bytes>,
	out_points: HashMap<String, OutPoint>,
	scripts: HashMap<String, Script>,
	deps: HashMap<String, CellDep>,
}

impl LocalResources
{
	pub fn new() -> Self
	{
		Self
		{
			binaries: HashMap::new(),
			out_points: HashMap::new(),
			scripts: HashMap::new(),
			deps: HashMap::new(),
		}
	}
}

fn build_default_context_and_resources() -> (Context, TransactionBuilder, LocalResources)
{
	// Create the default context.
	let mut context = Context::default();

	// Create a resource holder.
	let mut resources = LocalResources::new();

	// Load Binaries.
	resources.binaries.insert("ico".to_owned(), Loader::default().load_binary("ico-lock"));
	resources.binaries.insert("sudt".to_owned(), Loader::default().load_binary("sudt"));
	resources.binaries.insert("lock-1".to_owned(), Loader::default().load_binary("yes-lock-1"));
	resources.binaries.insert("lock-2".to_owned(), Loader::default().load_binary("yes-lock-2"));
	resources.binaries.insert("cap-test-1".to_owned(), Loader::default().load_binary("cap-test-1"));
	resources.binaries.insert("cap-test-2".to_owned(), Loader::default().load_binary("cap-test-2"));
	
	// Deploy Binaries.
	resources.out_points.insert("ico".to_owned(), context.deploy_contract(resources.binaries.get("ico").unwrap().clone()));
	resources.out_points.insert("sudt".to_owned(), context.deploy_contract(resources.binaries.get("sudt").unwrap().clone()));
	resources.out_points.insert("cap-test-1".to_owned(), context.deploy_contract(resources.binaries.get("cap-test-1").unwrap().clone()));
	resources.out_points.insert("cap-test-2".to_owned(), context.deploy_contract(resources.binaries.get("cap-test-2").unwrap().clone()));
	resources.out_points.insert("lock-1".to_owned(), context.deploy_contract(ALWAYS_SUCCESS.clone()));
	
	// Create Scripts.
	resources.scripts.insert("lock-1".to_owned(), context.build_script(resources.out_points.get("lock-1").unwrap(), Default::default()).expect("script"));
	resources.scripts.insert("lock-2".to_owned(), context.build_script(resources.out_points.get("lock-1").unwrap(), [1u8, 1].to_vec().into()).expect("script"));
	resources.scripts.insert("lock-3".to_owned(), context.build_script(resources.out_points.get("lock-1").unwrap(),[2u8, 1].to_vec().into()).expect("script"));

	// Create dependencies.
	resources.deps.insert("ico".to_owned(), CellDep::new_builder().out_point(resources.out_points.get("ico").unwrap().clone()).build());
	resources.deps.insert("sudt".to_owned(), CellDep::new_builder().out_point(resources.out_points.get("sudt").unwrap().clone()).build());
	resources.deps.insert("lock-1".to_owned(), CellDep::new_builder().out_point(resources.out_points.get("lock-1").unwrap().clone()).build());
	resources.deps.insert("cap-test-1".to_owned(), CellDep::new_builder().out_point(resources.out_points.get("cap-test-1").unwrap().clone()).build());
	resources.deps.insert("cap-test-2".to_owned(), CellDep::new_builder().out_point(resources.out_points.get("cap-test-2").unwrap().clone()).build());

	// Build transaction.
	let tx = TransactionBuilder::default()
		.cell_dep(resources.deps.get(&"ico".to_owned()).unwrap().clone())
		.cell_dep(resources.deps.get(&"sudt".to_owned()).unwrap().clone())
		.cell_dep(resources.deps.get(&"lock-1".to_owned()).unwrap().clone())
		.cell_dep(resources.deps.get(&"cap-test-1".to_owned()).unwrap().clone())
		.cell_dep(resources.deps.get(&"cap-test-2".to_owned()).unwrap().clone());

	(context, tx, resources)
}

fn create_input_capacity_cell(context: &mut Context, resources: &LocalResources, capacity: u64) -> CellInput
{
	let (output, output_data) = create_output_capacity_cell(context, resources, capacity);
	let input_out_point = context.create_cell(output, output_data);
	let input = CellInput::new_builder().previous_output(input_out_point).build();

	input
}

fn create_output_capacity_cell(_context: &mut Context, resources: &LocalResources, capacity: u64) -> (CellOutput, Bytes)
{
	let lock_script = resources.scripts.get("lock-1").unwrap().clone();
	
	let output = CellOutput::new_builder()
		.capacity(Capacity::shannons(capacity).as_u64().pack())
		.lock(lock_script)
		.build();
	let output_data: Bytes = Default::default();

	(output, output_data)
}

fn create_input_ico_cell(context: &mut Context, resources: &LocalResources, capacity: u64, tokens: u128, cost: u64, ico_owner_mode: bool, sudt_owner_mode: bool) -> CellInput
{
	let (output, output_data) = create_output_ico_cell(context, resources, capacity, tokens, cost, ico_owner_mode, sudt_owner_mode);
	let input_out_point = context.create_cell(output, output_data);
	let input = CellInput::new_builder().previous_output(input_out_point).build();

	input
}

fn create_output_ico_cell(context: &mut Context, resources: &LocalResources, capacity: u64, tokens: u128, cost: u64, ico_owner_mode: bool, sudt_owner_mode: bool) -> (CellOutput, Bytes)
{
	let lock_script = resources.scripts.get("lock-1").unwrap().clone();
	let lock_hash_owner: [u8; 32] = lock_script.calc_script_hash().unpack();
	let lock_hash_zero = [0u8; 32];
	let lock_hash_ico = if ico_owner_mode { lock_hash_owner } else { lock_hash_zero };
	let lock_hash_sudt = if sudt_owner_mode { lock_hash_owner } else { lock_hash_zero };

	let mut lock_hash_cost = lock_hash_ico.to_vec();
	lock_hash_cost.append(&mut cost.to_le_bytes().to_vec());
	let ico_script_args: Bytes = lock_hash_cost.into();
	let ico_script = context.build_script(resources.out_points.get("ico").unwrap(), ico_script_args).expect("script");

	let sudt_script_args: Bytes = lock_hash_sudt.to_vec().into();
	let sudt_script = context.build_script(resources.out_points.get("sudt").unwrap(), sudt_script_args).expect("script");
	
	let output = CellOutput::new_builder()
		.capacity(Capacity::shannons(capacity).as_u64().pack())
		.lock(ico_script)
		.type_(Some(sudt_script).pack())
		.build();
	let output_data: Bytes = tokens.to_le_bytes().to_vec().into();

	(output, output_data)
}

fn create_input_sudt_cell(context: &mut Context, resources: &LocalResources, capacity: u64, tokens: u128, is_owner_mode: bool) -> CellInput
{
	let (output, output_data) = create_output_sudt_cell(context, resources, capacity, tokens, is_owner_mode);
	let input_out_point = context.create_cell(output, output_data);
	let input = CellInput::new_builder().previous_output(input_out_point).build();

	input
}

fn create_output_sudt_cell(context: &mut Context, resources: &LocalResources, capacity: u64, tokens: u128, is_owner_mode: bool) -> (CellOutput, Bytes)
{
	let lock_script = resources.scripts.get("lock-1").unwrap().clone();
	let lock_hash_owner: [u8; 32] = lock_script.calc_script_hash().unpack();
	let lock_hash_zero = [0u8; 32];
	let lock_hash = if is_owner_mode { lock_hash_owner } else { lock_hash_zero };
	let sudt_script_args: Bytes = lock_hash.to_vec().into();
	let sudt_script = context.build_script(resources.out_points.get("sudt").unwrap(), sudt_script_args).expect("script");
	
	let output = CellOutput::new_builder()
		.capacity(Capacity::shannons(capacity).as_u64().pack())
		.lock(lock_script)
		.type_(Some(sudt_script).pack())
		.build();
	let output_data: Bytes = tokens.to_le_bytes().to_vec().into();

	(output, output_data)
}

#[test]
fn test_ico_no_change()
{
	// Constants
	const ICO_OWNER_MODE: bool = false;
	const SUDT_OWNER_MODE: bool = false;

	// Get defaults.
	let (mut context, tx, resources) = build_default_context_and_resources();

	// Prepare inputs.
	let mut inputs = vec!();
	let input = create_input_ico_cell(&mut context, &resources, 1_000, 1_000, 100, ICO_OWNER_MODE, SUDT_OWNER_MODE);
	inputs.push(input);
	
	// Prepare outputs.
	let mut outputs = vec!();
	let mut outputs_data = vec!();
	let (output, output_data) = create_output_ico_cell(&mut context, &resources, 1_000, 1_000, 100, ICO_OWNER_MODE, SUDT_OWNER_MODE);
	outputs.push(output);
	outputs_data.push(output_data);
	
	// Populate the transaction, build, and complete.
	let tx = tx.inputs(inputs).outputs(outputs).outputs_data(outputs_data.pack()).build();
	let tx = context.complete_tx(tx);

	// Execute the transaction.
	let _cycles = context.verify_tx(&tx, MAX_CYCLES).expect("pass verification");
	// println!("Cycles: {}", cycles);
}

#[test]
fn test_ico_buy()
{
	// Constants
	const ICO_OWNER_MODE: bool = false;
	const SUDT_OWNER_MODE: bool = false;

	// Get defaults.
	let (mut context, tx, resources) = build_default_context_and_resources();

	// Prepare inputs.
	let mut inputs = vec!();
	let input = create_input_capacity_cell(&mut context, &resources, 1_000);
	inputs.push(input);
	let input = create_input_ico_cell(&mut context, &resources, 1_000, 100, 100, ICO_OWNER_MODE, SUDT_OWNER_MODE);
	inputs.push(input);
	
	// Prepare outputs.
	let mut outputs = vec!();
	let mut outputs_data = vec!();
	let (output, output_data) = create_output_capacity_cell(&mut context, &resources, 800);
	outputs.push(output);
	outputs_data.push(output_data);
	let (output, output_data) = create_output_ico_cell(&mut context, &resources, 1_100, 99, 100, ICO_OWNER_MODE, SUDT_OWNER_MODE);
	outputs.push(output);
	outputs_data.push(output_data);
	let (output, output_data) = create_output_sudt_cell(&mut context, &resources, 100, 1, SUDT_OWNER_MODE);
	outputs.push(output);
	outputs_data.push(output_data);
	
	// Populate the transaction, build, and complete.
	let tx = tx.inputs(inputs).outputs(outputs).outputs_data(outputs_data.pack()).build();
	let tx = context.complete_tx(tx);

	// Execute the transaction.
	let _cycles = context.verify_tx(&tx, MAX_CYCLES).expect("pass verification");
	// println!("Cycles: {}", cycles);
}

#[test]
fn test_ico_add_lock()
{
	// Constants
	const ICO_OWNER_MODE: bool = false;
	const SUDT_OWNER_MODE: bool = false;

	// Get defaults.
	let (mut context, tx, resources) = build_default_context_and_resources();

	// Prepare inputs.
	let mut inputs = vec!();
	let input = create_input_capacity_cell(&mut context, &resources, 100);
	inputs.push(input);
	let input = create_input_sudt_cell(&mut context, &resources, 100, 100, SUDT_OWNER_MODE);
	inputs.push(input);
	
	// Prepare outputs.
	let mut outputs = vec!();
	let mut outputs_data = vec!();
	let (output, output_data) = create_output_capacity_cell(&mut context, &resources, 100);
	outputs.push(output);
	outputs_data.push(output_data);
	let (output, output_data) = create_output_ico_cell(&mut context, &resources, 100, 100, 100, ICO_OWNER_MODE, SUDT_OWNER_MODE);
	outputs.push(output);
	outputs_data.push(output_data);
	
	// Populate the transaction, build, and complete.
	let tx = tx.inputs(inputs).outputs(outputs).outputs_data(outputs_data.pack()).build();
	let tx = context.complete_tx(tx);

	// Execute the transaction.
	let _cycles = context.verify_tx(&tx, MAX_CYCLES).expect("pass verification");
	// println!("Cycles: {}", cycles);
}

#[test]
fn test_ico_remove_lock()
{
	// Constants
	const ICO_OWNER_MODE: bool = true;
	const SUDT_OWNER_MODE: bool = false;

	// Get defaults.
	let (mut context, tx, resources) = build_default_context_and_resources();

	// Prepare inputs.
	let mut inputs = vec!();
	let input = create_input_capacity_cell(&mut context, &resources, 100);
	inputs.push(input);
	let input = create_input_ico_cell(&mut context, &resources, 1_000, 100, 100, ICO_OWNER_MODE, SUDT_OWNER_MODE);
	inputs.push(input);
	
	// Prepare outputs.
	let mut outputs = vec!();
	let mut outputs_data = vec!();
	let (output, output_data) = create_output_capacity_cell(&mut context, &resources, 1_000);
	outputs.push(output);
	outputs_data.push(output_data);
	let (output, output_data) = create_output_sudt_cell(&mut context, &resources, 100, 100, SUDT_OWNER_MODE);
	outputs.push(output);
	outputs_data.push(output_data);
	
	// Populate the transaction, build, and complete.
	let tx = tx.inputs(inputs).outputs(outputs).outputs_data(outputs_data.pack()).build();
	let tx = context.complete_tx(tx);

	// Execute the transaction.
	let _cycles = context.verify_tx(&tx, MAX_CYCLES).expect("pass verification");
	// println!("Cycles: {}", cycles);
}

#[test]
fn test_ico_remove_lock_no_owner()
{
	// Constants
	const ICO_OWNER_MODE: bool = false;
	const SUDT_OWNER_MODE: bool = false;

	// Get defaults.
	let (mut context, tx, resources) = build_default_context_and_resources();

	// Prepare inputs.
	let mut inputs = vec!();
	let input = create_input_capacity_cell(&mut context, &resources, 100);
	inputs.push(input);
	let input = create_input_ico_cell(&mut context, &resources, 1_000, 100, 100, ICO_OWNER_MODE, SUDT_OWNER_MODE);
	inputs.push(input);
	
	// Prepare outputs.
	let mut outputs = vec!();
	let mut outputs_data = vec!();
	let (output, output_data) = create_output_capacity_cell(&mut context, &resources, 1_000);
	outputs.push(output);
	outputs_data.push(output_data);
	let (output, output_data) = create_output_sudt_cell(&mut context, &resources, 100, 100, SUDT_OWNER_MODE);
	outputs.push(output);
	outputs_data.push(output_data);
	
	// Populate the transaction, build, and complete.
	let tx = tx.inputs(inputs).outputs(outputs).outputs_data(outputs_data.pack()).build();
	let tx = context.complete_tx(tx);

	// Execute the transaction.
	let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
	assert_error_eq!(err, ScriptError::ValidationFailure(ERROR_STRUCTURE));
}

#[test]
fn test_ico_split_lock()
{
	// Constants
	const ICO_OWNER_MODE: bool = true;
	const SUDT_OWNER_MODE: bool = false;

	// Get defaults.
	let (mut context, tx, resources) = build_default_context_and_resources();

	// Prepare inputs.
	let mut inputs = vec!();
	let input = create_input_capacity_cell(&mut context, &resources, 100);
	inputs.push(input);
	let input = create_input_ico_cell(&mut context, &resources, 100, 100, 100, ICO_OWNER_MODE, SUDT_OWNER_MODE);
	inputs.push(input);
	
	// Prepare outputs.
	let mut outputs = vec!();
	let mut outputs_data = vec!();
	let (output, output_data) = create_output_ico_cell(&mut context, &resources, 100, 50, 100, ICO_OWNER_MODE, SUDT_OWNER_MODE);
	outputs.push(output);
	outputs_data.push(output_data);
	let (output, output_data) = create_output_ico_cell(&mut context, &resources, 100, 50, 100, ICO_OWNER_MODE, SUDT_OWNER_MODE);
	outputs.push(output);
	outputs_data.push(output_data);
	
	// Populate the transaction, build, and complete.
	let tx = tx.inputs(inputs).outputs(outputs).outputs_data(outputs_data.pack()).build();
	let tx = context.complete_tx(tx);

	// Execute the transaction.
	let _cycles = context.verify_tx(&tx, MAX_CYCLES).expect("pass verification");
	// println!("Cycles: {}", cycles);
}

#[test]
fn test_ico_split_lock_no_owner()
{
	// Constants
	const ICO_OWNER_MODE: bool = false;
	const SUDT_OWNER_MODE: bool = false;

	// Get defaults.
	let (mut context, tx, resources) = build_default_context_and_resources();

	// Prepare inputs.
	let mut inputs = vec!();
	let input = create_input_ico_cell(&mut context, &resources, 1_000, 100, 100, ICO_OWNER_MODE, SUDT_OWNER_MODE);
	inputs.push(input);
	
	// Prepare outputs.
	let mut outputs = vec!();
	let mut outputs_data = vec!();
	let (output, output_data) = create_output_ico_cell(&mut context, &resources, 500, 50, 100, ICO_OWNER_MODE, SUDT_OWNER_MODE);
	outputs.push(output);
	outputs_data.push(output_data);
	let (output, output_data) = create_output_ico_cell(&mut context, &resources, 500, 50, 100, ICO_OWNER_MODE, SUDT_OWNER_MODE);
	outputs.push(output);
	outputs_data.push(output_data);
	
	// Populate the transaction, build, and complete.
	let tx = tx.inputs(inputs).outputs(outputs).outputs_data(outputs_data.pack()).build();
	let tx = context.complete_tx(tx);

	// Execute the transaction.
	let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
	assert_error_eq!(err, ScriptError::ValidationFailure(ERROR_STRUCTURE));
}

#[test]
fn test_ico_combine_lock()
{
	// Constants
	const ICO_OWNER_MODE: bool = true;
	const SUDT_OWNER_MODE: bool = false;

	// Get defaults.
	let (mut context, tx, resources) = build_default_context_and_resources();

	// Prepare inputs.
	let mut inputs = vec!();
	let input = create_input_capacity_cell(&mut context, &resources, 100);
	inputs.push(input);
	let input = create_input_ico_cell(&mut context, &resources, 100, 50, 100, ICO_OWNER_MODE, SUDT_OWNER_MODE);
	inputs.push(input);
	let input = create_input_ico_cell(&mut context, &resources, 100, 50, 100, ICO_OWNER_MODE, SUDT_OWNER_MODE);
	inputs.push(input);
	
	// Prepare outputs.
	let mut outputs = vec!();
	let mut outputs_data = vec!();
	let (output, output_data) = create_output_ico_cell(&mut context, &resources, 300, 100, 100, ICO_OWNER_MODE, SUDT_OWNER_MODE);
	outputs.push(output);
	outputs_data.push(output_data);
	
	// Populate the transaction, build, and complete.
	let tx = tx.inputs(inputs).outputs(outputs).outputs_data(outputs_data.pack()).build();
	let tx = context.complete_tx(tx);

	// Execute the transaction.
	let _cycles = context.verify_tx(&tx, MAX_CYCLES).expect("pass verification");
	// println!("Cycles: {}", cycles);
}

#[test]
fn test_ico_combine_lock_no_owner()
{
	// Constants
	const ICO_OWNER_MODE: bool = false;
	const SUDT_OWNER_MODE: bool = false;

	// Get defaults.
	let (mut context, tx, resources) = build_default_context_and_resources();

	// Prepare inputs.
	let mut inputs = vec!();
	let input = create_input_capacity_cell(&mut context, &resources, 100);
	inputs.push(input);
	let input = create_input_ico_cell(&mut context, &resources, 100, 50, 100, ICO_OWNER_MODE, SUDT_OWNER_MODE);
	inputs.push(input);
	let input = create_input_ico_cell(&mut context, &resources, 100, 50, 100, ICO_OWNER_MODE, SUDT_OWNER_MODE);
	inputs.push(input);
	
	// Prepare outputs.
	let mut outputs = vec!();
	let mut outputs_data = vec!();
	let (output, output_data) = create_output_ico_cell(&mut context, &resources, 300, 100, 100, ICO_OWNER_MODE, SUDT_OWNER_MODE);
	outputs.push(output);
	outputs_data.push(output_data);
	
	// Populate the transaction, build, and complete.
	let tx = tx.inputs(inputs).outputs(outputs).outputs_data(outputs_data.pack()).build();
	let tx = context.complete_tx(tx);

	// Execute the transaction.
	let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
	assert_error_eq!(err, ScriptError::ValidationFailure(ERROR_STRUCTURE));
}

#[test]
fn test_ico_buy_invalid_ckbytes()
{
	// Constants
	const ICO_OWNER_MODE: bool = false;
	const SUDT_OWNER_MODE: bool = false;

	// Get defaults.
	let (mut context, tx, resources) = build_default_context_and_resources();

	// Prepare inputs.
	let mut inputs = vec!();
	let input = create_input_capacity_cell(&mut context, &resources, 100);
	inputs.push(input);
	let input = create_input_ico_cell(&mut context, &resources, 1_000, 100, 100, ICO_OWNER_MODE, SUDT_OWNER_MODE);
	inputs.push(input);
	
	// Prepare outputs.
	let mut outputs = vec!();
	let mut outputs_data = vec!();
	let (output, output_data) = create_output_capacity_cell(&mut context, &resources, 200);
	outputs.push(output);
	outputs_data.push(output_data);
	let (output, output_data) = create_output_ico_cell(&mut context, &resources, 900, 100, 100, ICO_OWNER_MODE, SUDT_OWNER_MODE);
	outputs.push(output);
	outputs_data.push(output_data);
	
	// Populate the transaction, build, and complete.
	let tx = tx.inputs(inputs).outputs(outputs).outputs_data(outputs_data.pack()).build();
	let tx = context.complete_tx(tx);

	// Execute the transaction.
	let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
	assert_error_eq!(err, ScriptError::ValidationFailure(ERROR_AMOUNT_CKBYTES));
}

#[test]
fn test_ico_buy_invalid_sudt()
{
	// Constants
	const ICO_OWNER_MODE: bool = false;
	const SUDT_OWNER_MODE: bool = false;

	// Get defaults.
	let (mut context, tx, resources) = build_default_context_and_resources();

	// Prepare inputs.
	let mut inputs = vec!();
	let input = create_input_capacity_cell(&mut context, &resources, 100);
	inputs.push(input);
	let input = create_input_sudt_cell(&mut context, &resources, 100, 100, SUDT_OWNER_MODE);
	inputs.push(input);
	let input = create_input_ico_cell(&mut context, &resources, 1_000, 100, 100, ICO_OWNER_MODE, SUDT_OWNER_MODE);
	inputs.push(input);
	
	// Prepare outputs.
	let mut outputs = vec!();
	let mut outputs_data = vec!();
	let (output, output_data) = create_output_ico_cell(&mut context, &resources, 1_200, 200, 100, ICO_OWNER_MODE, SUDT_OWNER_MODE);
	outputs.push(output);
	outputs_data.push(output_data);
	
	// Populate the transaction, build, and complete.
	let tx = tx.inputs(inputs).outputs(outputs).outputs_data(outputs_data.pack()).build();
	let tx = context.complete_tx(tx);

	// Execute the transaction.
	let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
	assert_error_eq!(err, ScriptError::ValidationFailure(ERROR_AMOUNT_SUDT));
}

#[test]
fn test_ico_sell()
{
	// Constants
	const ICO_OWNER_MODE: bool = false;
	const SUDT_OWNER_MODE: bool = false;

	// Get defaults.
	let (mut context, tx, resources) = build_default_context_and_resources();

	// Prepare inputs.
	let mut inputs = vec!();
	let input = create_input_sudt_cell(&mut context, &resources, 100, 1, SUDT_OWNER_MODE);
	inputs.push(input);
	let input = create_input_ico_cell(&mut context, &resources, 1_000, 100, 100, ICO_OWNER_MODE, SUDT_OWNER_MODE);
	inputs.push(input);
	
	// Prepare outputs.
	let mut outputs = vec!();
	let mut outputs_data = vec!();
	let (output, output_data) = create_output_ico_cell(&mut context, &resources, 900, 101, 100, ICO_OWNER_MODE, SUDT_OWNER_MODE);
	outputs.push(output);
	outputs_data.push(output_data);
	let (output, output_data) = create_output_capacity_cell(&mut context, &resources, 200);
	outputs.push(output);
	outputs_data.push(output_data);
	
	// Populate the transaction, build, and complete.
	let tx = tx.inputs(inputs).outputs(outputs).outputs_data(outputs_data.pack()).build();
	let tx = context.complete_tx(tx);

	// Execute the transaction.
	let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
	assert_error_eq!(err, ScriptError::ValidationFailure(ERROR_AMOUNT_CKBYTES));
}

#[test]
fn test_ico_change_cost()
{
	// Constants
	const ICO_OWNER_MODE: bool = true;
	const SUDT_OWNER_MODE: bool = false;

	// Get defaults.
	let (mut context, tx, resources) = build_default_context_and_resources();

	// Prepare inputs.
	let mut inputs = vec!();
	let input = create_input_ico_cell(&mut context, &resources, 1_000, 100, 100, ICO_OWNER_MODE, SUDT_OWNER_MODE);
	inputs.push(input);
	let input = create_input_capacity_cell(&mut context, &resources, 100);
	inputs.push(input);

	// Prepare outputs.
	let mut outputs = vec!();
	let mut outputs_data = vec!();
	let (output, output_data) = create_output_ico_cell(&mut context, &resources, 1_000, 100, 50, ICO_OWNER_MODE, SUDT_OWNER_MODE);
	outputs.push(output);
	outputs_data.push(output_data);
	let (output, output_data) = create_output_capacity_cell(&mut context, &resources, 100);
	outputs.push(output);
	outputs_data.push(output_data);

	// Populate the transaction, build, and complete.
	let tx = tx.inputs(inputs).outputs(outputs).outputs_data(outputs_data.pack()).build();
	let tx = context.complete_tx(tx);

	// Execute the transaction.
	let _cycles = context.verify_tx(&tx, MAX_CYCLES).expect("pass verification");
	// println!("Cycles: {}", cycles);
}

#[test]
fn test_ico_change_cost_no_owner()
{
	// Constants
	const ICO_OWNER_MODE: bool = false;
	const SUDT_OWNER_MODE: bool = false;

	// Get defaults.
	let (mut context, tx, resources) = build_default_context_and_resources();

	// Prepare inputs.
	let mut inputs = vec!();
	let input = create_input_ico_cell(&mut context, &resources, 1_000, 100, 100, ICO_OWNER_MODE, SUDT_OWNER_MODE);
	inputs.push(input);
	let input = create_input_capacity_cell(&mut context, &resources, 100);
	inputs.push(input);

	// Prepare outputs.
	let mut outputs = vec!();
	let mut outputs_data = vec!();
	let (output, output_data) = create_output_ico_cell(&mut context, &resources, 1_000, 100, 50, ICO_OWNER_MODE, SUDT_OWNER_MODE);
	outputs.push(output);
	outputs_data.push(output_data);
	let (output, output_data) = create_output_capacity_cell(&mut context, &resources, 100);
	outputs.push(output);
	outputs_data.push(output_data);
	
	// Populate the transaction, build, and complete.
	let tx = tx.inputs(inputs).outputs(outputs).outputs_data(outputs_data.pack()).build();
	let tx = context.complete_tx(tx);

	// Execute the transaction.
	let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
	assert_error_eq!(err, ScriptError::ValidationFailure(ERROR_STRUCTURE));
}

#[test]
fn test_ico_remove_capacity()
{
	// Constants
	const ICO_OWNER_MODE: bool = true;
	const SUDT_OWNER_MODE: bool = false;

	// Get defaults.
	let (mut context, tx, resources) = build_default_context_and_resources();

	// Prepare inputs.
	let mut inputs = vec!();
	let input = create_input_ico_cell(&mut context, &resources, 1_000, 100, 100, ICO_OWNER_MODE, SUDT_OWNER_MODE);
	inputs.push(input);
	let input = create_input_capacity_cell(&mut context, &resources, 100);
	inputs.push(input);

	// Prepare outputs.
	let mut outputs = vec!();
	let mut outputs_data = vec!();
	let (output, output_data) = create_output_ico_cell(&mut context, &resources, 100, 100, 100, ICO_OWNER_MODE, SUDT_OWNER_MODE);
	outputs.push(output);
	outputs_data.push(output_data);
	let (output, output_data) = create_output_capacity_cell(&mut context, &resources, 1_000);
	outputs.push(output);
	outputs_data.push(output_data);

	// Populate the transaction, build, and complete.
	let tx = tx.inputs(inputs).outputs(outputs).outputs_data(outputs_data.pack()).build();
	let tx = context.complete_tx(tx);

	// Execute the transaction.
	let _cycles = context.verify_tx(&tx, MAX_CYCLES).expect("pass verification");
	// println!("Cycles: {}", cycles);
}

#[test]
fn test_ico_remove_capacity_no_owner()
{
	// Constants
	const ICO_OWNER_MODE: bool = false;
	const SUDT_OWNER_MODE: bool = false;

	// Get defaults.
	let (mut context, tx, resources) = build_default_context_and_resources();

	// Prepare inputs.
	let mut inputs = vec!();
	let input = create_input_ico_cell(&mut context, &resources, 1_000, 100, 100, ICO_OWNER_MODE, SUDT_OWNER_MODE);
	inputs.push(input);
	let input = create_input_capacity_cell(&mut context, &resources, 100);
	inputs.push(input);

	// Prepare outputs.
	let mut outputs = vec!();
	let mut outputs_data = vec!();
	let (output, output_data) = create_output_ico_cell(&mut context, &resources, 100, 100, 100, ICO_OWNER_MODE, SUDT_OWNER_MODE);
	outputs.push(output);
	outputs_data.push(output_data);
	let (output, output_data) = create_output_capacity_cell(&mut context, &resources, 1_000);
	outputs.push(output);
	outputs_data.push(output_data);

	// Populate the transaction, build, and complete.
	let tx = tx.inputs(inputs).outputs(outputs).outputs_data(outputs_data.pack()).build();
	let tx = context.complete_tx(tx);

	// Execute the transaction.
	let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
	assert_error_eq!(err, ScriptError::ValidationFailure(ERROR_AMOUNT_CKBYTES));
}

#[test]
fn test_ico_remove_tokens()
{
	// Constants
	const ICO_OWNER_MODE: bool = true;
	const SUDT_OWNER_MODE: bool = false;

	// Get defaults.
	let (mut context, tx, resources) = build_default_context_and_resources();

	// Prepare inputs.
	let mut inputs = vec!();
	let input = create_input_ico_cell(&mut context, &resources, 100, 1_100, 100, ICO_OWNER_MODE, SUDT_OWNER_MODE);
	inputs.push(input);
	let input = create_input_capacity_cell(&mut context, &resources, 100);
	inputs.push(input);

	// Prepare outputs.
	let mut outputs = vec!();
	let mut outputs_data = vec!();
	let (output, output_data) = create_output_ico_cell(&mut context, &resources, 100, 100, 100, ICO_OWNER_MODE, SUDT_OWNER_MODE);
	outputs.push(output);
	outputs_data.push(output_data);
	let (output, output_data) = create_output_sudt_cell(&mut context, &resources, 100, 1_000, SUDT_OWNER_MODE);
	outputs.push(output);
	outputs_data.push(output_data);

	// Populate the transaction, build, and complete.
	let tx = tx.inputs(inputs).outputs(outputs).outputs_data(outputs_data.pack()).build();
	let tx = context.complete_tx(tx);

	// Execute the transaction.
	let _cycles = context.verify_tx(&tx, MAX_CYCLES).expect("pass verification");
	// println!("Cycles: {}", cycles);
}

#[test]
fn test_ico_remove_tokens_no_owner()
{
	// Constants
	const ICO_OWNER_MODE: bool = false;
	const SUDT_OWNER_MODE: bool = false;

	// Get defaults.
	let (mut context, tx, resources) = build_default_context_and_resources();

	// Prepare inputs.
	let mut inputs = vec!();
	let input = create_input_ico_cell(&mut context, &resources, 100, 1_100, 100, ICO_OWNER_MODE, SUDT_OWNER_MODE);
	inputs.push(input);
	let input = create_input_capacity_cell(&mut context, &resources, 100);
	inputs.push(input);

	// Prepare outputs.
	let mut outputs = vec!();
	let mut outputs_data = vec!();
	let (output, output_data) = create_output_ico_cell(&mut context, &resources, 100, 100, 100, ICO_OWNER_MODE, SUDT_OWNER_MODE);
	outputs.push(output);
	outputs_data.push(output_data);
	let (output, output_data) = create_output_sudt_cell(&mut context, &resources, 100, 1_000, SUDT_OWNER_MODE);
	outputs.push(output);
	outputs_data.push(output_data);

	// Populate the transaction, build, and complete.
	let tx = tx.inputs(inputs).outputs(outputs).outputs_data(outputs_data.pack()).build();
	let tx = context.complete_tx(tx);

	// Execute the transaction.
	let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
	assert_error_eq!(err, ScriptError::ValidationFailure(ERROR_EXCHANGE_RATE));
}

#[test]
fn test_ico_add_tokens()
{
	// Constants
	const ICO_OWNER_MODE: bool = true;
	const SUDT_OWNER_MODE: bool = false;

	// Get defaults.
	let (mut context, tx, resources) = build_default_context_and_resources();

	// Prepare inputs.
	let mut inputs = vec!();
	let input = create_input_ico_cell(&mut context, &resources, 100, 0, 100, ICO_OWNER_MODE, SUDT_OWNER_MODE);
	inputs.push(input);
	let input = create_input_sudt_cell(&mut context, &resources, 100, 1_100, SUDT_OWNER_MODE);
	inputs.push(input);

	// Prepare outputs.
	let mut outputs = vec!();
	let mut outputs_data = vec!();
	let (output, output_data) = create_output_ico_cell(&mut context, &resources, 100, 1_000, 100, ICO_OWNER_MODE, SUDT_OWNER_MODE);
	outputs.push(output);
	outputs_data.push(output_data);
	let (output, output_data) = create_output_sudt_cell(&mut context, &resources, 100, 100, SUDT_OWNER_MODE);
	outputs.push(output);
	outputs_data.push(output_data);

	// Populate the transaction, build, and complete.
	let tx = tx.inputs(inputs).outputs(outputs).outputs_data(outputs_data.pack()).build();
	let tx = context.complete_tx(tx);

	// Execute the transaction.
	let _cycles = context.verify_tx(&tx, MAX_CYCLES).expect("pass verification");
	// println!("Cycles: {}", cycles);
}

#[test]
fn test_ico_add_tokens_no_owner()
{
	// Constants
	const ICO_OWNER_MODE: bool = false;
	const SUDT_OWNER_MODE: bool = false;

	// Get defaults.
	let (mut context, tx, resources) = build_default_context_and_resources();

	// Prepare inputs.
	let mut inputs = vec!();
	let input = create_input_ico_cell(&mut context, &resources, 100, 0, 100, ICO_OWNER_MODE, SUDT_OWNER_MODE);
	inputs.push(input);
	let input = create_input_sudt_cell(&mut context, &resources, 100, 1_100, SUDT_OWNER_MODE);
	inputs.push(input);

	// Prepare outputs.
	let mut outputs = vec!();
	let mut outputs_data = vec!();
	let (output, output_data) = create_output_ico_cell(&mut context, &resources, 100, 1_000, 100, ICO_OWNER_MODE, SUDT_OWNER_MODE);
	outputs.push(output);
	outputs_data.push(output_data);
	let (output, output_data) = create_output_sudt_cell(&mut context, &resources, 100, 100, SUDT_OWNER_MODE);
	outputs.push(output);
	outputs_data.push(output_data);

	// Populate the transaction, build, and complete.
	let tx = tx.inputs(inputs).outputs(outputs).outputs_data(outputs_data.pack()).build();
	let tx = context.complete_tx(tx);

	// Execute the transaction.
	let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
	assert_error_eq!(err, ScriptError::ValidationFailure(ERROR_AMOUNT_SUDT));
}

#[test]
fn test_ico_add_tokens_dual_owner()
{
	// Constants
	const ICO_OWNER_MODE: bool = true;
	const SUDT_OWNER_MODE: bool = true;

	// Get defaults.
	let (mut context, tx, resources) = build_default_context_and_resources();

	// Prepare inputs.
	let mut inputs = vec!();
	let input = create_input_ico_cell(&mut context, &resources, 100, 0, 100, ICO_OWNER_MODE, SUDT_OWNER_MODE);
	inputs.push(input);
	let input = create_input_capacity_cell(&mut context, &resources, 100);
	inputs.push(input);

	// Prepare outputs.
	let mut outputs = vec!();
	let mut outputs_data = vec!();
	let (output, output_data) = create_output_ico_cell(&mut context, &resources, 100, 1_000, 100, ICO_OWNER_MODE, SUDT_OWNER_MODE);
	outputs.push(output);
	outputs_data.push(output_data);
	let (output, output_data) = create_output_capacity_cell(&mut context, &resources, 100);
	outputs.push(output);
	outputs_data.push(output_data);
	
	// Populate the transaction, build, and complete.
	let tx = tx.inputs(inputs).outputs(outputs).outputs_data(outputs_data.pack()).build();
	let tx = context.complete_tx(tx);

	// Execute the transaction.
	let _cycles = context.verify_tx(&tx, MAX_CYCLES).expect("pass verification");
	// println!("Cycles: {}", cycles);
}

#[test]
fn test_ico_invalid_args()
{
	// Get defaults.
	let (mut context, tx, resources) = build_default_context_and_resources();

	// Make an ICO Cell with an invalid argument.
	let lock_hash = [0u8; 32].to_vec();
	let ico_script = context.build_script(resources.out_points.get("ico").unwrap(), lock_hash.clone().into()).expect("script");
	let sudt_script = context.build_script(resources.out_points.get("sudt").unwrap(), lock_hash.into()).expect("script");
	let output = CellOutput::new_builder().capacity(Capacity::shannons(1_000).as_u64().pack()).lock(ico_script).type_(Some(sudt_script).pack()).build();
	let output_data: Bytes = 1_000u128.to_le_bytes().to_vec().into();
	let input_out_point = context.create_cell(output.clone(), output_data.clone());
	let input = CellInput::new_builder().previous_output(input_out_point).build();

	// Prepare inputs.
	let mut inputs = vec!();
	inputs.push(input);
	
	// Prepare outputs.
	let mut outputs = vec!();
	let mut outputs_data = vec!();
	outputs.push(output);
	outputs_data.push(output_data);

	// Populate the transaction, build, and complete.
	let tx = tx.inputs(inputs).outputs(outputs).outputs_data(outputs_data.pack()).build();
	let tx = context.complete_tx(tx);

	// Execute the transaction.
	let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
	assert_error_eq!(err, ScriptError::ValidationFailure(ERROR_ARGS_LEN));
}

#[test]
fn test_ico_invalid_cost()
{
	// Constants
	const ICO_OWNER_MODE: bool = false;
	const SUDT_OWNER_MODE: bool = false;

	// Get defaults.
	let (mut context, tx, resources) = build_default_context_and_resources();

	// Prepare inputs.
	let mut inputs = vec!();
	let input = create_input_capacity_cell(&mut context, &resources, 100);
	inputs.push(input);
	let input = create_input_ico_cell(&mut context, &resources, 100, 100, 0, ICO_OWNER_MODE, SUDT_OWNER_MODE);
	inputs.push(input);
	
	// Prepare outputs.
	let mut outputs = vec!();
	let mut outputs_data = vec!();
	let (output, output_data) = create_output_capacity_cell(&mut context, &resources, 100);
	outputs.push(output);
	outputs_data.push(output_data);
	let (output, output_data) = create_output_ico_cell(&mut context, &resources, 100, 100, 0, ICO_OWNER_MODE, SUDT_OWNER_MODE);
	outputs.push(output);
	outputs_data.push(output_data);
	
	// Populate the transaction, build, and complete.
	let tx = tx.inputs(inputs).outputs(outputs).outputs_data(outputs_data.pack()).build();
	let tx = context.complete_tx(tx);

	// Execute the transaction.
	let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
	assert_error_eq!(err, ScriptError::ValidationFailure(ERROR_COST));
}
