use std::iter;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use ff::Field;
use rand_core::OsRng;
use zcash_note_encryption::batch;
use zcash_primitives::{
    consensus::{NetworkUpgrade::Canopy, Parameters, TEST_NETWORK},
    memo::MemoBytes,
    sapling::{
        note_encryption::{
            try_sapling_compact_note_decryption, try_sapling_note_decryption,
            PreparedIncomingViewingKey, SaplingDomain,
        },
        prover::mock::MockTxProver,
        value::NoteValue,
        Diversifier, PaymentAddress, SaplingIvk,
    },
    transaction::components::sapling::{
        builder::SaplingBuilder, CompactOutputDescription, GrothProofBytes, OutputDescription,
    },
};

#[cfg(unix)]
use pprof::criterion::{Output, PProfProfiler};

fn bench_note_decryption(c: &mut Criterion) {
    let mut rng = OsRng;
    let height = TEST_NETWORK.activation_height(Canopy).unwrap();

    let valid_ivk = SaplingIvk(jubjub::Fr::random(&mut rng));
    let invalid_ivk = SaplingIvk(jubjub::Fr::random(&mut rng));

    // Construct a Sapling output.
    let output: OutputDescription<GrothProofBytes> = {
        let diversifier = Diversifier([0; 11]);
        let pk_d = diversifier.g_d().unwrap() * valid_ivk.0;
        let pa = PaymentAddress::from_parts(diversifier, pk_d).unwrap();

        let mut builder = SaplingBuilder::new(TEST_NETWORK, height);
        builder
            .add_output(
                &mut rng,
                None,
                pa,
                NoteValue::from_raw(100),
                MemoBytes::empty(),
            )
            .unwrap();
        let bundle = builder
            .build(&MockTxProver, &mut (), &mut rng, height, None)
            .unwrap()
            .unwrap();
        bundle.shielded_outputs()[0].clone()
    };

    let valid_ivk = PreparedIncomingViewingKey::new(&valid_ivk);
    let invalid_ivk = PreparedIncomingViewingKey::new(&invalid_ivk);

    {
        let mut group = c.benchmark_group("sapling-note-decryption");
        group.throughput(Throughput::Elements(1));

        group.bench_function("valid", |b| {
            b.iter(|| {
                try_sapling_note_decryption(&TEST_NETWORK, height, &valid_ivk, &output).unwrap()
            })
        });

        group.bench_function("invalid", |b| {
            b.iter(|| try_sapling_note_decryption(&TEST_NETWORK, height, &invalid_ivk, &output))
        });

        let compact = CompactOutputDescription::from(output.clone());

        group.bench_function("compact-valid", |b| {
            b.iter(|| {
                try_sapling_compact_note_decryption(&TEST_NETWORK, height, &valid_ivk, &compact)
                    .unwrap()
            })
        });

        group.bench_function("compact-invalid", |b| {
            b.iter(|| {
                try_sapling_compact_note_decryption(&TEST_NETWORK, height, &invalid_ivk, &compact)
            })
        });
    }

    {
        let mut group = c.benchmark_group("sapling-batch-note-decryption");

        for (nivks, noutputs) in [(1, 10), (10, 1), (10, 10), (50, 50)] {
            let invalid_ivks: Vec<_> = iter::repeat(invalid_ivk.clone()).take(nivks).collect();
            let valid_ivks: Vec<_> = iter::repeat(valid_ivk.clone()).take(nivks).collect();

            let outputs: Vec<_> = iter::repeat(output.clone())
                .take(noutputs)
                .map(|output| (SaplingDomain::for_height(TEST_NETWORK, height), output))
                .collect();

            group.bench_function(
                BenchmarkId::new(format!("valid-{}", nivks), noutputs),
                |b| b.iter(|| batch::try_note_decryption(&valid_ivks, &outputs)),
            );

            group.bench_function(
                BenchmarkId::new(format!("invalid-{}", nivks), noutputs),
                |b| b.iter(|| batch::try_note_decryption(&invalid_ivks, &outputs)),
            );

            let compact: Vec<_> = outputs
                .into_iter()
                .map(|(domain, output)| (domain, CompactOutputDescription::from(output)))
                .collect();

            group.bench_function(
                BenchmarkId::new(format!("compact-valid-{}", nivks), noutputs),
                |b| b.iter(|| batch::try_compact_note_decryption(&valid_ivks, &compact)),
            );

            group.bench_function(
                BenchmarkId::new(format!("compact-invalid-{}", nivks), noutputs),
                |b| b.iter(|| batch::try_compact_note_decryption(&invalid_ivks, &compact)),
            );
        }
    }
}

#[cfg(unix)]
criterion_group! {
    name = benches;
    config = Criterion::default().with_profiler(PProfProfiler::new(100, Output::Flamegraph(None)));
    targets = bench_note_decryption
}
#[cfg(not(unix))]
criterion_group!(benches, bench_note_decryption);
criterion_main!(benches);
