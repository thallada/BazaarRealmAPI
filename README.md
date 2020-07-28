# Development Setup

1. Install and run postgres.
2. Create postgres user and database (and add uuid extension while you're there 
   ):
    createuser shopkeeper
    createdb shopkeeper
    sudo -u postgres -i psql
    postgres=# ALTER DATABASE shopkeeper OWNER TO shopkeeper;
    \password shopkeeper
    postgres=# CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
3. Save password somewhere safe and then and add a `.env` file to the project 
   directory with the contents:
    DATABASE_URL=postgresql://shopkeeper:<password>@localhost/shopkeeper
    RUST_LOG="shopkeeper=debug"
    HOST="http://localhost:3030"
4. Create a new file at `src/db/refinery.toml` with the contents:
    [main]
    db_type = "Postgres"
    db_host = "localhost"
    db_port = "5432"
    db_user = "shopkeeper"
    db_pass = "<database-password-here>"
    db_name = "shopkeeper"
4. Run `cargo run -- -m` which will compile the app in debug mode and run the 
   database migrations.
5. Run `./devserver.sh` to run the dev server (by default it listens at 
   `127.0.0.1:3030`).

# Todo

* Make self-contained docker container that can run the app without any setup.
* Add rate-limiting per IP address. The `tower` crate has a service that might 
  be useful for this.
