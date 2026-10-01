use ark_bls12_381::Bls12_381;
use ark_ec::pairing::Pairing;
use ark_std::UniformRand;
use clap::Parser;
use dist_primitive::dpoly_comm::PolynomialCommitmentCub;
use mpc_net::{LocalTestNet, MPCNet, MultiplexedStreamID};
use secret_sharing::pss::PackedSharingParams;

type E = Bls12_381;
type Fr = <E as Pairing>::ScalarField;
type G1 = <E as Pairing>::G1;

#[derive(Parser)]
struct Cli {
    #[arg(long, default_value_t = 2)]
    l: usize,
    #[arg(long, default_value_t = 8)]
    n: usize,
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let args = Cli::parse();
    let pp = PackedSharingParams::<Fr>::new(args.l);
    let parties = pp.n;
    let rng = &mut ark_std::test_rng();
    let evals: Vec<Fr> = (0..(1usize << args.n)).map(|_| Fr::rand(rng)).collect();
    let trapdoor: Vec<Fr> = (0..args.n).map(|_| Fr::rand(rng)).collect();
    let g1 = G1::rand(rng);
    let g2 = <E as Pairing>::G2::rand(rng);
    let cub = PolynomialCommitmentCub::<E>::new(g1, g2, trapdoor);
    let native = cub.mature();
    let packed = cub.to_packed(&pp);
    let expected = native.commit(&evals);

    let mut workers = vec![Vec::new(); parties];
    for chunk in evals.chunks(args.l) {
        let shares = pp.pack_from_public(chunk.to_vec());
        for (party, share) in shares.into_iter().enumerate() {
            workers[party].push(share);
        }
    }

    let net = LocalTestNet::new_local_testnet(parties).await.unwrap();
    let result = net
        .simulate_network_round((workers, packed, args.l), |net, (workers, packed, l)| async move {
            let pp = PackedSharingParams::<Fr>::new(l);
            let id = net.party_id() as usize;
            packed[id]
                .c_commit(
                    &vec![workers[id].clone()],
                    &pp,
                    &net,
                    MultiplexedStreamID::Zero,
                )
                .await
                .unwrap()[0]
        })
        .await;
    let reconstructed = pp.unpack(result);
    let pass = reconstructed.iter().all(|x| *x == expected);
    println!(
        "PVIA_SECOND_INSTANCE_COMMIT_SANITY: {}",
        if pass { "PASS" } else { "FAIL" }
    );
    println!("l={}", args.l);
    println!("parties={}", parties);
    println!("n={}", args.n);
    println!("reference_match={}", if pass { 1 } else { 0 });
    if !pass {
        std::process::exit(2);
    }
}
