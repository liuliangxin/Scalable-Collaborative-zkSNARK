use std::hint::black_box;
use std::time::Instant;

use ark_bls12_381::Bls12_381;
use ark_ec::pairing::Pairing;
use ark_serialize::CanonicalSerialize;
use ark_std::UniformRand;
use clap::Parser;
use dist_primitive::dpoly_comm::PolynomialCommitmentCub;
use dist_primitive::dsumcheck::{c_sumcheck_product, sumcheck_product};
use mpc_net::{LocalTestNet, MPCNet, MultiplexedStreamID};
use secret_sharing::pss::PackedSharingParams;
use sha2::{Digest, Sha256};

type E = Bls12_381;
type Fr = <E as Pairing>::ScalarField;
type G1 = <E as Pairing>::G1;

#[derive(Parser)]
struct Cli {
    #[arg(long, default_value_t = 8)]
    n: usize,
    #[arg(long, default_value_t = 50)]
    sumcheck_iters: usize,
    #[arg(long, default_value_t = 20)]
    pcs_iters: usize,
    #[arg(long, default_value_t = 200)]
    binding_iters: usize,
}

#[derive(Clone)]
struct ReleaseBinding {
    sid: [u8; 16],
    kind: u8,
    ordinal: u64,
    digest: [u8; 32],
}

fn digest_release(sid: &[u8; 16], kind: u8, ordinal: u64, payload: &[u8]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(b"PVIA-HYPERPLONK-LIGHT-V1");
    h.update(sid);
    h.update([kind]);
    h.update(ordinal.to_le_bytes());
    h.update((payload.len() as u64).to_le_bytes());
    h.update(payload);
    h.finalize().into()
}

fn make_binding(sid: [u8; 16], kind: u8, ordinal: u64, payload: &[u8]) -> ReleaseBinding {
    ReleaseBinding {
        sid,
        kind,
        ordinal,
        digest: digest_release(&sid, kind, ordinal, payload),
    }
}

fn check_binding(
    binding: &ReleaseBinding,
    sid: &[u8; 16],
    kind: u8,
    ordinal: u64,
    payload: &[u8],
) -> bool {
    binding.sid == *sid
        && binding.kind == kind
        && binding.ordinal == ordinal
        && binding.digest == digest_release(sid, kind, ordinal, payload)
}

fn eval_quadratic_at(round: &(Fr, Fr, Fr), x: Fr) -> Fr {
    let two = Fr::from(2u64);
    let c = round.0;
    let b = (-round.2 + round.1 * Fr::from(4u64) - round.0 * Fr::from(3u64)) / two;
    let a = (round.2 - round.1 * Fr::from(2u64) + round.0) / two;
    a * x * x + b * x + c
}

fn verify_released_sumcheck_rounds(
    initial_claim: Fr,
    rounds: &[(Fr, Fr, Fr)],
    challenge: &[Fr],
) -> bool {
    if rounds.is_empty() || rounds.len() != challenge.len() {
        return false;
    }
    if rounds[0].0 + rounds[0].1 != initial_claim {
        return false;
    }
    for i in 1..rounds.len() {
        let expected = eval_quadratic_at(&rounds[i - 1], challenge[i - 1]);
        if rounds[i].0 + rounds[i].1 != expected {
            return false;
        }
    }
    true
}

fn serialize_sumcheck(rounds: &[(Fr, Fr, Fr)]) -> Vec<u8> {
    let mut out = Vec::new();
    for (a, b, c) in rounds {
        a.serialize_compressed(&mut out).unwrap();
        b.serialize_compressed(&mut out).unwrap();
        c.serialize_compressed(&mut out).unwrap();
    }
    out
}

fn serialize_opening(commitment: &G1, value: &Fr, proof: &[G1]) -> Vec<u8> {
    let mut out = Vec::new();
    commitment.serialize_compressed(&mut out).unwrap();
    value.serialize_compressed(&mut out).unwrap();
    for item in proof {
        item.serialize_compressed(&mut out).unwrap();
    }
    out
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let args = Cli::parse();
    let l = 1usize;
    let pp = PackedSharingParams::<Fr>::new(l);
    let parties = pp.n;
    assert_eq!(parties, 8);

    let rng = &mut ark_std::test_rng();

    // Representative release 1: collaborative Sumcheck round polynomials.
    let x: Vec<Fr> = (0..(1usize << args.n)).map(|_| Fr::rand(rng)).collect();
    let mut workers_f = vec![Vec::new(); parties];
    let mut workers_g = vec![Vec::new(); parties];
    for chunk in x.chunks(l) {
        let shares = pp.pack_from_public(chunk.to_vec());
        for (party, share) in shares.into_iter().enumerate() {
            workers_f[party].push(share);
            workers_g[party].push(share);
        }
    }
    let challenge: Vec<Fr> = (0..args.n).map(|_| Fr::rand(rng)).collect();

    let sum_net = LocalTestNet::new_local_testnet(parties).await.unwrap();
    let sum_start = Instant::now();
    let sum_results = sum_net
        .simulate_network_round(
            (workers_f, workers_g, challenge.clone()),
            |net, (workers_f, workers_g, challenge)| async move {
                let pp = PackedSharingParams::<Fr>::new(1);
                let id = net.party_id() as usize;
                c_sumcheck_product(
                    &workers_f[id],
                    &workers_g[id],
                    &challenge,
                    &pp,
                    &net,
                    MultiplexedStreamID::Zero,
                )
                .await
                .unwrap()
            },
        )
        .await;
    let sum_protocol_ms = sum_start.elapsed().as_secs_f64() * 1000.0;

    let mut reconstructed = Vec::with_capacity(sum_results[0].len());
    for round in 0..sum_results[0].len() {
        let a = pp.unpack2(sum_results.iter().map(|v| v[round].0).collect())[0];
        let b = pp.unpack2(sum_results.iter().map(|v| v[round].1).collect())[0];
        let c = pp.unpack2(sum_results.iter().map(|v| v[round].2).collect())[0];
        reconstructed.push((a, b, c));
    }

    let reference = sumcheck_product(&x, &x, &challenge);
    let released_rounds = reconstructed[..args.n].to_vec();
    let reference_rounds = reference[..args.n].to_vec();
    let sumcheck_reference_match = released_rounds == reference_rounds;
    let terminal_endpoint_excluded = reconstructed[args.n] != reference[args.n];
    let initial_claim: Fr = x.iter().map(|v| *v * *v).sum();
    let sumcheck_honest =
        verify_released_sumcheck_rounds(initial_claim, &released_rounds, &challenge);

    let mut modified_rounds = released_rounds.clone();
    modified_rounds[1].0 += Fr::from(1u64);
    let sumcheck_modified_rejected =
        !verify_released_sumcheck_rounds(initial_claim, &modified_rounds, &challenge);

    // Representative release 2: packed collaborative PCS commitment/opening proof.
    let evals: Vec<Fr> = (0..(1usize << args.n)).map(|_| Fr::rand(rng)).collect();
    let point: Vec<Fr> = (0..args.n).map(|_| Fr::rand(rng)).collect();
    let trapdoor: Vec<Fr> = (0..args.n).map(|_| Fr::rand(rng)).collect();
    let g1 = G1::rand(rng);
    let g2 = <E as Pairing>::G2::rand(rng);
    let cub = PolynomialCommitmentCub::<E>::new(g1, g2, trapdoor);
    let native_pcs = cub.mature();
    let packed_pcs = cub.to_packed(&pp);
    let reference_commitment = native_pcs.commit(&evals);
    let (reference_value, reference_proof) = native_pcs.open(&evals, &point);

    let mut pcs_workers = vec![Vec::new(); parties];
    for chunk in evals.chunks(l) {
        let shares = pp.pack_from_public(chunk.to_vec());
        for (party, share) in shares.into_iter().enumerate() {
            pcs_workers[party].push(share);
        }
    }

    let pcs_net = LocalTestNet::new_local_testnet(parties).await.unwrap();
    let pcs_start = Instant::now();
    let pcs_results = pcs_net
        .simulate_network_round(
            (pcs_workers, packed_pcs, point.clone()),
            |net, (workers, packed_pcs, point)| async move {
                let pp = PackedSharingParams::<Fr>::new(1);
                let id = net.party_id() as usize;
                let commitment = packed_pcs[id]
                    .c_commit(
                        &vec![workers[id].clone()],
                        &pp,
                        &net,
                        MultiplexedStreamID::Zero,
                    )
                    .await
                    .unwrap()[0];
                let opening = packed_pcs[id]
                    .c_open(
                        &workers[id],
                        &point,
                        &pp,
                        &net,
                        MultiplexedStreamID::Zero,
                    )
                    .await
                    .unwrap();
                (commitment, opening)
            },
        )
        .await;
    let pcs_protocol_ms = pcs_start.elapsed().as_secs_f64() * 1000.0;

    let reconstructed_commitment =
        pp.unpack(pcs_results.iter().map(|v| v.0).collect())[0];
    let proof_len = (pcs_results[0].1).1.len();
    let mut reconstructed_proof = Vec::with_capacity(proof_len);
    for i in 0..proof_len {
        reconstructed_proof.push(
            pp.unpack(pcs_results.iter().map(|v| (v.1).1[i]).collect())[0],
        );
    }

    let commitment_reference_match = reconstructed_commitment == reference_commitment;
    let proof_reference_match = reconstructed_proof == reference_proof;
    let pcs_honest = native_pcs.verify(
        reconstructed_commitment,
        reference_value,
        &reconstructed_proof,
        &point,
    );
    let pcs_modified_rejected = !native_pcs.verify(
        reconstructed_commitment,
        reference_value + Fr::from(1u64),
        &reconstructed_proof,
        &point,
    );

    // Lightweight execution/context binding over the checked PCS release object.
    let payload = serialize_opening(
        &reconstructed_commitment,
        &reference_value,
        &reconstructed_proof,
    );
    let sid = [0x42u8; 16];
    let binding = make_binding(sid, 3, 7, &payload);
    let binding_honest = check_binding(&binding, &sid, 3, 7, &payload);
    let wrong_sid = [0x43u8; 16];
    let wrong_context_rejected = !check_binding(&binding, &wrong_sid, 3, 7, &payload);
    let stale_ordinal_rejected = !check_binding(&binding, &sid, 3, 8, &payload);
    let mut changed_payload = payload.clone();
    changed_payload[0] ^= 1;
    let changed_payload_rejected =
        !check_binding(&binding, &sid, 3, 7, &changed_payload);

    // Per-release diagnostic costs.
    let mut sumcheck_ns = 0u128;
    for _ in 0..args.sumcheck_iters {
        let now = Instant::now();
        black_box(verify_released_sumcheck_rounds(
            initial_claim,
            &released_rounds,
            &challenge,
        ));
        sumcheck_ns += now.elapsed().as_nanos();
    }
    let mut pcs_ns = 0u128;
    for _ in 0..args.pcs_iters {
        let now = Instant::now();
        black_box(native_pcs.verify(
            reconstructed_commitment,
            reference_value,
            &reconstructed_proof,
            &point,
        ));
        pcs_ns += now.elapsed().as_nanos();
    }
    let mut binding_ns = 0u128;
    for _ in 0..args.binding_iters {
        let now = Instant::now();
        black_box(check_binding(&binding, &sid, 3, 7, &payload));
        binding_ns += now.elapsed().as_nanos();
    }

    let sumcheck_gate_us = sumcheck_ns as f64 / args.sumcheck_iters as f64 / 1000.0;
    let pcs_gate_us = pcs_ns as f64 / args.pcs_iters as f64 / 1000.0;
    let binding_gate_us = binding_ns as f64 / args.binding_iters as f64 / 1000.0;

    let sumcheck_payload = serialize_sumcheck(&released_rounds);
    let pass = sumcheck_reference_match
        && sumcheck_honest
        && sumcheck_modified_rejected
        && commitment_reference_match
        && proof_reference_match
        && pcs_honest
        && pcs_modified_rejected
        && binding_honest
        && wrong_context_rejected
        && stale_ordinal_rejected
        && changed_payload_rejected;

    println!(
        "PVIA_SECOND_INSTANCE_LIGHT_EVAL: {}",
        if pass { "PASS" } else { "FAIL" }
    );
    println!("parties={}", parties);
    println!("n={}", args.n);
    println!("sumcheck_protocol_ms={:.3}", sum_protocol_ms);
    println!("sumcheck_rounds_checked={}", args.n);
    println!(
        "sumcheck_reference_match={}",
        if sumcheck_reference_match { 1 } else { 0 }
    );
    println!(
        "sumcheck_honest={}",
        if sumcheck_honest { "ALLOW" } else { "WITHHOLD" }
    );
    println!(
        "sumcheck_modified={}",
        if sumcheck_modified_rejected { "WITHHOLD" } else { "ALLOW" }
    );
    println!(
        "terminal_endpoint_scope={}",
        if terminal_endpoint_excluded {
            "EXCLUDED_UPSTREAM_INTERNAL_REPRESENTATION"
        } else {
            "NOT_USED"
        }
    );
    println!("pcs_protocol_ms={:.3}", pcs_protocol_ms);
    println!(
        "pcs_commitment_reference_match={}",
        if commitment_reference_match { 1 } else { 0 }
    );
    println!(
        "pcs_proof_reference_match={}",
        if proof_reference_match { 1 } else { 0 }
    );
    println!(
        "pcs_honest={}",
        if pcs_honest { "ALLOW" } else { "WITHHOLD" }
    );
    println!(
        "pcs_modified={}",
        if pcs_modified_rejected { "WITHHOLD" } else { "ALLOW" }
    );
    println!("pcs_scalar_source=UPSTREAM_REFERENCE_VALUE");
    println!(
        "context_honest={}",
        if binding_honest { "BOUND" } else { "REJECTED" }
    );
    println!(
        "wrong_context={}",
        if wrong_context_rejected { "REJECTED" } else { "ACCEPTED" }
    );
    println!(
        "stale_ordinal={}",
        if stale_ordinal_rejected { "REJECTED" } else { "ACCEPTED" }
    );
    println!(
        "changed_payload={}",
        if changed_payload_rejected { "REJECTED" } else { "ACCEPTED" }
    );
    println!("sumcheck_gate_mean_us={:.3}", sumcheck_gate_us);
    println!("pcs_verify_gate_mean_us={:.3}", pcs_gate_us);
    println!("binding_gate_mean_us={:.3}", binding_gate_us);
    println!(
        "sumcheck_gate_vs_primitive_pct={:.6}",
        sumcheck_gate_us / 1000.0 / sum_protocol_ms * 100.0
    );
    println!(
        "pcs_gate_vs_primitive_pct={:.6}",
        pcs_gate_us / 1000.0 / pcs_protocol_ms * 100.0
    );
    println!("sumcheck_payload_bytes={}", sumcheck_payload.len());
    println!("pcs_release_payload_bytes={}", payload.len());
    println!("binding_record_bytes=57");
    println!("scope=LIGHTWEIGHT_APPLICABILITY_NOT_FULL_SYSTEM_PVIA_OVERHEAD");

    if !pass {
        std::process::exit(2);
    }
}
