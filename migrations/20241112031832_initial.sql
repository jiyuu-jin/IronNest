-- SQLBook: Code
CREATE TYPE device_type AS ENUM ('kasa-plug', 'kasa-light', 'kasa-dimmer', 'kasa-power-strip', 'tuya-light', 'tuya-grow-light', 'ring-doorbell', 'roku-tv', 'stoplight');

CREATE TABLE device (
    id BIGSERIAL PRIMARY KEY,
    name TEXT NOT NULL,
    device_type device_type NOT NULL,
    ip TEXT NOT NULL,
    battery_percentage INT8,
    power_state INT4 NOT NULL,
    last_seen TIMESTAMPTZ NOT NULL,
    mac_address TEXT,
    child_id TEXT,
    coalesced_child_id TEXT GENERATED ALWAYS AS (COALESCE(child_id, '')) STORED,
    CONSTRAINT unique_ip_child_id UNIQUE (ip, coalesced_child_id),
    CONSTRAINT unique_mac_child_id UNIQUE (mac_address, coalesced_child_id)
);

CREATE TABLE actions (
    id BIGSERIAL PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    cron VARCHAR(255) NOT NULL,
    function_name VARCHAR(255) NOT NULL,
    function_args JSONB NOT NULL
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
    snapshot_timestamp TIMESTAMPTZ NOT NULL,
    health INT8 NOT NULL
);

CREATE TABLE ring_video_item (
    ding_id TEXT PRIMARY KEY,
    camera_id INT8,
    created_at TIMESTAMPTZ NOT NULL,
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

create TABLE integration (
    id BIGSERIAL PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    enabled BOOLEAN DEFAULT FALSE,
    image TEXT NOT NULL
);

create TABLE tuya_device_data (
    id INTEGER PRIMARY KEY REFERENCES device(id),
    device_id TEXT,
    local_key TEXT 
)