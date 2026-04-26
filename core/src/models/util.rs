use rand::distr::Uniform;
use rand::rng;
use rand::rngs::ThreadRng;
use rand::RngExt;

const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";

#[inline]
pub fn random_id<const N: usize>() -> [u8; N] {
    random_id_with_rng::<N>(&mut rng())
}

#[inline]
pub fn random_id_with_rng<const N: usize>(rng: &mut ThreadRng) -> [u8; N] {
    let uniform = Uniform::new(0, CHARSET.len()).unwrap();
    let mut buf = [0u8; N];
    for b in buf.iter_mut() {
        *b = CHARSET[rng.sample(uniform)];
    }
    buf
}
