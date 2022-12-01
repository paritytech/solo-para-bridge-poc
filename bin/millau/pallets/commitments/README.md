License: Unlicense

# Commitments Pallet

## Overview
The act of gathering plain information block-by-block on-chain, in incentivised game environments such as paid surveys, auctions, oracle data verification is one that can be easily gamed through copying strategies of other players. To combat this, developers may employ a commit-reveal scheme to allow users to prove their answers early, conceal their answers, and reveal their answers in the future, proving their original correctness.

Commitments pallet carries logic to assist runtime developers in creating pallets that utilize commit-reveal schemes. This is accomplished through an expectation that users will submit a hash of their answers + a random seed to a `commit` function, and the verification of their answer + random seed later. During a reveal phase, the `reveal` function tries to copy the process used to hash the random seed along with the answer in order to recreate the original commit given by the user. If the recreated answer is the same as the original, we can say that the user's commitment was fulfilled.

As a pallet meant to only assist pallet developers in implementing commit-reveal schemes in their pallets, `pallet-commitments` contains no extrinsics.

## Assumptions
Pallet-commitments is very bare, and makes little assumptions of how the pallet developer wishes to build their commit-reveal logic. The only parameter is reveal window length configuration, which simply disallows `reveals` if the developer has not specified a window.

## Usage
A very basic usage of the pallet is as follows

1. Tightly couple your pallet with `pallet-commitments`. It is expected that pallets working with pallet-commitments would need full access to the `Commit` and `RevealWindow` storage items for more freedom in developing their commitment scheme.
2. Wrap the pallet's `commit` function in some extrinsic that receives the commitment hashed with the random seed from the user with any relevant application logic(access lists, lock up tokens, etc)
```rust
		#[pallet::weight(10000)]
		pub fn commit_answer(
			origin: OriginFor<T>,
            // Some u64 identifier for the topic that users are committing/revealing answers toward/
            // "question_id" could represent some specific question of a survey game, for example
			question_id: u64,
            // A hash of the original answer like: hash(answer + random_seed) (using rand OSRNG)
            committed_answer: T::Hash,
            // Some unique id for your pallet. This should be some constant value
            pallet_id: PalletId
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
            // Assuming a 'loose-coupling' configuration - https://docs.substrate.io/how-to-guides/v3/pallet-design/loose-coupling/
            let result = T::Commitments::commit(who, committed_answer, question_id);
            // ... handle result, etc
            Ok(())
        }
```
2. Write some logic to determine when the reveal period should open. For example, you might schedule the reveal period after you receive a majority of answers towards a `commit_key`. Note that for protection, commits are not allowed within the window of reveals. Once you know when the reveal period should occur, update the `RevealWindow` storage item accordingly.
3. Wrap the pallet's `reveal` function in some extrinsic. The `reveal` function will perform the validation on the answer, so any logic that is dependent on the answer result can be added here(token unlocks, etc.)

```rust

		#[pallet::weight(10000)]
		pub fn reveal_answer(
			origin: OriginFor<T>,
            // The answer that was used along with a random seed to generate the commit hash
			original_answer: Hash,
            // The id for the same topic that we issued a commitment for before
			question_id: u64,
            // The same random seed that the user had used when creating the hash for their commit hash
			random_seed: u64,
            // The same unique id for your pallet
            pallet_id: PalletId
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
            let result = T::Commitments::reveal(who, original_answer, question_id, random_seed);
            // ...any handling of the verification result
            Ok(())
        }
```
4. Answers can be referenced after-the fact on the `Commit`s StorageItem. `Commitment`s that are associated with a correct answer will have a `Some(hash)` in the `fulfillment` field.

