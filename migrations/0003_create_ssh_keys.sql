CREATE TABLE ssh_keys (
    id uuid PRIMARY KEY,
    user_id uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    public_key text NOT NULL,
    created_at timestamptz DEFAULT now()
);

CREATE INDEX ssh_keys_user_id_idx ON ssh_keys(user_id);
