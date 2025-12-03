ALTER TABLE ssh_keys ADD COLUMN hostname TEXT;

CREATE UNIQUE INDEX ssh_keys_public_key_idx ON ssh_keys(public_key);