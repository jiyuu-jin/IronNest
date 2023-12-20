# IronNest

IronNest is a home automation system designed to integrate with various smart devices. The current implementation integrates with the Ring API to control ring devices.

## Features

- Integration with Ring doorbells, cameras, etc.
- Fetch authentication tokens from Ring.
- Get details of Ring devices.
- Obtain socket tickets from Ring.
- Built using Axum framework for a lightweight and efficient server.

![image](https://github.com/jiyuu-jin/IronNest/assets/19313806/c4426aed-3793-4e03-9973-87893cf2d8d3)

## Project Structure

The project has the following main files:
- `main.rs`: The main entry point, sets up the server, routes, and middleware.
- `utils.rs`: Contains utility functions and the `RingRestClient` for making requests to the Ring API.
- `handlers`: Contains handlers for various routes.
- `types`: Contains data structures and types used across the project.

## Environment Setup

Before running the project, ensure you have the following environment variables set:
- `RING_REFRESH_TOKEN`: Your Ring refresh token.
- `RING_AUTH_TOKEN`: Your Ring authentication token.

These can be set in a `.env` file at the root of your project.

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

- `GET /`: Health check endpoint. Returns "Iron Nest is running!".
- `GET /api/ring`: Fetches details related to the Ring doorbell.
- `GET /api/ring/auth`: Authenticates with the Ring API.

## Contributions

Contributions to IronNest are welcome! Please fork the repository, make your changes, and submit a pull request.

## License

[MIT License](LICENSE)

## Acknowledgements

Special thanks to the Rust community and the creators of the Axum framework for providing valuable resources.
