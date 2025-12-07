CREATE TYPE public.access_type AS ENUM('none', 'read', 'write', 'admin');
CREATE TYPE public.user_type AS ENUM('normal', 'temporary');

CREATE TABLE users (
    id uuid PRIMARY KEY,
    created_at timestamptz DEFAULT now(),
    last_login timestamptz DEFAULT now(),
    user_type user_type NOT NULL,
    slug varchar(128) UNIQUE NOT NULL,
    name varchar(128) NOT NULL,
    email varchar(255) NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    pronouns varchar(32) NOT NULL DEFAULT '',
    organization varchar(64) NOT NULL DEFAULT '',
    location varchar(64) NOT NULL DEFAULT '',
    website varchar(255) NOT NULL DEFAULT '',

    default_main_branch varchar(128) NOT NULL DEFAULT 'main',
    password_hash TEXT NOT NULL,
    CONSTRAINT users_slug_format CHECK (slug ~ '^[a-z0-9_.-]+$')
);
CREATE INDEX users_last_login_idx ON users USING btree (last_login);
CREATE INDEX users_user_type_idx ON users USING btree (user_type);
CREATE UNIQUE INDEX users_email_lower_idx ON users (lower(email));


CREATE TABLE sessions (
    id uuid PRIMARY KEY,
    expires_at timestamptz DEFAULT now() + interval '1 month',
    owner uuid NOT NULL,
    CONSTRAINT sessions_user FOREIGN KEY (owner) REFERENCES public.users(id) ON DELETE cascade ON UPDATE no action
);
CREATE INDEX sessions_owner_idx ON sessions USING btree (owner);
CREATE INDEX sessions_expires_at_idx ON sessions USING btree (expires_at);


CREATE TABLE projects (
    id uuid PRIMARY KEY,
    created_at timestamptz DEFAULT now(),
    owner uuid NOT NULL,
    public_access access_type NOT NULL DEFAULT 'none',

    slug varchar(128) NOT NULL,
    name varchar(128) NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    website varchar(255) NOT NULL DEFAULT '',

    main_branch varchar(128) NOT NULL DEFAULT 'main',
    newest_commit_time timestamptz,

    CONSTRAINT projects_owner FOREIGN KEY (owner) REFERENCES public.users(id) ON DELETE cascade ON UPDATE no action,
    CONSTRAINT projects_owner_slug_unique UNIQUE (owner, slug),
    CONSTRAINT projects_slug_format CHECK (slug ~ '^[a-z0-9_.-]+$')
);
CREATE INDEX projects_owner_idx ON projects USING btree (owner);


CREATE TABLE ssh_keys (
    public_key text PRIMARY KEY,
    user_id uuid NOT NULL,
    hostname text,
    created_at timestamptz DEFAULT now(),

    CONSTRAINT ssh_keys_user FOREIGN KEY (user_id) REFERENCES public.users(id) ON DELETE cascade ON UPDATE no action
);
CREATE INDEX ssh_keys_user_id_idx ON ssh_keys(user_id);
