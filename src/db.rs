use crate::models::{Request, RequestStr, RequestInput, User, Oauth};
use diesel::prelude::*;

pub fn get_open_requests(conn: &PgConnection) -> Result<Vec<Request>, diesel::result::Error> {
    use crate::schema::request::dsl::*;
    let open_requests = request
        .filter(status.eq("OPEN"))
        .order(pub_date.desc())
        .load::<Request>(conn)?;

    Ok(open_requests)
}

pub fn get_request_detail_by_id(
    conn: &PgConnection,
    id_: i64,
) -> Result<RequestStr, diesel::result::Error> {
    let request = get_request_by_id(conn, id_)?;
    let packager = {
        if let Some(packager_id) = request.packager_id {
            get_user_by_id(conn, packager_id)?
        } else {
            None
        }
    };
    let requester = get_user_by_id(conn, request.requester_id)?;
    let result = RequestStr {
        id: request.id,
        status: request.status,
        type_: request.type_,
        name: request.name,
        description: request.description,
        pub_date: request.pub_date,
        note: request.note,
        packager: {
            if let Some(packager) = packager {
                Some(packager.username)
            } else {
                None
            }
        },
        requester: {
            if let Some(requester) = requester {
                requester.username
            } else {
                "Unknown".to_owned()
            }
        },
    };

    Ok(result)
}

pub fn get_request_by_id(conn: &PgConnection, id_: i64) -> Result<Request, diesel::result::Error> {
    use crate::schema::request::dsl::*;
    let result = request.filter(id.eq(id_)).first(conn)?;
    Ok(result)
}

pub fn get_user_by_id(
    conn: &PgConnection,
    id_: i64,
) -> Result<Option<User>, diesel::result::Error> {
    use crate::schema::user::dsl::*;
    let user_ = user.filter(id.eq(id_)).first::<User>(conn).optional()?;
    Ok(user_)
}

pub fn get_user_by_username(
    conn: &PgConnection,
    username_: &str,
) -> Result<Option<User>, diesel::result::Error> {
    use crate::schema::user::dsl::*;
    let user_ = user
        .filter(username.eq(username_))
        .first::<User>(conn)
        .optional()?;
    Ok(user_)
}

pub fn get_user_by_oauth(
    conn: &PgConnection,
    service: &str,
    oid_: Option<&str>,
    token: Option<&str>,
) -> Result<User, diesel::result::Error> {
    use crate::schema::*;
    let (user_, _) = user::table.inner_join(oauth::table).filter(
        oauth::type_
            .eq(service)
            .and(oauth::oid.eq(oid_))
            .and(oauth::token.eq(token)),
    ).first::<(User, Oauth)>(conn)?;

    Ok(user_)
}

// Writables

pub fn close_request_by_id(
    conn: &PgConnection,
    id_: i64,
    reject: bool,
) -> Result<(), diesel::result::Error> {
    use crate::schema::request::dsl::*;
    diesel::update(request.find(id_))
        .set(status.eq({
            if reject {
                "REJECTED"
            } else {
                "DONE"
            }
        }))
        .get_result::<Request>(conn)?;

    Ok(())
}

pub fn update_password_hash(conn: &PgConnection, username_: String, hash: String) -> Result<(), diesel::result::Error> {
    use crate::schema::user::dsl::*;
    diesel::update(user.filter(username.eq(username_)))
        .set(password_hash.eq(hash))
        .get_result::<User>(conn)?;

    Ok(())
}
