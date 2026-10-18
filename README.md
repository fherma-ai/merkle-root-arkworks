<!-- GENERATED from merkle_root/bn254@1.0.0. Do not edit — `--update` rewrites it. -->
# A Merkle root over BN254, compressed two-to-one with Poseidon

An answer to `merkle_root/bn254@1.0.0`, in rust.

## What you write

`src/solve.rs`, and nothing else.

```
init(Point)         -> state     setup. Not measured.
run(state, Inputs)  -> Outputs   the task. Measured, and only this.
free(state)                      optional.
```

`init` never sees the data, so there is nothing for it to compute ahead of
time. Reading the case and writing the answer happen outside the measured
call, in generated code — which is why seeing the inputs early is not a way
to answer early.

## The shapes

- `leaves` — tensor<N x L x u64>
- returns `root` — tensor<L x u64>

## Generated, and not yours

Every file but the one above carries a header saying so. They are rewritten
by `fherma implementation init --update`, which never touches yours.
