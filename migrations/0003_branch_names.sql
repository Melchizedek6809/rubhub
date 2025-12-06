ALTER TABLE projects
    ADD COLUMN main_branch varchar(128) NOT NULL DEFAULT 'main',
    DROP COLUMN default_branch;

ALTER TABLE users
    ADD COLUMN default_main_branch varchar(128) NOT NULL DEFAULT 'main';