use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};
use bitcoin::secp256k1::{Secp256k1, SecretKey};
use bitcoin::{PrivateKey, PublicKey, Address, Network};
use ibig::ubig;
use std::collections::HashSet;

fn generate_address_benchmark(secret_key: &SecretKey) -> Address {
    let secp = Secp256k1::new();
    let private_key = PrivateKey {
        compressed: true,
        network: Network::Bitcoin.into(),
        inner: *secret_key,
    };
    let public_key = PublicKey::from_private_key(&secp, &private_key);
    Address::p2pkh(&public_key, Network::Bitcoin)
}

fn benchmark_address_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("address_generation");
    
    // Test avec différentes tailles de clés
    for size in [1000, 10000, 100000].iter() {
        group.bench_with_input(
            BenchmarkId::new("generate_addresses", size),
            size,
            |b, &size| {
                b.iter(|| {
                    for i in 1u32..=size {
                        let key_bytes = i.to_be_bytes();
                        let mut padded = [0u8; 32];
                        padded[32 - key_bytes.len()..].copy_from_slice(&key_bytes);
                        
                        if let Ok(secret_key) = SecretKey::from_slice(&padded) {
                            let _address = generate_address_benchmark(&secret_key);
                        }
                    }
                });
            },
        );
    }
    
    group.finish();
}

fn benchmark_key_patterns(c: &mut Criterion) {
    let mut group = c.benchmark_group("key_patterns");
    
    let base_key = ubig!(0x20000000000000000u128);
    
    group.bench_function("generate_patterns", |b| {
        b.iter(|| {
            let mut patterns = Vec::new();
            
            // Pattern original
            patterns.push(base_key.clone());
            
            // Patterns avec offsets
            for offset in [1, 2, 3, 5, 8, 13, 21, 34, 55, 89] {
                patterns.push(&base_key + offset);
                if base_key > ubig!(offset) {
                    patterns.push(&base_key - offset);
                }
            }
            
            // Multiplication par facteurs premiers
            for factor in [2, 3, 5, 7, 11, 13] {
                patterns.push(&base_key * factor);
            }
            
            patterns
        });
    });
    
    group.finish();
}

fn benchmark_hashset_lookup(c: &mut Criterion) {
    let mut group = c.benchmark_group("hashset_lookup");
    
    // Créer un HashSet de test
    let mut test_addresses = HashSet::new();
    for i in 1..=10000 {
        test_addresses.insert(format!("1Address{}", i));
    }
    
    group.bench_function("lookup_existing", |b| {
        b.iter(|| {
            test_addresses.contains("1Address5000")
        });
    });
    
    group.bench_function("lookup_nonexisting", |b| {
        b.iter(|| {
            test_addresses.contains("1NonExistentAddress")
        });
    });
    
    group.finish();
}

criterion_group!(
    benches,
    benchmark_address_generation,
    benchmark_key_patterns,
    benchmark_hashset_lookup
);
criterion_main!(benches);