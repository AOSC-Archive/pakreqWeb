use crate::models::{Oauth, Request, RequestInput, RequestStr, User};
use anyhow::{anyhow, Result};
use sqlx::PgPool;

pub async fn get_open_requests(conn: &PgPool) -> Result<Vec<Request>> {
    let records = sqlx::query!(r#"SELECT * FROM request WHERE status = 'OPEN' ORDER BY id DESC"#)
        .fetch_all(conn)
        .await?;
    let mut open_requests = Vec::new();
    open_requests.reserve(records.len());
    for record in records {
        open_requests.push(Request {
            id: record.id,
            status: record.status,
            type_: record.r#type,
            name: record.name,
            description: record.description,
            requester_id: record.requester_id,
            packager_id: record.packager_id,
            pub_date: record.pub_date,
            note: record.note,
        });
    }

    Ok(open_requests)
}

pub async fn get_open_requests_json(conn: &PgPool) -> Result<String> {
    let records = sqlx::query!(
        r#"SELECT json_agg(request)::TEXT AS json FROM request WHERE status = 'OPEN'"#
    )
    .fetch_one(conn)
    .await?;

    Ok(records.json.ok_or(anyhow!("PG returned empty string"))?)
}

pub async fn get_request_detail_by_id(conn: &PgPool, id_: i64) -> Result<RequestStr> {
    let record = sqlx::query!(
        r#"
        SELECT *,
        (SELECT username FROM "user" WHERE r.requester_id = "user".id) AS requester,
        (SELECT username FROM "user" WHERE r.packager_id = "user".id) AS packager FROM request r
        WHERE r.id = $1
        "#,
        id_
    )
    .fetch_one(conn)
    .await?;
    let result = RequestStr {
        id: record.id,
        status: record.status,
        type_: record.r#type,
        name: record.name,
        description: record.description,
        pub_date: record.pub_date,
        note: record.note,
        packager: record.packager,
        requester: record.requester.unwrap_or("Unknown".to_string()),
    };

    Ok(result)
}

pub async fn get_user_by_username(conn: &PgPool, username_: &str) -> Result<User> {
    let record = sqlx::query!(
        r#"SELECT id, username, password_hash FROM "user" WHERE username = $1"#,
        username_
    )
    .fetch_one(conn)
    .await?;

    Ok(User {
        id: record.id,
        username: record.username,
        admin: false,
        password_hash: record.password_hash,
    })
}

pub async fn get_user_by_oid(conn: &PgPool, service: &str, oid: &str) -> Result<User> {
    let user_ = sqlx::query_as!(
        User,
        r#"SELECT u.id, u.username, u.admin, u.password_hash
        FROM "user" u INNER JOIN oauth o ON o.uid = u.id
        WHERE o.oid = $1 AND o.type = $2"#,
        oid,
        service
    )
    .fetch_one(conn)
    .await?;

    Ok(user_)
}

pub async fn get_oauth_by_username(conn: &PgPool, username: &str) -> Result<Vec<Oauth>> {
    let mut oauth = Vec::new();
    let records = sqlx::query!(
        r#"SELECT o.uid, o.type, o.oid
        FROM "user" u INNER JOIN oauth o ON o.uid = u.id
        WHERE u.username = $1"#,
        username
    )
    .fetch_all(conn)
    .await?;
    for record in records {
        oauth.push(Oauth {
            uid: record.uid,
            type_: record.r#type,
            oid: record.oid,
            token: None,
        });
    }

    Ok(oauth)
}

// Writables

pub async fn close_request_by_id(conn: &PgPool, id_: i64, reject: bool) -> Result<()> {
    sqlx::query!(
        r#"UPDATE request SET status = $1 WHERE id = $2"#,
        if reject { "REJECTED" } else { "DONE" },
        id_
    )
    .execute(conn)
    .await?;

    Ok(())
}

pub async fn update_password_hash(conn: &PgPool, username_: String, hash: String) -> Result<()> {
    sqlx::query!(
        r#"UPDATE "user" SET password_hash = $1 WHERE username = $2"#,
        hash,
        username_
    )
    .execute(conn)
    .await?;

    Ok(())
}

pub async fn add_user(conn: &PgPool, user_: User) -> Result<()> {
    let mut tx = conn.begin().await?;
    sqlx::query!(
        r#"INSERT INTO "user" (username, admin, password_hash) VALUES ($1, $2, $3)"#,
        user_.username,
        user_.admin,
        user_.password_hash
    )
    .execute(&mut tx)
    .await?;
    tx.commit().await?;

    Ok(())
}

pub async fn add_oauth_info(conn: &PgPool, info: Oauth) -> Result<()> {
    let mut tx = conn.begin().await?;
    sqlx::query!(
        r#"INSERT INTO "oauth" (uid, type, oid, token) VALUES ($1, $2, $3, $4)"#,
        info.uid,
        info.type_,
        info.oid,
        info.token
    )
    .execute(&mut tx)
    .await?;
    tx.commit().await?;

    Ok(())
}

pub async fn delete_oauth_info(conn: &PgPool, info: Oauth) -> Result<()> {
    let mut tx = conn.begin().await?;
    sqlx::query!(
        r#"DELETE FROM "oauth" WHERE uid = $1 AND type = $2 AND oid = $3"#,
        info.uid,
        info.type_,
        info.oid,
    )
    .execute(&mut tx)
    .await?;
    tx.commit().await?;

    Ok(())
}
