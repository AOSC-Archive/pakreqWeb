use crate::models::{Request, RequestStr, User};
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
                packager.username
            } else {
                "Unknown".to_owned()
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
