CREATE TABLE media_index (
    id BIGSERIAL PRIMARY KEY,
    path TEXT,
    hash BYTEA NOT NULL,
    synced BOOLEAN NOT NULL,
    lost BOOLEAN NOT NULL
);

CREATE UNIQUE INDEX idx_unique_path ON media_index (path) WHERE (synced and not lost);
CREATE INDEX idx_path ON media_index (path);
CREATE INDEX idx_hash ON media_index (hash);
