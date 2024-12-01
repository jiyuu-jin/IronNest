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

docker-build-push:
  docker compose build iron_nest
  docker compose push iron_nest

deploy:
  ssh turingpi-1 "cd ~/ironnest && docker compose pull iron_nest && docker compose up -d iron_nest"

build-deploy: docker-build-push deploy

forward-postgres:
  ssh turingpi-1 -N -L 127.0.0.1:5433:127.0.0.1:5433
