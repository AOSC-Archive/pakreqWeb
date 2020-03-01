table! {
    oauth (uid, type_) {
        uid -> Int8,
        #[sql_name = "type"]
        type_ -> Text,
        oid -> Nullable<Text>,
        token -> Nullable<Text>,
    }
}

table! {
    request (id) {
        id -> Int8,
        status -> Text,
        #[sql_name = "type"]
        type_ -> Text,
        name -> Text,
        description -> Nullable<Text>,
        requester_id -> Int8,
        packager_id -> Nullable<Int8>,
        pub_date -> Date,
        note -> Nullable<Text>,
    }
}

table! {
    user (id) {
        id -> Int8,
        username -> Text,
        admin -> Bool,
        password_hash -> Nullable<Text>,
    }
}

joinable!(oauth -> user (uid));
joinable!(request -> user (requester_id));

allow_tables_to_appear_in_same_query!(
    oauth,
    request,
    user,
);
