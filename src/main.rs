use slabmap::SlabMap;

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
    const COUNT: usize = 1 << 10;
    let mut bitslab  = SlabMap::<usize, usize>::with_capacity(COUNT);

    let mut indices = Vec::new();
    for i in 0..44 {
        let q = bitslab.insert(i, i*40000);
        println!("inserted: {:?}:{:?} -> {:?}", i, i*40000, q);
        indices.push(q);
    }

    for i in 0..5 {
        let q = bitslab.remove(&(i*10));
        println!("removed: {:?} -> {:?}", i, q);
    }

    for i in 0..44 {
        let q = bitslab.insert(i*2, i*2*40000);
        println!("inserted: {:?}:{:?} -> {:?}", i, i*2*40000, q);
        indices.push(q);
    }

}