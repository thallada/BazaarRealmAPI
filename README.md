# BazaarRealmAPI
The API for the Bazaar Realm Skyrim mod which is responsible for storing and
serving data related to the mod to all users.

Right now, the types of data the API stores and the endpoints to access them
are (all prefixed under `/v1`, the API version):

* `/owners`: Every player character that has registered with this API server.
   Contains their unique api key. Owners own shops.
* `/shops`: Metadata about each shop including name, description, and who owns
   it.
* `/interior_ref_lists`: Lists of in-game ObjectReferences that are in the
   interior of individual shops. When a user visits a shop, these references
   are loaded into the cell.
* `/merchandise_lists`: Lists of in-game Forms that are in the merchant chest
   of individual shops. When a user visits a shop, these forms are loaded
   onto the shop's shelves and are purchasable.

Bazaar Realm was designed to allow users to change the API they are using the
mod under, if they wish. The API can run on a small server with minimal
resources, which should be suitable for a small group of friends to share
shops with each other.

It uses the [`warp`](https://crates.io/crates/warp) web server framework and
[`sqlx`](https://crates.io/crates/sqlx) for database queries to a [PostgreSQL
database](https://www.postgresql.org).

## Development Setup

1. Install and run postgres.
2. Create postgres user and database (and add uuid extension while you're there 
   ):
```
createuser bazaarrealm
createdb bazaarrealm
sudo -u postgres -i psql
postgres=# ALTER DATABASE bazaarrealm OWNER TO bazaarrealm;
\password bazaarrealm
postgres=# CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

# Or, on Windows in PowerShell:

& 'C:\Program Files\PostgreSQL\13\bin\createuser.exe' -U postgres bazaarrealm
& 'C:\Program Files\PostgreSQL\13\bin\createdb.exe' -U postgres bazaarrealm
& 'C:\Program Files\PostgreSQL\13\bin\psql.exe' -U postgres
postgres=# ALTER DATABASE bazaarrealm OWNER TO bazaarrealm;
\password bazaarrealm
postgres=# CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
```
3. Save password somewhere safe and then and add a `.env` file to the project 
   directory with the contents:
```
DATABASE_URL=postgresql://bazaarrealm:<password>@localhost/bazaarrealm
RUST_LOG="bazaar_realm_api=debug"
HOST="http://localhost:3030"
```
4. Create a new file at `src/db/refinery.toml` with the contents:
```
[main]
db_type = "Postgres"
db_host = "localhost"
db_port = "5432"
db_user = "bazaarrealm"
db_pass = "<database-password-here>"
db_name = "bazaarrealm"
```
5. Run `cargo run -- -m` which will compile the app in debug mode and run the 
   database migrations.
6. Run `./devserver.sh` to run the dev server (by default it listens at 
   `127.0.0.1:3030`).

## Testing Data

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

## Authentication

I don't want to require users of Bazaar Realm to have to remember a password,
so I forgoed the typical username and password authentication in favor of a
unique UUID identifier instead. This is the api key that the
`BazaarRealmClient` generates when the user first starts the mod in a game.
The api key is stored in the save game files for the player character and is
required to be sent with any API request that modifies data.

Yes, it's not most secure solution, but I'm not convinced security is a huge
concern here. As long as users don't share their API key or the save game
files that contain it, their data should be secure.

# Todo

* Add update endpoints.
* Add endpoints for the other models.
* Make self-contained docker container that can run the app without any setup.
* Add rate-limiting per IP address. The `tower` crate has a service that might 
  be useful for this.