use barrel::{backend::Pg, types, Migration};

pub fn migration() -> String {
    let mut m = Migration::new();

    m.create_table("owners", |t| {
        t.add_column("id", types::primary().indexed(true));
        t.add_column("name", types::varchar(255));
        t.add_column("api_key", types::uuid());
        t.add_column("ip_address", types::custom("inet").nullable(true));
        t.add_column("mod_version", types::varchar(25));
        t.add_column("created_at", types::custom("timestamp(3)"));
        t.add_column("updated_at", types::custom("timestamp(3)"));
        t.add_index(
            "owners_unique_name_and_api_key",
            types::index(vec!["name", "api_key"]).unique(true),
        );
    });

    m.create_table("shops", |t| {
        t.add_column("id", types::primary().indexed(true));
        t.add_column("name", types::varchar(255));
        t.add_column("owner_id", types::foreign("owners", "id").indexed(true));
        t.add_column("description", types::text().nullable(true));
        t.add_column("is_not_sell_buy", types::boolean().default(true));
        t.add_column("sell_buy_list_id", types::integer().default(0));
        t.add_column("vendor_id", types::integer());
        t.add_column("vendor_gold", types::integer());
        t.add_column("created_at", types::custom("timestamp(3)"));
        t.add_column("updated_at", types::custom("timestamp(3)"));
        t.add_index(
            "shops_unique_name_and_owner_id",
            types::index(vec!["name", "owner_id"]).unique(true),
        );
    });

    m.create_table("merchandise", |t| {
        t.add_column("id", types::primary().indexed(true));
        t.add_column("shop_id", types::foreign("shops", "id").indexed(true));
        t.add_column("mod_name", types::varchar(255));
        t.add_column("local_form_id", types::integer());
        t.add_column("quantity", types::integer());
        t.add_column("created_at", types::custom("timestamp(3)"));
        t.add_column("updated_at", types::custom("timestamp(3)"));
        t.add_index(
            "merchandise_unique_mod_shop_id_name_and_local_form_id",
            types::index(vec!["shop_id", "mod_name", "local_form_id"]).unique(true),
        );
    });

    m.create_table("transactions", |t| {
        t.add_column("id", types::primary().indexed(true));
        t.add_column("shop_id", types::foreign("shops", "id").indexed(true));
        t.add_column("merchandise_id", types::foreign("merchandise", "id"));
        t.add_column("customer_name", types::varchar(255));
        t.add_column("is_customer_npc", types::boolean());
        t.add_column("is_customer_buying", types::boolean());
        t.add_column("quantity", types::integer());
        t.add_column("is_void", types::boolean());
        t.add_column("created_at", types::custom("timestamp(3)"));
    });

    m.create_table("interior_ref_lists", |t| {
        t.add_column("id", types::primary().indexed(true));
        t.add_column("shop_id", types::foreign("shops", "id").indexed(true));
        t.add_column("ref_list", types::custom("jsonb"));
        t.add_column("created_at", types::custom("timestamp(3)"));
        t.add_column("updated_at", types::custom("timestamp(3)"));
    });

    m.make::<Pg>()
}
