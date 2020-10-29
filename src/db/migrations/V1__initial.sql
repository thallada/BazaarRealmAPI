CREATE TABLE IF NOT EXISTS "owners" (
    "id" SERIAL PRIMARY KEY NOT NULL,
    "name" VARCHAR(255) NOT NULL,
    "api_key" UUID NOT NULL UNIQUE,
    "ip_address" inet,
    "mod_version" INTEGER NOT NULL,
    "created_at" timestamp(3) NOT NULL,
    "updated_at" timestamp(3) NOT NULL
);
CREATE UNIQUE INDEX "owners_unique_name_and_api_key" ON "owners" ("name", "api_key");
CREATE TABLE "shops" (
    "id" SERIAL PRIMARY KEY NOT NULL,
    "name" VARCHAR(255) NOT NULL,
    "owner_id" INTEGER REFERENCES "owners"(id) NOT NULL,
    "description" TEXT,
    "created_at" timestamp(3) NOT NULL,
    "updated_at" timestamp(3) NOT NULL
);
CREATE UNIQUE INDEX "shops_unique_name_and_owner_id" ON "shops" ("name", "owner_id");
CREATE TABLE "interior_ref_lists" (
    "id" SERIAL PRIMARY KEY NOT NULL,
    "shop_id" INTEGER REFERENCES "shops"(id) NOT NULL UNIQUE,
    "owner_id" INTEGER REFERENCES "owners"(id) NOT NULL,
    "ref_list" jsonb NOT NULL,
    "created_at" timestamp(3) NOT NULL,
    "updated_at" timestamp(3) NOT NULL
);
CREATE TABLE "merchandise_lists" (
    "id" SERIAL PRIMARY KEY NOT NULL,
    "shop_id" INTEGER REFERENCES "shops"(id) NOT NULL UNIQUE,
    "owner_id" INTEGER REFERENCES "owners"(id) NOT NULL,
    "form_list" jsonb NOT NULL
        CONSTRAINT "merchandise_quantity_gt_zero" CHECK (NOT jsonb_path_exists(form_list, '$[*].quantity ? (@ < 1)')),
    "created_at" timestamp(3) NOT NULL,
    "updated_at" timestamp(3) NOT NULL
);