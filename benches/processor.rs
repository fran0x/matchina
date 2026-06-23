use compact_str::CompactString;
use criterion::{BatchSize, Criterion, Throughput, black_box, criterion_group, criterion_main};
use matchina::{
    engine::Engine,
    order::{OrderRequest, OrderSide, util::DEFAULT_PAIR},
};
use rand::{Rng, SeedableRng, rngs::StdRng};
use rust_decimal::Decimal;

const WORKLOAD_SIZE: usize = 20_000;
const MID_PRICE_CENTS: i64 = 10_000;

pub fn process(c: &mut Criterion) {
    let workloads = [
        ("normal", normal_workload(WORKLOAD_SIZE)),
        ("same_price_fifo", same_price_fifo_workload(WORKLOAD_SIZE)),
        ("cancel_heavy", cancel_heavy_workload(WORKLOAD_SIZE)),
        ("market_sweep", market_sweep_workload(WORKLOAD_SIZE)),
        ("flash_crash", flash_crash_workload(WORKLOAD_SIZE)),
    ];

    let mut group = c.benchmark_group("process_batch");
    group.sample_size(10);

    for (name, workload) in workloads {
        group.throughput(Throughput::Elements(workload.len() as u64));
        group.bench_function(name, |b| {
            b.iter_batched(
                || workload.clone(),
                |orders| process_orders(black_box(orders)),
                BatchSize::LargeInput,
            );
        });
    }

    group.finish();
}

fn process_orders(orders: Vec<OrderRequest>) {
    let mut engine = Engine::new(DEFAULT_PAIR);
    for order in orders {
        black_box(engine.process(black_box(order))).unwrap();
    }
    black_box(engine.orderbook());
}

fn normal_workload(size: usize) -> Vec<OrderRequest> {
    let mut rng = StdRng::seed_from_u64(0xA11CE);
    let mut orders = Vec::with_capacity(size);

    for id in 1..=size as u64 {
        let side = if rng.gen_bool(0.5) {
            OrderSide::Bid
        } else {
            OrderSide::Ask
        };
        let offset = rng.gen_range(-120..=120);
        let price = MID_PRICE_CENTS + offset;
        let quantity = rng.gen_range(1..=25);
        let limit_price = if rng.gen_bool(0.9) { Some(cents(price)) } else { None };

        orders.push(create(id, side, limit_price, quantity));
    }

    orders
}

fn same_price_fifo_workload(size: usize) -> Vec<OrderRequest> {
    let mut orders = Vec::with_capacity(size);
    let resting_orders = size / 2;
    let mut id = 1;

    for _ in 0..resting_orders {
        orders.push(create(id, OrderSide::Bid, Some(cents(MID_PRICE_CENTS)), 10));
        id += 1;
    }

    while orders.len() < size {
        orders.push(create(id, OrderSide::Ask, Some(cents(MID_PRICE_CENTS)), 50));
        id += 1;
    }

    orders
}

fn cancel_heavy_workload(size: usize) -> Vec<OrderRequest> {
    let mut rng = StdRng::seed_from_u64(0xCA7CE1);
    let mut orders = Vec::with_capacity(size);
    let mut live_order_ids = Vec::with_capacity(size);
    let mut id = 1;

    while orders.len() < size {
        if !live_order_ids.is_empty() && rng.gen_bool(0.45) {
            let index = rng.gen_range(0..live_order_ids.len());
            let order_id = live_order_ids.swap_remove(index);
            orders.push(OrderRequest::Cancel { order_id });
        } else {
            let price = MID_PRICE_CENTS - rng.gen_range(1..=500);
            live_order_ids.push(id);
            orders.push(create(id, OrderSide::Bid, Some(cents(price)), rng.gen_range(1..=20)));
            id += 1;
        }
    }

    orders
}

fn market_sweep_workload(size: usize) -> Vec<OrderRequest> {
    let mut orders = Vec::with_capacity(size);
    let resting_orders = size * 3 / 4;
    let mut id = 1;

    for level in 0..resting_orders {
        let price = MID_PRICE_CENTS + 1 + (level % 200) as i64;
        orders.push(create(id, OrderSide::Ask, Some(cents(price)), 10));
        id += 1;
    }

    while orders.len() < size {
        orders.push(create(id, OrderSide::Bid, None, 100));
        id += 1;
    }

    orders
}

fn flash_crash_workload(size: usize) -> Vec<OrderRequest> {
    let mut orders = Vec::with_capacity(size);
    let resting_orders = size * 4 / 5;
    let mut id = 1;

    for level in 0..resting_orders {
        let price = MID_PRICE_CENTS - 1 - (level % 500) as i64;
        orders.push(create(id, OrderSide::Bid, Some(cents(price)), 10));
        id += 1;
    }

    while orders.len() < size {
        orders.push(create(id, OrderSide::Ask, None, 250));
        id += 1;
    }

    orders
}

fn create(id: u64, side: OrderSide, limit_price: Option<Decimal>, quantity: i64) -> OrderRequest {
    OrderRequest::Create {
        account_id: CompactString::new_inline("bench"),
        order_id: id,
        pair: CompactString::new_inline(DEFAULT_PAIR),
        side,
        limit_price,
        quantity: Decimal::from(quantity),
    }
}

fn cents(value: i64) -> Decimal {
    Decimal::new(value, 2)
}

criterion_group!(benches, process);
criterion_main!(benches);
