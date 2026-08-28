Postgres setup

This repository includes a simple Postgres setup for local development and CI.

Quickstart (local):

1. Copy .env.example to .env and edit values if needed:
   cp .env.example .env

2. Start Postgres with Docker Compose:
   docker compose up -d

3. The database will be initialized using the SQL files in db/init (docker-entrypoint-initdb.d).

4. Connect using DATABASE_URL in .env or psql:
   psql "postgresql://appuser:apppassword@localhost:5432/appdb"

CI:

- A GitHub Actions workflow (.github/workflows/ci-postgres.yml) starts a Postgres service and runs db/init/schema.sql before running tests.

Notes & next steps:

- Replace placeholder credentials in .env (do NOT commit real secrets).
- Add your application's database configuration or ORM settings to use DATABASE_URL.
- If you want, I can add a sample config for Node (TypeORM/knex), Django, Rails, or another stack.
