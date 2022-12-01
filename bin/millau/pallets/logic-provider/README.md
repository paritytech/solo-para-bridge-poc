License: Unlicense

# Logic Provider Pallet

## Overview
Business logic modules for processing the required Metadata JSON object.

Logic provider runs of proof of stake mechanism where it is responsible for submitting hash into the chain, staking some amount of the submitter before submitting the hash and issuing the rewards
to the participants who have submitted the correct result or burning tokens for those who submitted incorrect(not equal to committed hash)/wrong or didn't reveal their resultant hash.

Here the reward issuance is based on the majority rule, if the participants count reaches the majority then only consensus begins
else and the winner are selected if and only if the correctness percentage is more than the majority rule.

The submission of result is based on commit and reveal approach, the users need to commit their hash wrapped with
random seed and can reveal their original hash along with the random seed once reveal window has started for accepting revealed hashes.
No commitments are accpeted after opening of reveal window.

## Interface

### Dispatchable Functions
- `commit_processing_result_hash` - Responsible for submitting the committed hash by staking some defined
   amount from the submitter's balance.
- `reveal_processing_result_hash` - Responsible for submitting the revealed hash.
- `issue_rewards` - Responsible for configuring out the winning participants, issuing rewards to winning participants,
   and burning staked tokens from the accounts whi has submitted incorrect result,
- `set_majority_type` - Responsible for setting the majority for consensus, default is one-third of the total participants.
- `resolve_metadata_dispute` - Responsible for resolving the dispute manually in case of consensus error(Eg: Consensus not reached).

# How to benchmark the pallet

For adequate weight estimation, one **must** benchmark a pallet.
That's why when adding a xt, we should add a benchmark for it.
To run the benchmarks and do weight estimation, do the following steps:
* Build the node with the `runtime-benchmarks` feature (`cargo build --release
--features=runtime-benchmarks`)
* Execute the benchmark by invoking `./benchmark.sh` from the `src` dir
* After, you will get the `weights_new.rs` file, which contains the weight info
for your benchmarked xts
* Copy the weight implementation from `weights_new.rs` to `weights.rs`

Voilà! You have completed the weight estimation.
