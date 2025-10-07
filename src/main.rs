mod slab;

#[macro_export]
macro_rules! timed {
    ($block:block) => {{
        let start = std::time::Instant::now();
        std::hint::black_box($block);
        let end = start.elapsed();
        end
    }};
}

fn main() {
    const COUNT: usize = 1 << 24;
    let mut bitslab = slab::BitmapSlab2::<usize>::with_capacity(COUNT);
}