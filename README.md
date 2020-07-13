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
3. Save password somewhere safe and then update the password in `refinery.toml` 
   and add a `.env` file to the project directory with the contents:
    DATABASE_URL=postgresql://shopkeeper@<password>@localhost/shopkeeper
4. Run `cargo run -- -m` which will compile the app in debug mode and run the 
   database migrations.
5. Run `./devserver.sh` to run the dev server (by default it listens at 
   `0.0.0.0:3030`).

# Todo

* Make self-contained docker container that can run the app without any setup.
