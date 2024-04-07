use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};
use matchine::{
    engine::Engine,
    order::util::{generate, DEFAULT_PAIR},
};

pub fn process(c: &mut Criterion) {
    let range = 1..;
    let mut orders = generate(range);

    c.bench_function("process", |b| {
        b.iter_batched(
            || Engine::new(DEFAULT_PAIR),
            |mut engine| {
                let order_request = black_box(orders.next().unwrap());
                black_box(engine.process(order_request))
            },
            BatchSize::SmallInput,
        );
    });
}

criterion_group!(benches, process);
criterion_main!(benches);
