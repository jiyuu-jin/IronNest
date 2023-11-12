fmt:
  cargo +nightly fmt -- --config group_imports=StdExternalCrate,imports_granularity=One

clippy:
  cargo clippy

lint: fmt clippy
