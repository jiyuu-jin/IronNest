fmt:
  cargo +nightly fmt -- --config group_imports=StdExternalCrate,imports_granularity=One

clippy:
  cargo clippy --all-features

lint: fmt clippy
