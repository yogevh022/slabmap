use slabmap::SlabMap;

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
    const COUNT: usize = 1 << 20;

    let mut slabmap: SlabMap<usize, usize> = SlabMap::with_capacity(COUNT);

    let sm = timed!({
        for i in 1..COUNT/2 {
            let base_i = i * 6;
            let q = slabmap.insert(i-1, i);
            let q2 = slabmap.insert(i-2, i);
            let q3 = slabmap.insert(i-3, i);
            let q4 = slabmap.insert(i-4, i);
            let q5 = slabmap.insert(i-5, i);
            unsafe { slabmap.remove_unchecked(&(i-4)) };
            let q6 = slabmap.insert(i-6, i);
            unsafe { slabmap.remove_unchecked(&(i-5)) };
            unsafe { slabmap.remove_unchecked(&(i-2)) };
        }
    });

    println!("slabmap: {:?}", sm);
}