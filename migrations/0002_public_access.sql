ALTER TABLE projects
    ADD COLUMN public_access access_type NOT NULL DEFAULT 'none';

UPDATE projects
SET public_access = COALESCE(default_access, 'none');
