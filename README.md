# Development Setup

1. Install and run postgres.
2. Create postgres user and database (and add uuid extension while you're there 
   ):
```
createuser shopkeeper
createdb shopkeeper
sudo -u postgres -i psql
postgres=# ALTER DATABASE shopkeeper OWNER TO shopkeeper;
\password shopkeeper
postgres=# CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

# Or, on Windows in PowerShell:

& 'C:\Program Files\PostgreSQL\13\bin\createuser.exe' -U postgres shopkeeper
& 'C:\Program Files\PostgreSQL\13\bin\createdb.exe' -U postgres shopkeeper
& 'C:\Program Files\PostgreSQL\13\bin\psql.exe' -U postgres
postgres=# ALTER DATABASE shopkeeper OWNER TO shopkeeper;
\password shopkeeper
postgres=# CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
```
3. Save password somewhere safe and then and add a `.env` file to the project 
   directory with the contents:
```
DATABASE_URL=postgresql://shopkeeper:<password>@localhost/shopkeeper
RUST_LOG="shopkeeper=debug"
HOST="http://localhost:3030"
```
4. Create a new file at `src/db/refinery.toml` with the contents:
```
[main]
db_type = "Postgres"
db_host = "localhost"
db_port = "5432"
db_user = "shopkeeper"
db_pass = "<database-password-here>"
db_name = "shopkeeper"
```
5. Run `cargo run -- -m` which will compile the app in debug mode and run the 
   database migrations.
6. Run `./devserver.sh` to run the dev server (by default it listens at 
   `127.0.0.1:3030`).

# Testing Data

Using [httpie](https://httpie.org/) you can use the json files in
`test_data/` to seed the database with data.

The `POST` endpoints require an API key. You can just [generate a random
uuidv4](https://www.uuidgenerator.net/version4), just make sure to use the
same one in all future requests.

```
http POST "http://localhost:3030/v1/owners" @test_data\owner.json api-key:"13e2f39c-033f-442f-b42a-7ad640d2e439"
http POST "http://localhost:3030/v1/shops" @test_data\shop.json api-key:"13e2f39c-033f-442f-b42a-7ad640d2e439"
http POST "http://localhost:3030/v1/interior_ref_lists" @test_data\interior_ref_list.json api-key:"13e2f39c-033f-442f-b42a-7ad640d2e439"
http POST "http://localhost:3030/v1/merchandise_lists" @test_data\merchandise_list.json api-key:"13e2f39c-033f-442f-b42a-7ad640d2e439"
# Then, you can test the GET endpoints
http GET "http://localhost:3030/v1/owners"
http GET "http://localhost:3030/v1/shops"
http GET "http://localhost:3030/v1/interior_ref_lists"
http GET "http://localhost:3030/v1/merchandise_lists"
```

# Todo

* Add update endpoints.
* Add endpoints for the other models.
* Make self-contained docker container that can run the app without any setup.
* Add rate-limiting per IP address. The `tower` crate has a service that might 
  be useful for this.
