# Performing Runtime Upgrades

Forkless runtime upgrades is one of the key features of Substrate-based chains. But, as
always, with great power comes great responsibility. Let's go over some things, which
one should **not** do when upgrading *this* chain.

## TL; DR
Generally, if you look at a PR and see:
- that some type is used in both the `primitives` and the runtime
- some existing extrinsic that is called by the client has its signature changed

Chances are the node will break after performing the runtime upgrade containing these changes.
Check thoroughly if the off-chain logic depends on the runtime types that you are going
to change. This almost always saves you from runtime breakages.

### Changing function signatures

Our node consists of two parts, basically - the off-chain part & the on-chain part.
As you know, forkless runtime upgrades are destined to upgrade the running *on-chain* code.
That is, if one updates their running chain, which internally runs some off-chain logic,
chances are that this off-chain logic will break - and that's not good.
That is our case. Apart from running miscellaneous processes off-chain (tx queue,
rpc server, consensus, grandpa voter, etc.), we run vital logic off-chain. In this project,
the off-chain client component is responsible for doing some heavy-duty computations and
submitting the results of those computations onto the chain.
Meaning, if we change the signature of the function in the pallet and perform a runtime
upgrade, our changes will not be automatically reflected in the off-chain component,
leading to breaking code.

To sum up, this is a change that leads to a **hard fork**, which is not inherently bad -
just be aware such upgrade will in fact require the nodes to restart.

### Changing core types

This is an easy one - you change the hash type, the account id type, etc. on-chain,
you don't reflect it necessarily in the off-chain component. It leads to breakages, too.

### Testing Upgrades for breakage
If you are unsure whether a change is breaking or not, you can always try the upgrade for the proposed
change yourself and see whether it breaks.
