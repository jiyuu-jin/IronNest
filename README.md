# IronNest

IronNest is a home automation system designed to integrate with various smart devices. The current implementation integrates with the Ring, Alex, TP-Link, and Roku devices, all controllable by GPT-3.5-turbo-1106.

## Features

- Integration with Ring doorbells, cameras, etc.
- Fetch authentication tokens from Ring.
- Get details of Ring, Alexa, Roku & TP-Link devices.
- Obtain socket tickets from Ring.
- Autmotic local network discovery of devices.
- Chain multiple commands and control devices by `type`, `name`, or by `ip`.

![image](https://github.com/jiyuu-jin/IronNest/assets/19313806/c4426aed-3793-4e03-9973-87893cf2d8d3)

## Project Structure

The project has the following main files:
- `main.rs`: The main entry point, sets up the server, routes, and middleware.
- `intergations.rs`: Contains the utility functions & types for an intergartion such as the `RingRestClient` for making requests to the Ring API.
  - `client`: Contains and integrations client & authentication logic.
  - `types`: Contains an intergartions data structures and types.

## Dependencies

```bash
rustup target add wasm32-unknown-unknown
cargo install trunk cargo-leptos leptosfmt just
```

## Running the Project

```bash
just dev
```

## Endpoints

- `GET /`: Main dashboard displaying any number of intergartions".
- `GET /api/ring/keypress`: Submit ring keypresses.
- `GET /login`: Authenticates with the Ring API.

## Contributions

Contributions to IronNest are welcome! Please fork the repository, make your changes, and submit a pull request.

## License

[MIT License](LICENSE)

## Acknowledgements

Special thanks to the Rust community and the creators of the Axum & Leptos frameworks for providing such valuable resources.
