fmt: leptos-fmt
  cargo +nightly fmt -- --config group_imports=StdExternalCrate,imports_granularity=One

leptos-fmt:
  leptosfmt .

clippy:
  cargo clippy --features=ssr
  cargo clippy --features=hydrate

lint: fmt clippy

dev: lint
  RUST_BACKTRACE=1 cargo leptos watch

wipe:
  docker compose down postgres
  rm -rf docker-data/postgres

infra:
  docker compose up postgres
