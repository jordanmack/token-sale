//! Simple Token Sale Lock Script
//! https://github.com/jordanmack/token-sale
//! 
//! A simple Lock Script for handling the sale of SUDT tokens for CKBytes on Nervos CKB.
//! The Lock Script can be added to any SUDT Cell to enable any user to buy SUDT tokens for a predefined price in CKBytes.
//! 
//! Args Definition
//! 0: The Owner's Lock Script Hash (32 Bytes)
//! 1: The Cost per token in CKByte Shannons. (u64 LE 8 Bytes)
//! 2: A unique identifier for the Token Sale Cell. (u32 LE 4 bytes)
//! 
//! Constraints
//! 1. The arguments must be equal or greater than 40 bytes in length. The arguments length will be 44 bytes or more with a unique identifier, but the Script does not check this.
//! 2. If an input Cell's lock hash matches that specified in the args, owner mode is then enabled and the Cell unlocks unconditionally.
//! 3. The transaction must have exactly one input Cell with the Token Sale Lock Script and exactly one output Cell with the Token Sale Lock Script. These Lock Scripts must have the same arguments.
//! 4. The Type Script of both the input Token Sale Cell and output Token Sale Cell must match.
//! 5. The cost of SUDTs in Shannons must be greater than or equal to 1.
//! 6. The capacity on the output Token Sale Cell must be higher than on the input Token Sale Cell.
//! 7. The SUDT amount of the output Token Sale Cell must be lower than the input Token Sale Cell.
//! 8. The capacity difference between the input/output Token Sale Cells must equal the SUDT amount difference between the input/output Token Sale Cells multiplied by the cost.

#![no_std]
#![no_main]
#![feature(lang_items)]
#![feature(alloc_error_handler)]
#![feature(panic_info_message)]

// Import `Result` from `core` instead of from `std` since we are in no-std mode.
use core::result::Result;

// Import CKB syscalls and structures.
// https://nervosnetwork.github.io/ckb-std/riscv64imac-unknown-none-elf/doc/ckb_std/index.html
// use ckb_std::{debug, default_alloc, entry};
use ckb_std::{default_alloc, entry};
use ckb_std::ckb_constants::Source;
use ckb_std::ckb_types::{bytes::Bytes, packed::Bytes as Args, packed::Script, prelude::*};
use ckb_std::error::{SysError};
use ckb_std::high_level::{load_cell, load_cell_data, load_cell_lock_hash, load_script, QueryIter};

// Constants
const COST_AMOUNT_LEN: usize = 8; // Number of bytes for the token cost amount. (u64)
const LOCK_HASH_LEN: usize = 32; // Number of bytes for a lock hash. (Blake2b)
const SUDT_AMOUNT_DATA_LEN: usize = 16; // Number of bytes for an SUDT amount. (u128)
const ARGS_LEN: usize = LOCK_HASH_LEN + COST_AMOUNT_LEN; // Number of bytes required for args.

entry!(entry);
default_alloc!();

/// Program entry point.
fn entry() -> i8
{
	// Call main function and return error code.
	match main()
	{
		Ok(_) => 0,
		Err(err) => err as i8,
	}
}

/// Local error values.
/// Low values are reserved for Sys Error codes.
/// Values 100+ are for custom errors.
#[repr(i8)]
enum Error
{
	IndexOutOfBound = 1,
	ItemMissing,
	LengthNotEnough,
	Encoding,
	ArgsLen = 100,
	AmountCkbytes,
	AmountSudt,
	ExchangeRate,
	InvalidCost,
	InvalidStructure,
}

/// Map Sys Errors to local Error values.
impl From<SysError> for Error
{
	fn from(err: SysError) -> Self
	{
		use SysError::*;
		match err
		{
			IndexOutOfBound => Self::IndexOutOfBound,
			ItemMissing => Self::ItemMissing,
			LengthNotEnough(_) => Self::LengthNotEnough,
			Encoding => Self::Encoding,
			Unknown(err_code) => panic!("Unexpected Sys Error: {}", err_code),
		}
	}
}

/// Determine if owner mode is enabled.
fn check_owner_mode(args: &Args) -> Result<bool, Error>
{
	// Compares the Lock Script Hash from the first 32 bytes of the args with the Lock Scripts
	// of all input Cells to determine if a match exists.
	let args: Bytes = args.unpack();
	let is_owner_mode = QueryIter::new(load_cell_lock_hash, Source::Input)
		.find(|lock_hash| args[0..LOCK_HASH_LEN] == lock_hash[..]).is_some();

	Ok(is_owner_mode)
}

/// Determine the capacity and token amount in all Cells matching the specified Lock Script and Type Script.
fn determine_token_sale_cell_amounts(lock_script: &Script, type_script: &Script, source: Source) -> Result<(u64, u128), Error>
{
	let mut buf = [0u8; SUDT_AMOUNT_DATA_LEN];
	let lock_script_bytes = &lock_script.as_bytes()[..];
	let type_script_bytes = &type_script.as_bytes()[..];

	// Loop through all Cells in the specified source.
	let mut total_capacity = 0;
	let mut total_tokens = 0;
	let mut i = 0;
	loop
	{
		let cell = match load_cell(i, source)
		{
			Ok(cell) => cell,
			Err(SysError::IndexOutOfBound) => break,
			Err(e) => return Err(e.into()),
		};

		// Check if this Cell matches the Lock Script and Type Script.
		let cell_lock_bytes = &cell.lock().as_bytes()[..];
		let cell_type_bytes = &cell.type_().as_bytes()[..];
		if cell_lock_bytes == lock_script_bytes && cell_type_bytes == type_script_bytes
		{
			// Ensure the Cell data is valid then add the capacity and token amount to the totals.
			let data = load_cell_data(i, source)?;
			if data.len() == SUDT_AMOUNT_DATA_LEN
			{
				buf.copy_from_slice(&data);
				total_tokens += u128::from_le_bytes(buf);
				total_capacity += cell.capacity().unpack();
			}
			else
			{
				return Err(Error::Encoding);
			}
		}

		i += 1;
	}

	Ok((total_capacity, total_tokens))
}

/// Retrieve the token cost from the args.
fn determine_token_cost(args: &Args) -> Result<u64, Error>
{
	let args: Bytes = args.unpack();
	let mut buf = [0u8; COST_AMOUNT_LEN];

	// The token amount immediately follows the Lock Hash in the args.
	let slice_start = LOCK_HASH_LEN;
	let slice_end = slice_start + COST_AMOUNT_LEN;

	// Copy bytes from the args into a u64. 
	buf.copy_from_slice(&args[slice_start..slice_end]);
	let token_cost = u64::from_le_bytes(buf);

	if token_cost < 1
	{
		return Err(Error::InvalidCost);
	}

	Ok(token_cost)
}

/// Ensure that all the capacity, token, and cost amounts are valid.
fn validate_amounts(token_cost: u64, input_capacity_amount: u64, output_capacity_amount: u64, input_token_amount: u128, output_token_amount: u128) -> Result<(), Error>
{
	// The output capacity must be more than the input capacity.
	if output_capacity_amount <= input_capacity_amount
	{
		return Err(Error::AmountCkbytes);
	}

	// The output tokens must be less than the input tokens.
	if output_token_amount >= input_token_amount
	{
		return Err(Error::AmountSudt);
	}

	// The capacity received must properly equate to the tokens released at the proper token cost.
	if (output_capacity_amount - input_capacity_amount) as u128 != (input_token_amount - output_token_amount) * token_cost as u128
	{
		return Err(Error::ExchangeRate);
	}

	Ok(())
}

/// Ensure that a valid input Token Sale Cell exists.
fn validate_token_sale_inputs() -> Result<(Script, Script), Error>
{
	// Verify that index 1 does not exist.
	if load_cell(1, Source::GroupInput).is_ok()
	{
		return Err(Error::InvalidStructure);
	}

	// Load the Token Sale Cell. There should be exactly 1.
	let token_sale_cell = load_cell(0, Source::GroupInput)?;

	// Extract the Scripts. Both must exist.
	let lock_script = token_sale_cell.lock();
	let type_script = token_sale_cell.type_().to_opt().ok_or(Error::InvalidStructure)?;

	Ok((lock_script, type_script))
}

/// Ensure that a valid output Token Sale Cell exists.
fn validate_token_sale_outputs(lock_script: &Script, type_script: &Script) -> Result<(), Error>
{
	let lock_script_bytes = &lock_script.as_bytes()[..];
	let type_script_bytes = &type_script.as_bytes()[..];

	// Loop through all the output Cells.
	let mut i = 0;
	let mut token_sale_lock_cells = 0;
	loop
	{
		let cell = match load_cell(i, Source::Output)
		{
			Ok(cell) => cell,
			Err(SysError::IndexOutOfBound) => break,
			Err(e) => return Err(e.into()),
		};

		// Count up matching Token Sale Cells with a matching SUDT Type Script.
		let cell_lock_bytes = &cell.lock().as_bytes()[..];
		let cell_type_bytes = &cell.type_().as_bytes()[..];
		if cell_lock_bytes == lock_script_bytes && cell_type_bytes == type_script_bytes
		{
			token_sale_lock_cells += 1;
		}

		i += 1;
	}

	// debug!("Total Token Sale Lock Cells: {}", token_sale_lock_cells);

	// There must be exactly one output Token Sale Lock Cell and it must have a Type Script matching the input Token Sale Lock Cell.
	if token_sale_lock_cells != 1
	{
		return Err(Error::InvalidStructure);
	}

	Ok(())
}

fn main() -> Result<(), Error>
{
	// Load arguments from the current script.
	let script = load_script()?;
	let args = script.args();

	// Verify that the minimum length of the arguments was given.
	if args.len() < ARGS_LEN
	{
		return Err(Error::ArgsLen);
	}

	// If program is in owner mode then unlock immediately.
	if check_owner_mode(&args)?
	{
		// debug!("Token Sale owner mode enabled.");
		return Ok(());
	}

	// Check the inputs to ensure there is a single input Token Sale Cell.
	let (lock_script, type_script) = validate_token_sale_inputs()?;

	// Check the outputs to ensure there is a single output Token Sale Cell.
	validate_token_sale_outputs(&lock_script, &type_script)?;

	// Find all the capacity, token, and cost amounts.
	let token_cost = determine_token_cost(&args)?;
	let (input_capacity_amount, input_token_amount) = determine_token_sale_cell_amounts(&lock_script, &type_script, Source::GroupInput)?;
	let (output_capacity_amount, output_token_amount) = determine_token_sale_cell_amounts(&lock_script, &type_script, Source::Output)?;

	// debug!("Token Cost: {}", token_cost);
	// debug!("Input/Output Capacity: {}/{}", input_capacity_amount, output_capacity_amount);
	// debug!("Input/Output Token Amount: {}/{}", input_token_amount, output_token_amount);

	// Validate that all amounts are in balance.
	validate_amounts(token_cost, input_capacity_amount, output_capacity_amount, input_token_amount, output_token_amount)?;

	Ok(())
}
