CREATE TABLE ipld_blobs (
    cid BYTEA PRIMARY KEY,
    content BYTEA NOT NULL
);

CREATE TABLE mish_states (
    name VARCHAR(255) PRIMARY KEY,
    state JSONB NOT NULL
);
