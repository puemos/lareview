# Database Migrations

This folder contains SQL migration files for the LaReview database schema.

## Naming Convention

Migration files follow the pattern: `{version:04}_{description}.sql`

Example: `0009_update_thread_status_constraint.sql`

## How Migrations Work

1. The database tracks the current schema version in the `user_version` pragma
2. On startup, the application checks if the current version is less than the target version
3. For each missing version, it loads and executes the corresponding SQL file from this directory
4. After all migrations complete, the `user_version` is updated to the latest version

## Adding a New Migration

1. Increment `SCHEMA_VERSION` in `src/infra/db/database.rs`
2. Create a new SQL file in this directory with the next version number
3. Write your migration SQL (DDL statements, data transformations, etc.)
4. Add comments explaining what the migration does

## Migration Best Practices

- **Keep migrations immutable**: Once a migration is released, never modify it
- **Test thoroughly**: Migrations run on user data and must be bulletproof
- **Handle data carefully**: When restructuring tables, ensure data is preserved
- **Use transactions**: SQLite executes the entire file in a transaction by default
- **Comment complex logic**: Explain non-obvious transformations

## Current Migrations

- 0009: Update threads table CHECK constraint for ReviewStatus values
