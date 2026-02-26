use canvapress::{decode_fill, encode_erase};
use rand::{Rng, SeedableRng};

#[test]
fn roundtrip_random() {
    let mut rng = rand::rngs::StdRng::seed_from_u64(7);
    let sizes = [1024usize, 10240usize, 25600usize];

    for &n in &sizes {
        let mut payload = vec![0u8; n];
        for b in payload.iter_mut() { *b = rng.gen(); }

        let raw = encode_erase(&payload).unwrap();
        let out = decode_fill(&raw).unwrap();

        assert_eq!(out, payload);
    }
}