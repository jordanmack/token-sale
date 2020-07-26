# Simple ICO Lock Script

A simple Lock Script for handling the sale of [SUDT](https://talk.nervos.org/t/rfc-simple-udt-draft-spec/4333) tokens for CKBytes on Nervos CKB.

The ICO Lock can be added to an SUDT Cell to enable any user to buy SUDT tokens for a predefined price in CKBytes.

> This project is still under active development and should not be used in production environments.

## Constraints
The ICO Lock enforces the following constraints to ensure proper operation.

1. The arguments must be equal or greater than 40 bytes in length.
2. If an input Cell's lock hash matches that specified in the args, owner mode is then enabled and the Cell unlocks unconditionally.
3. There must be exactly one input Cell with the ICO Lock Script and exactly one output Cell with the ICO Lock Script.
4. The Type Script of both the input ICO Cell and output ICO Cell must match.
5. The cost of SUDTs in Shannons must be greater than or equal to 1.
6. The capacity on the output ICO Cell must be equal or higher than on the input ICO Cell.
7. The SUDT amount of the output ICO Cell must be equal or lower than the input ICO Cell.
8. The capacity difference between the input/output ICO Cells divided by the cost must equal the SUDT amount difference between the input/output ICO Cells.

## Building

This project is built in Rust using the [Capsule](https://github.com/nervosnetwork/capsule) development framework.

### Supported Environments
- Linux
- MacOS
- Windows (WSL2)

### Prerequisites
- [Capsule](https://github.com/nervosnetwork/capsule)

### Building a debug binary:

``` sh
capsule build
```

### Running tests on the debug binary:

``` sh
capsule test
```

### Building a release binary:

``` sh
capsule build --release
```

### Running tests on the release binary:

``` sh
capsule test --release
```

## Usage

### Args Definition
- Owner lock script hash (32 bytes)
- Cost per token in CKByte Shannons. (u64 LE 8 bytes)

The total size of the args should be exactly 40 bytes.

> Warning: Failure to supply proper arguments to the Lock Script can result in the permanent loss of SUDT tokens.

### Owner Mode

Administrative control of the ICO Lock is enabled using the Owner Input Recognition design pattern. If any input Cell in a transaction has a Lock Script Hash that matches the first 32 bytes of the args provided to the ICO Lock, then owner mode is enabled.

Owner mode allows the following actions:
- Add or remove CKBytes from the Cell.
- Add or remove SUDT tokens from the Cell.
- Update the token cost.
- Removal of the ICO Lock in favor of a different lock.

## License
[MIT](LICENSE)