# PAR-coSNARK lightweight evaluation

This directory contains the lightweight second-instantiation evaluation used to assess the PAR-coSNARK release-check abstraction on the PSS/HyperPlonk-style collaborative prover. It is an applicability evaluation, not a second full-system PAR-coSNARK overhead benchmark.

## Build

```bash
rustup toolchain install nightly-2024-05-03 --profile minimal
export RUSTFLAGS="-Ctarget-cpu=native -Awarnings"
cargo +nightly-2024-05-03 build --release \
  --example hyperplonk \
  --example pvia_light_eval \
  --example pvia_commit_sanity \
  --features local
```

## Run

```bash
target/release/examples/hyperplonk --l 1 --n 8
target/release/examples/pvia_light_eval --n 8
target/release/examples/pvia_commit_sanity --l 2 --n 8
```

The main acceptance marker is `PVIA_SECOND_INSTANCE_LIGHT_PACKAGE: PASS` in `pvia-evaluation/results/FINAL_STATUS.txt`.

The `PVIA_` prefix in this executable output marker is retained verbatim for compatibility with the source code.

The experiment is intentionally lightweight: it validates representative collaborative Sumcheck and PCS release objects and a 16-party commitment sanity check. It is not reported as a second end-to-end PAR-coSNARK overhead benchmark.
