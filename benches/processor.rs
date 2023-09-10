use criterion::{criterion_group, criterion_main, Criterion, black_box, BatchSize};
use exchange::{order::util::{DEFAULT_PAIR, generate}, engine::Engine};

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
