use crate::models::{Request, User};
use diesel::prelude::*;

pub fn get_open_requests(conn: &PgConnection) -> Result<Vec<Request>, diesel::result::Error> {
    use crate::schema::request::dsl::*;
    let open_requests = request
        .filter(status.eq("OPEN"))
        .order(pub_date.desc())
        .load::<Request>(conn)?;

    Ok(open_requests)
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
