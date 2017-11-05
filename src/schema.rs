table! {
    papers (id) {
        id -> Integer,
        title -> Text,
        authors -> Text,
        published_at -> BigInt,
        description -> Text,
        link -> Text,
    }
}

//infer_schema!("dotenv:DATABASE_URL");
