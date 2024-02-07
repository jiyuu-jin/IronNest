# IronNest

IronNest is a home automation system designed to integrate with various smart devices. The current implementation integrates with Ring, Alexa, TP-Link, and Roku devices, all controllable by GPT-3.5-turbo-1106.

## Features

- Integration with Ring doorbells, cameras, etc.
- Fetch authentication tokens from Ring.
- Get details of Ring, Alexa, Roku & TP-Link devices.
- Obtain socket tickets from Ring.
- Automatic local network discovery of devices.
- Chain multiple commands and control devices by `type`, `name`, or by `ip`.

<img width="1728" alt="image" src="https://github.com/jiyuu-jin/IronNest/assets/19313806/51236523-af7b-463f-9b84-48251751abed">

<img width="1727" alt="image" src="https://github.com/jiyuu-jin/IronNest/assets/19313806/cd0e9216-d3bc-418b-b888-5fe294a0a5a4">


## Project Structure

The project has the following main files:
- `main.rs`: The main entry point, sets up the server, routes, and middleware.
- `intergations.rs`: Contains the utility functions & types for an intergartion such as the `RingRestClient` for making requests to the Ring API.
  - `client`: Contains an integrations client & authentication logic.
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
