# `solana-zk-layer2`

A mini Solana layer2 (network extension) that proves state transitions by verifying Groth16 proofs, leveraging Solana’s BN254 precompiles for efficient cryptographic operations.

## Motivation
Solana's BN254 precompiles bring the potential for execution layer scalability. This project aims to explore innovations in Solana's network extensions.
[`SIMD-0302: BN254 G2 Arithmetic Syscalls`](https://github.com/solana-foundation/solana-improvement-documents/blob/main/proposals/XXXX-bn254-g2-syscalls.md)

## Additional Notes
This project uses the [`groth16-solana`](https://github.com/Lightprotocol/groth16-solana/) crate from Light Protocol Labs for the Groth16 proof verification, and [`sp1-solana`](https://github.com/succinctlabs/sp1-solana/) crate for generate Groth16 proof, and the [`ark-bn254`](https://github.com/arkworks-rs/algebra) crate for the elliptic curve operations.
