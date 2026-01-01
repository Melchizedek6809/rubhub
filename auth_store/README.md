# RubHub Auth Store

This crate provides an interface for storing and retrieving
sensitive data, mainly used for authentication.

So for example to look up the user for a given session you
can call `AuthStore::get_user_by_session`. To persist
any changes you can call functions like `User::save`.

All of this is implemented via an append-only log that
is stored in `data/auth.ndjson` by default. On startup
we read this file and parse it line by line updating an in-memory
representation (currently a DashMap), so if a user gets defined
multiple times in the file later entries overwrite what came before.

This way we can simplify things quite a bit, since whenever something
changed we only need to append a single line to the file. To delete
an entry we have variants that specify which user/session to delete.

One limitation is that this means that every single user/session token needs
to be in memory at all times, since a single `User` record is just a couple hundred
bytes though this shouldn't be much of an issue in practice. This however means that
we can look up the user for a given session without doing any filesystem operations.
