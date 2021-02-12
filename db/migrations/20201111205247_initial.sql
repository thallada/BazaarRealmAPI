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
    "gold" INTEGER NOT NULL DEFAULT 0
        CONSTRAINT "shop_gold_gt_zero" CHECK (gold >= 0),
    "shop_type" VARCHAR(255) NOT NULL DEFAULT 'general_store',
    "vendor_keywords" TEXT[] NOT NULL DEFAULT '{"VendorItemKey", "VendorNoSale"}',
    "vendor_keywords_exclude" BOOLEAN NOT NULL DEFAULT true,
    "created_at" timestamp(3) NOT NULL,
    "updated_at" timestamp(3) NOT NULL
);
CREATE UNIQUE INDEX "shops_unique_name_and_owner_id" ON "shops" ("name", "owner_id");
CREATE TABLE "interior_ref_lists" (
    "id" SERIAL PRIMARY KEY NOT NULL,
    "shop_id" INTEGER REFERENCES "shops"(id) NOT NULL UNIQUE,
    "owner_id" INTEGER REFERENCES "owners"(id) NOT NULL,
    "ref_list" jsonb NOT NULL,
    "shelves" jsonb NOT NULL,
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
CREATE INDEX "merchandise_lists_mod_name_and_local_form_id" ON "merchandise_lists" USING GIN (form_list jsonb_path_ops);
CREATE TABLE "vendors" (
    "id" SERIAL PRIMARY KEY NOT NULL,
    "shop_id" INTEGER REFERENCES "shops"(id) NOT NULL UNIQUE,
    "owner_id" INTEGER REFERENCES "owners"(id) NOT NULL,
    "name" VARCHAR(255) NOT NULL,
    "body_preset" INTEGER NOT NULL
);
CREATE UNIQUE INDEX "vendors_unique_name_and_owner_id" ON "vendors" ("name", "owner_id", "shop_id");
CREATE TABLE "transactions" (
    "id" SERIAL PRIMARY KEY NOT NULL,
    "shop_id" INTEGER REFERENCES "shops"(id) NOT NULL,
    "owner_id" INTEGER REFERENCES "owners"(id) NOT NULL,
    "mod_name" VARCHAR(260) NOT NULL,
    "local_form_id" INTEGER NOT NULL,
    "name" TEXT NOT NULL,
    "form_type" INTEGER NOT NULL,
    "is_food" BOOLEAN NOT NULL,
    "price" INTEGER NOT NULL,
    "is_sell" BOOLEAN NOT NULL,
    "quantity" INTEGER NOT NULL,
    "amount" INTEGER NOT NULL,
    "keywords" TEXT[] NOT NULL DEFAULT '{}',
    "created_at" timestamp(3) NOT NULL,
    "updated_at" timestamp(3) NOT NULL
);
CREATE INDEX "transactions_shop_id" ON "transactions" ("shop_id");
CREATE INDEX "transactions_owner_id" ON "transactions" ("owner_id");
CREATE INDEX "transactions_mod_name_and_local_form_id" ON "transactions" ("mod_name", "local_form_id");
