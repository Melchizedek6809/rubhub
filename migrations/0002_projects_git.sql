ALTER TABLE projects
    ADD COLUMN default_branch varchar(128) NOT NULL DEFAULT 'main',
    ADD COLUMN newest_commit_time timestamptz,
    ADD COLUMN newest_commit_hash varchar(40);