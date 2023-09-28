![Rust](https://github.com/flopezlasanta/merx-rs/actions/workflows/rust.yml/badge.svg)
![Audit](https://github.com/flopezlasanta/merx-rs/actions/workflows/audit.yml/badge.svg)

# merx-rs

This is a minimalistic matching engine designed for a crypto exchange. It supports various order types including limit orders, market orders, and order features such as Immediate-Or-Cancel (IOC), Fill-Or-Kill (FOK), and Post-Only orders.

## Features

- **Limit Orders:** Traders can place buy or sell orders at specified price levels, ensuring their orders are executed at their desired prices or better.
- **Market Orders:** Traders can place orders to be executed at the current market price, guaranteeing an immediate fill.
- **Immediate-Or-Cancel (IOC):** IOC orders are designed for immediate execution. Any portion of an IOC order that cannot be filled immediately is canceled.
- **Fill-Or-Kill (FOK):** FOK orders demand complete execution. If the entire order cannot be filled immediately, it is canceled.
- **Post-Only Orders:** Post-Only orders are added to the order book and are only executed as maker orders, ensuring no additional fees as a taker.

## Usage

To run the simulation with this matching engine, simply execute the following command:

```shell
just run
```

[Just](https://github.com/casey/just) is used to manage various build and development tasks, and you can explore the available options with:

```shell
just
```

## Contributing

Contributions from the community as welcomed! Feel free to submit bug reports, feature requests, or even pull requests to enhance the matching engine.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

---

Feel free to customize and expand upon this README based on your project's specific details and requirements.
