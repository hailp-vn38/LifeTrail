CREATE EXTENSION IF NOT EXISTS postgis;

CREATE TABLE users (
    id UUID PRIMARY KEY,
    display_name TEXT NOT NULL CHECK (char_length(display_name) > 0),
    timezone TEXT NOT NULL CHECK (char_length(timezone) > 0),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Phase 1 has exactly one deployment Owner while retaining the future users → devices model.
CREATE UNIQUE INDEX users_single_owner_idx ON users ((true));

CREATE TABLE devices (
    id UUID PRIMARY KEY,
    owner_user_id UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    name TEXT NOT NULL CHECK (char_length(name) > 0),
    token_digest BYTEA NOT NULL UNIQUE CHECK (octet_length(token_digest) = 32),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX devices_owner_user_id_idx ON devices(owner_user_id);
