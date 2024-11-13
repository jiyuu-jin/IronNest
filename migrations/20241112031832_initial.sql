-- SQLBook: Code
CREATE TYPE device_type AS ENUM ('smart-plug', 'smart-light', 'ring-doorbell', 'roku-tv', 'stoplight');

CREATE TABLE devices (
    id BIGSERIAL PRIMARY KEY,
    name TEXT NOT NULL,
    device_type device_type NOT NULL,
    ip TEXT NOT NULL UNIQUE,
    battery_percentage INT8,
    power_state INT8 NOT NULL,
    last_seen TIMESTAMPTZ NOT NULL
);

CREATE TABLE events (
    id BIGSERIAL PRIMARY KEY,
    schedule TEXT NOT NULL,
    function TEXT NOT NULL,
    parameters TEXT NOT NULL
);

CREATE TABLE auth (
    id BIGSERIAL PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    hardware_id TEXT,
    auth_token TEXT,
    refresh_token TEXT,
    last_login TIMESTAMPTZ,
    captcha TEXT
);

CREATE TABLE ring_cameras (
    id BIGSERIAL PRIMARY KEY,
    description TEXT NOT NULL,
    snapshot_image TEXT NOT NULL,
    snapshot_timestamp INT8 NOT NULL,
    health INT8 NOT NULL
);

CREATE TABLE ring_video_item (
    ding_id TEXT PRIMARY KEY,
    camera_id INT8,
    created_at INT8 NOT NULL,
    hq_url TEXT NOT NULL
);

CREATE TABLE ingredient (
    id BIGSERIAL PRIMARY KEY,
    name TEXT NOT NULL
);

CREATE TABLE recipe (
    id BIGSERIAL PRIMARY KEY,
    name TEXT NOT NULL
);

CREATE TABLE recipe_ingredient (
    id BIGSERIAL PRIMARY KEY,
    recipe_id INTEGER NOT NULL,
    ingredient_id INTEGER NOT NULL,
    amount INTEGER NOT NULL,
    FOREIGN KEY(recipe_id) REFERENCES recipe(id),
    FOREIGN KEY(ingredient_id) REFERENCES ingredient(id)
);

CREATE TABLE amounts (
    id BIGSERIAL PRIMARY KEY,
    ingredient_id INTEGER NOT NULL,
    FOREIGN KEY(ingredient_id) REFERENCES ingredient(id)
);
