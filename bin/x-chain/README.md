# Messaging Template Pallet

This Pallet is meant to serve as a template for cross-chain message pathways from solo chains to the X parachain.

That relationship is described as follows according to the design:

1. New Sovereign Chain copies this pallet
2. Business logic updates are made to the pallet copy
3. Pallet is implemented for the solo chain, on a pallet index that is unused on both solo chains and X parachain.
4. Pallet is copied to X parachain, implemented at the same pallet index.

In terms of maintenance for these pallets, the extrinsic indices should never change, or else their communication will break.
