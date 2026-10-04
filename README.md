# Scalable Collaborative zk-SNARK — PVIA Second-Instantiation Artifact

This fork preserves the public **Scalable Collaborative zk-SNARK / PSS-HyperPlonk** artifact and adds the lightweight **second-instantiation evaluation** used by the PVIA project.

The primary PVIA implementation and full end-to-end evaluation are in:

https://github.com/liuliangxin/Code-Based-Scalable-coSNARKs

This repository serves a different purpose: it tests whether the PVIA release-check abstraction can be applied to a structurally different collaborative prover without turning this repository into a second full-system PVIA benchmark.

## PVIA additions in this fork

The final evaluation-specific additions are:

```text
hyperplonk/examples/pvia_light_eval.rs
hyperplonk/examples/pvia_commit_sanity.rs

pvia-evaluation/
  README.md
  SETUP_STATUS.txt
  patches/
  results/
```

The integration is intentionally small. The final experiment does **not** activate an algorithmic rewrite of the upstream HyperPlonk protocol.

Compatibility changes are recorded under:

```text
pvia-evaluation/patches/
```

They include the Rust-1.80 dependency/iterator compatibility required to reproduce the artifact on the paper-era nightly toolchain.

## Environment used for the PVIA applicability test

The test was executed on:

```text
OS: Linux 5.4
CPU: 2 x Intel Xeon Gold 6330 @ 2.00 GHz
Hardware threads: 112
RAM: 251 GiB

Rust: nightly-2024-05-03
rustc: 1.80.0-nightly
```

## Toolchain setup

Install the matching nightly:

```bash
rustup toolchain install nightly-2024-05-03 --profile minimal
source "$HOME/.cargo/env"
```

Check:

```bash
rustc +nightly-2024-05-03 --version
cargo +nightly-2024-05-03 --version
```

## Build

From the repository root:

```bash
export RUSTFLAGS="-Ctarget-cpu=native -Awarnings"

cargo +nightly-2024-05-03 build \
  --release \
  --example hyperplonk \
  --example pvia_light_eval \
  --example pvia_commit_sanity \
  --features local
```

The three relevant binaries are:

```text
target/release/examples/hyperplonk
target/release/examples/pvia_light_eval
target/release/examples/pvia_commit_sanity
```

## 1. Upstream artifact smoke test

Run the original HyperPlonk example:

```bash
target/release/examples/hyperplonk --l 1 --n 8
```

In the fixed evaluation environment, the upstream artifact completed 3/3 local smoke runs. The median collaborative-simulation time was approximately 4.17 s.

This value is used only to show that the upstream artifact executes in the test environment. It is **not** used as a cross-system performance comparison with the primary PVIA repository.

## 2. Eight-party PVIA lightweight evaluation

Run:

```bash
target/release/examples/pvia_light_eval --n 8
```

Expected top-level marker:

```text
PVIA_SECOND_INSTANCE_LIGHT_EVAL: PASS
```

The test checks representative release objects:

```text
parties=8
n=8

Sumcheck released rounds:
  8/8 match the upstream local reference

modified Sumcheck round:
  WITHHOLD

packed PCS commitment:
  reference match

PCS opening-proof elements:
  reference match

native PCS verification:
  PASS with the corresponding upstream reference opening value

context binding:
  BOUND

wrong context:
  REJECTED

stale ordinal:
  REJECTED

modified payload:
  REJECTED
```

## 3. Sixteen-party commitment sanity check

Run:

```bash
target/release/examples/pvia_commit_sanity --l 2 --n 8
```

Expected:

```text
PVIA_SECOND_INSTANCE_COMMIT_SANITY: PASS
l=2
parties=16
n=8
reference_match=1
```

## Fixed result files

Curated results are under:

```text
pvia-evaluation/results/
```

Important files:

```text
FINAL_STATUS.txt
SUMMARY.txt
SOURCE_HASHES.txt
second_instance_light_eval.csv
second_instance_light_eval_runs.csv
second_instance_light_eval.tex
```

The main package-level acceptance marker is:

```text
PVIA_SECOND_INSTANCE_LIGHT_PACKAGE: PASS
```

## Measured lightweight costs

Five measured runs were collected after one warm-up.

Representative medians:

| Item | Median |
|---|---:|
| Collaborative Sumcheck primitive | 7.287 ms |
| Sumcheck release check | 67.334 us |
| Collaborative PCS commit/open path | 372.518 ms |
| Native PCS verification check | 30.763 ms |
| Context binding check | 0.797 us |

The corresponding per-operation ratios are diagnostic only. They must **not** be reported as end-to-end PVIA overhead for this repository.

The primary end-to-end PVIA overhead experiment is in:

https://github.com/liuliangxin/Code-Based-Scalable-coSNARKs

## Scope and artifact boundary

The upstream artifact is explicitly a research proof-of-concept benchmark. The PVIA evaluation therefore keeps a conservative correctness scope.

In particular:

- the eight verifier-visible Sumcheck round polynomials are included in the release claim;
- the upstream terminal Sumcheck endpoint is excluded from that claim;
- the collaborative PCS commitment and opening-proof elements are checked against the upstream reference;
- the final scalar returned by `c_open` remains in an upstream internal share representation;
- native PCS verification therefore uses the corresponding upstream reference opening value;
- no claim is made that this fork performs a complete end-to-end verification of every upstream HyperPlonk benchmark output.

## Reproduction note

For a paper-style repetition:

```bash
mkdir -p pvia-evaluation/manual-results

# warm-up
target/release/examples/pvia_light_eval --n 8 \
  > pvia-evaluation/manual-results/run_0.log 2>&1

# five measured runs
for i in 1 2 3 4 5; do
  target/release/examples/pvia_light_eval --n 8 \
    > "pvia-evaluation/manual-results/run_$i.log" 2>&1

  grep -q '^PVIA_SECOND_INSTANCE_LIGHT_EVAL: PASS$' \
    "pvia-evaluation/manual-results/run_$i.log"

  echo "run $i PASS"
done

target/release/examples/pvia_commit_sanity --l 2 --n 8 \
  > pvia-evaluation/manual-results/commit16.log 2>&1
```

## Upstream provenance

The original project implements the paper:

**Scalable Collaborative zk-SNARK and its Application to Fully Distributed Proof Delegation**, USENIX Security 2025.

The original artifact documentation is preserved below.

---

# Upstream artifact documentation

# Scalable-Collaborative-zk-SNARK

Rust implementation of the paper "[Scalable Collaborative zk-SNARK and its Application to Fully Distributed Proof Delegation](https://eprint.iacr.org/2024/940)", which appears in [*USENIX Security 2025*](https://www.usenix.org/conference/usenixsecurity25).

**⚠️ WARNING**: This is an academic proof-of-concept prototype and has **not** undergone a thorough code review. It is **NOT suitable** for production use. The benchmarks are intended primarily to measure time, memory, and communication complexities; **correctness of the output proofs is not guaranteed**.

**🔗 Acknowledgment**: This project is built on top of the [arkworks ecosystem](https://github.com/arkworks-rs). Several crates were adapted from and are credited to [collaborative-zksnark](https://github.com/alex-ozdemir/collaborative-zksnark) and [zkSaaS](https://github.com/guruvamsi-policharla/zksaas).

## Overview 

- [`config/`](config): Some configuration files and parameters.
- [`dist-primitive/`](dist-primitive): Implementation of collaborative and distributed primitives introduced in the paper.
- [`hack/`](hack): Scripts for running the code and benchmarks.
- [`hyperplonk/`](hyperplonk): Proof-of-concept implementation of collaborative [HyperPlonk](https://eprint.iacr.org/2022/1355), including the monolithic prover.
- [`mpc-net/`](mpc-net): Implementation of an MPC network for inter-party communication.
- [`secret-sharing/`](secret-sharing): Implementation of the Packed Secret Sharing (PSS) scheme, supporting both finite field and elliptic curve group elements.

## Illustration

**🙋 Artifact Evaluation**: For artifact evaluation reviewers, we provide detailed guidance. You can jump to [How to Benchmark](#benchmark).

In this work, we assume multiparty is connected through a peer-to-peer network for smooth operation of the MPC protocol. Each peer can be a low-end instance (e.g., 2 vCPU and 4 GB memory is enough). Upon receiving the secret-shared witness, the parties collaborate to generate a ZK proof for large-scale circuits while preserving witness privacy.

Ideally, the code should be executed in a distributed network environment with 16/32/64/128 servers. However, it is not always feasible for developers to conduct tests on such a large number of machines. Therefore, we provide three operation modes, which are categorized as follows and can be switched by adjusting the Rust [features](./hyperplonk/Cargo.toml):

- `benchmark`: This mode actually runs a *distributed* network, where different parties are deployed on different machines and collaborate together to generate a proof. We provide some scripts to deploy such a cluster, and our benchmark is based on this mode. See benchmark instructions [here](#benchmark). This mode actually communicates through a *LAN/WAN* network.
- `leader`: A single peer *locally* simulates its own part of the proof generation according to the protocol. It is ensured that this peer accurately executes its assigned tasks, and we properly track the computation time and communication overhead. Since in the paper, every peer undertakes the same workload, it is a promising way to evaluate the complexities in one server. This mode does NOT actually communicate through a network.
- `local` and `local-multi-thread`: The `local` mode simulates the distributed cluster *locally*, where all tasks are executed sequentially by a single thread. This means, in each protocol, the thread performs the computation for one party and then proceeds to the next. As a result, the total execution time should be divided by the number of parties to approximate the actual runtime in a real distributed setting.
The `local-multi-thread` mode enables multiple threads to simulate different parties locally, with each thread running concurrently to represent a separate party. Therefore, the number of available threads on your machine should not be less than the number of parties. However, we note that the performance estimation in this multi-threaded mode is often inaccurate. 
This mode actually communicates through a *Local* network.

## Version

It uses a nightly version of Rust.

```
rustup 1.27.1 (54dd3d00f 2024-04-24)
cargo 1.80.0-nightly (05364cb2f 2024-05-03)
```

**WARNING**: The `stdsimd` feature has been recently removed in the Rust nightly build. If you have installed your Rust toolchain recently, you should switch to an "older" version.

## How to run

### Benchmark

The benchmarks are based on the `benchmark` mode. A crucial parameter is $l$, which represents the packing factor as defined in the paper. To run a benchmark with packing factor $l$, you need $l \times 8$ servers/machines. The expected speedup is approximately between $l$ and $2l$ times.

**🙋 Artifact Evaluation**: We understand that it may be difficult for reviewers to access a large number of servers to reproduce the results in the `benchmark` mode, although the results presented in the paper were obtained using this mode. Therefore, you can use the `local` mode to simulate the results. Jump to [here](#collaborative--distributed-primitives) for references. Remember to divide the total execution time by the number of servers $N$ to estimate the actual running time.

If you have an additional *jump server* for your cluster, deploying the cluster becomes easier. A script is provided at `hack/prepare-server.sh` to prepare the jump server for running benchmarks. For inter-server communication, you will need an IP address file containing the list of server IPs in the following format:
```
192.168.1.2
192.168.1.3
192.168.1.4
192.168.1.5

```
Make sure the file ends with a newline, and provide the jump server's IP address as an input to the script. Notably, there are a few things to tweak:

1. You need to install `just` and `zip` command in the head.
2. You need to change the username and identity file in the script. Currently, the script contains hardcoded values for the SSH identity file and username. For example:
    ```
    scp -i ~/.ssh/zkp.pem ~/.ssh/zkp.pem root@$2:/root/.ssh/
    ```
    You should replace the `.pem` file path, the username `root`, and any associated paths with your own settings. Be sure to update all occurrences consistently throughout the scripts to match your environment.
3. Be sure to check the `pack.sh` scripts and see if any path is not correct for your system. 
4. Remove the zkSaaS-related lines if they are not available
5. Change the directories if you don't like them
6. Change the ports if they are not available
7. You can use the `tc` command in a Linux machine to control the network speed to simulate LAN/WAN.

There are 4 benchmarks available. They are:
1. [Collaborative and monolithic Hyperplonk (for general circuits)](./hack/run-hyperplonk/), coressponding to §5.2, Fig. 3, Tab. 2 in the paper
2. [Collaborative Hyperplonk (for data-parallel circuits)](./hack/run-hyperplonk-dataparallel/), §5.3, Tab. 4
3. [Collaborative permcheck (with prodcheck)](./hack/run-cpermcheck/), §4.3, Tab. 5
4. [Collaborative permcheck (improved)](./hack/run-dpermcheck/), §5.1, Tab. 5

The source code of zkSaaS can be found [here](https://github.com/guruvamsi-policharla/zksaas), and it can be executed by following the instructions provided in that repository. We also provide [scripts](./hack/run-zksaas/) to reproduce its result.

We strongly advise you run the scripts above, or you may have to read through the script yourself to understand how the scripts work and how to manually set up the addresses. To run the benchmarks, you have to:

1. Go to the jump server
2. `cd` to the desired bench
3. Change the benchmark scales in `handle_server.sh`, and run following commands:
    ```bash
    ./handle_server.sh ./ip_addresses.txt
    ```
4. You shall see results in `output` folder. We also provide a `read_data.ipynb` script for reading these output into .csv files.

### Collaborative \& Distributed primitives

When there are not enough machines, we also offer Rust examples for *locally* evaluating collaborative and distributed primitives under the `dist-primitive` folder. If you have [`just`](https://github.com/casey/just) installed, you can run:

```bash
just run --release --example <example name> <args>
```

If you don't have just, execute the examples using the raw cargo commands:

```bash
RUSTFLAGS="-Ctarget-cpu=native -Awarnings" cargo +nightly run --release --example <example name> <args>
```

For example, to run a collaborative sumcheck protocol in a `leader` mode (only one party executes its job locally), run:

```bash
just run --release --example sumcheck -F leader -- --l 8 --n 20
# WARNING: If you encounter a `Too many open files` error, please adjust your environment setting with `ulimit -HSn 65536` 
```

This command locally simulates the task of a single server in a network where $64 = l \times 8$ parties participate, and the input size for the sumcheck protocol is $2^{20}$. The output will indicate that the leader's running time is approximately $\frac{1}{8}$ of that of the local prover.

Also you can run:
```bash
just run --release --example sumcheck -F local -- --l 8 --n 20
```

This command initiates a local network to perform the same task. The output time should be divided by $N = 64 = 8 \times 8$ to estimate the simulated execution time for each party.

To further benchmark the collaborative primitives in a large scale, please check the scripts under `hack` folder (e.g., `hack/bench_sumcheck.sh`). We only provide commands for leader mode. To switch modes, try different Rust features. You can also change to `benchmark` mode if you have enough hardware resources.

### Collaborative ZKPs

We offer implementation and examples for collaborative HyperPlonk (in the `hyperplonk` crate). For example, to run the comparison between monolithic Hyperplonk and collaborative Hyperplonk:

```bash
# At the root directory
just run --release --example hyperplonk -F local -- --l 8 --n 15
# WARNING: HyperPlonk currently cannot be run in a ``leader`` mode locally
```

The program outputs the time taken for the a server running the protocol and its actual communication cost (both incoming and outgoing data) during the proof generation. This output can be redirected to a file for further analysis.


## License

This library is released under the MIT License.
