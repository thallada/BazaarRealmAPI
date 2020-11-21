# BazaarRealmAPI

The API for the Bazaar Realm Skyrim mod which is responsible for storing and
serving data related to the mod to all users.

Right now, the types of data the API stores and the endpoints to access them
are (all prefixed under `/v1`, the API version):

- `/owners`: Every player character that has registered with this API server.
  Contains their unique api key. Owners own shops.
- `/shops`: Metadata about each shop including name, description, and who owns
  it.
- `/interior_ref_lists`: Lists of in-game ObjectReferences that are in the
  interior of individual shops. When a user visits a shop, these references
  are loaded into the cell.
- `/merchandise_lists`: Lists of in-game Forms that are in the merchant chest
  of individual shops. When a user visits a shop, these forms are loaded
  onto the shop's shelves and are purchasable.
- `/transactions`: Allows posting a new buy or sell between an owner and a
  shop's merchandise.

Bazaar Realm was designed to allow users to change the API they are using the
mod under, if they wish. The API can run on a small server with minimal
resources, which should be suitable for a group of friends to share shops
with each other.

It uses the [`warp`](https://crates.io/crates/warp) web server framework and
[`sqlx`](https://crates.io/crates/sqlx) for database queries to a [PostgreSQL
database](https://www.postgresql.org).

The API was designed with performance as a high priority. When it serves a
response, it also caches that response so future queries for the same data
can be returned in less than 1ms. To reduce data sent over the network,
clients can use the
[ETag](https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/ETag)
headers to indicate to the server what version of the data they have cached
so the server can send a 304 response with no data if the resource hasn't
changed since the client last requested. Using the
[Accept](https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Accept)
header, clients can also opt for the more space-efficient and faster to
deserialize [bincode](https://github.com/servo/bincode) format instead of the
JSON default.

Related projects:

- [`BazaarRealmClient`](https://github.com/thallada/BazaarRealmClient): DLL that
  handles requests and responses to this API
- [`BazaarRealmPlugin`](https://github.com/thallada/BazaarRealmPlugin):
  [SKSE](https://skse.silverlock.org/) plugin for the mod that modifies data
  within the Skyrim game engine
- [`BazaarRealmMod`](https://github.com/thallada/BazaarRealmMod): Papyrus
  scripts, ESP plugin, and all other resources for the mod

## Docker Setup

The easiest way to get the server up and running is using Docker.

1. Download and install [Docker Desktop](https://www.docker.com/get-started)
2. In PowerShell, cmd.exe, or a terminal run `docker pull postgres:alpine` then `docker pull thallada/bazaarrealm:latest`
3. Run (replacing `<password>` with a secure generated password):

```
docker run -d --name postgres --network=bazaarrealm --network-alias=db --env POSTGRES_DB=bazaarrealm --env POSTGRES_USER=bazaarrealm --env POSTGRES_PASSWORD=<password> postgres:alpine
```

4. Run (replacing `<password>` with what you generated in previous step):

```
docker run -d --name bazaarrealm -p 3030:3030 --network=bazaarrealm --network-alias=api --env DATABASE_URL=postgresql://bazaarrealm:<password>@db/bazaarrealm --env HOST=http://localhost:3030 thallada/bazaarrealm:latest
```

5. The server should now be available at `http://localhost:3030`.

## Docker-Compose Setup

An alternative way to set up the API, is to use `docker-compose` which can
orchestrate setting up the database and web server containers for you. This
method is more useful if you would like to make changes to the API code and
test them out.

1. Download and install [Docker Desktop](https://www.docker.com/get-started)
2. Git clone this repo into a folder of your choosing: `git clone https://github.com/thallada/BazaarRealmAPI.git`
3. Create a new file `.env.docker` in the checked out `bazaar_realm_api`
   folder with the contents (replacing `<password>` with a secure generated
   password):

```
DATABASE_URL="postgresql://bazaarrealm:<password>@db/bazaarrealm"
RUST_LOG="bazaar_realm_api=debug,warp=info"
HOST="http://localhost:3030"
PORT=3030
POSTGRES_DB=bazaarrealm
POSTGRES_USER=bazaarrealm
POSTGRES_PASSWORD=<password>
```

3. In the checked out repo, run: `docker-compose build`
4. Once that completes, run: `docker-compose up`

## Manual Development Setup

If you would prefer to run the server outside Docker on your host machine, do
the following steps to get everything setup.

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
PORT=3030
```

4. Install
   [`sqlx_cli`](https://github.com/launchbadge/sqlx/tree/master/sqlx-cli) with
   `cargo install --version=0.1.0-beta.1 sqlx-cli --no-default-features --features postgres`
5. Run `sqlx migrate --source db/migrations run` which will run all the database 
   migrations.
6. Run `./devserver.sh` to run the dev server (by default it listens at
   `127.0.0.1:3030`). Note that this runs the server in debug mode and shouldn't
   be used to serve requests from the mod. You can build the release version of
   the server with `cargo build --release`.

## TLS setup

If you would like to access the server over HTTPS, you can use [Let's
Encrypt](https://letsencrypt.org/) to generate a SSL certificate and key and
provide it to the API. Once you use [certbot](https://certbot.eff.org/) to
generate the certificate and key for your domain in
`/etc/letsencrypt/live/<domain>/`, run the api server with:

```
docker run -d --name bazaarrealm --network=host --env DATABASE_URL=postgresql://bazaarrealm:<password>@localhost/bazaarrealm --env PORT=443 --HOST=https://<domain> --env TLS_CERT=/etc/letsencrypt/live/<domain>/fullchain.pem --env TLS_KEY=/etc/letsencrypt/live/<domain>/privkey.pem -v /etc/letsencrypt/:/etc/letsencrypt/ thallada/bazaarrealm:latest
```

This command assumes that you are on Linux and you have a running postgres
database already set up outside of docker. See Manual Development Setup for
database setup instructions.

The server should be accessible at your domain: `https://<domain>`.

## Testing Data

Using [httpie](https://httpie.org/) you can use the json files in
`test_data/` to seed the database with data.

The `POST` endpoints require an API key. You can just [generate a random
uuidv4](https://www.uuidgenerator.net/version4), just make sure to use the
same one in all future requests.

```
http POST "http://localhost:3030/v1/owners" @test_data\owner.json api-key:"13e2f39c-033f-442f-b42a-7ad640d2e439"
http POST "http://localhost:3030/v1/shops" @test_data\shop.json api-key:"13e2f39c-033f-442f-b42a-7ad640d2e439"
http PATCH "http://localhost:3030/v1/shops/1/interior_ref_list" @test_data\interior_ref_list.json api-key:"13e2f39c-033f-442f-b42a-7ad640d2e439"
http PATCH "http://localhost:3030/v1/shops/1/merchandise_list" @test_data\merchandise_list.json api-key:"13e2f39c-033f-442f-b42a-7ad640d2e439"
# Then, you can test the GET endpoints
http GET "http://localhost:3030/v1/owners"
http GET "http://localhost:3030/v1/shops"
http GET "http://localhost:3030/v1/interior_ref_lists"
http GET "http://localhost:3030/v1/merchandise_lists"
```

## Database Migrations

Migrations are handled by `sqlx`. When the server initially starts, it will
connect to the database and check if there are any migrations in
`db/migrations` that have not yet been applied. It will apply any at that
time and then continue starting the server.

A new migration can be created by running: `sqlx migrate add <name>`.

To allow the docker container for the API to get built in CI without a
database, the `sqlx-data.json` file needs to be re-generated every time the
database schema changes or any query is updated. It can be generated with `cargo 
sqlx prepare`.

## Authentication

I don't want to require users of Bazaar Realm to have to remember a password,
so I forgoed the typical username and password authentication in favor of a
unique UUID identifier instead. This is the api key that the
`BazaarRealmClient` generates when the user first starts the mod in a game.
The api key is stored in the save game files for the player character and is
required to be sent with any API request that modifies data.

Yes, it's not the most secure solution, but I'm not convinced security is a
huge concern here. As long as users don't share their API key or the save
game files that contain it, their data should be secure.
