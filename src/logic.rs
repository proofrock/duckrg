// Copyright (c) 2023-, Germano Rizzo <oss /AT/ germanorizzo /DOT/ it>
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::{collections::HashMap, ops::DerefMut, time::Duration};

use actix_web::{http::header::Header, rt::time::sleep, web, HttpRequest, Responder};
use actix_web_httpauth::headers::authorization::{Authorization, Basic};
use duckdb::{types::Value, ToSql, Transaction};
use eyre::Result;
use serde_json::{json, Map as JsonMap, Value as JsonValue};

use crate::{
    auth::process_auth,
    commons::{check_stored_stmt, ParamsContainer},
    db_config::{AuthMode, DbConfig},
    main_config::Db,
    req_res::{self, Response, ResponseItem},
    MUTEXES,
};

fn val_db2val_json(val: Value) -> JsonValue {
    match val {
        Value::Null => JsonValue::Null,
        Value::Boolean(v) => json!(v),
        Value::TinyInt(v) => json!(v),
        Value::SmallInt(v) => json!(v),
        Value::Int(v) => json!(v),
        Value::BigInt(v) => json!(v),
        Value::HugeInt(v) => json!(v),
        Value::UTinyInt(v) => json!(v),
        Value::USmallInt(v) => json!(v),
        Value::UInt(v) => json!(v),
        Value::UBigInt(v) => json!(v),
        Value::Decimal(v) => json!(v),
        Value::Float(v) => json!(v),
        Value::Double(v) => json!(v),
        Value::Date32(v) => json!(v),
        Value::Time64(_, i) => json!(i),    // FIXME
        Value::Timestamp(_, i) => json!(i), // FIXME
        Value::Text(v) => json!(v),
        Value::Blob(v) => json!(v),
        Value::Enum(v) => json!(v),
        _ => JsonValue::Null,
    }
}

fn calc_params(params: &Vec<JsonValue>) -> ParamsContainer {
    let mut ret_params: Vec<Box<dyn ToSql>> = Vec::new();

    for v in params {
        let val: Box<dyn ToSql> = if v.is_string() {
            Box::new(v.as_str().unwrap().to_owned())
        } else {
            Box::new(v.to_owned())
        };
        ret_params.push(val);
    }

    ParamsContainer::from(ret_params)
}

#[allow(clippy::type_complexity)]
fn do_query(
    tx: &Transaction,
    sql: &str,
    values: &Option<JsonValue>,
) -> Result<(Option<Vec<JsonValue>>, Option<usize>, Option<Vec<usize>>)> {
    // TODO See why this impl in sqliterg doesn't work
    let mut stmt = tx.prepare(sql)?;
    let mut rows = match values {
        Some(p) => {
            let array = p.as_array().unwrap();
            stmt.query(calc_params(array).slice().as_slice())?
        }
        None => stmt.query([])?,
    };
    let column_names = rows
        .as_ref() // Oh my dear God why.
        .unwrap()
        .column_names();
    let mut response = vec![];
    loop {
        let row = rows.next();
        match row {
            Ok(row) => match row {
                Some(row) => {
                    let mut map: JsonMap<String, JsonValue> = JsonMap::new();
                    for (i, col_name) in column_names.iter().enumerate() {
                        let value: Value = row.get_unwrap(i);
                        map.insert(col_name.to_string(), val_db2val_json(value));
                    }
                    response.push(JsonValue::Object(map));
                }
                None => break,
            },
            Err(e) => return Err(eyre!(e.to_string())),
        }
    }
    Ok((Some(response), None, None))
}

#[allow(clippy::type_complexity)]
fn do_statement(
    tx: &Transaction,
    sql: &str,
    values: &Option<JsonValue>,
    values_batch: &Option<Vec<JsonValue>>,
) -> Result<(Option<Vec<JsonValue>>, Option<usize>, Option<Vec<usize>>)> {
    Ok(if values.is_none() && values_batch.is_none() {
        let changed_rows = tx.execute(sql, [])?;
        (None, Some(changed_rows), None)
    } else if values.is_some() {
        let array = values.as_ref().unwrap().as_array().unwrap();
        let changed_rows = tx.execute(sql, calc_params(array).slice().as_slice())?;
        (None, Some(changed_rows), None)
    } else {
        // values_batch.is_some()
        let mut stmt = tx.prepare(sql)?;
        let mut ret = vec![];
        for p in values_batch.as_ref().unwrap() {
            let array = p.as_array().unwrap();
            let changed_rows = stmt.execute(calc_params(array).slice().as_slice())?;
            ret.push(changed_rows);
        }
        (None, None, Some(ret))
    })
}

fn process(
    db_name: &str,
    http_req: web::Json<req_res::Request>,
    stored_statements: &HashMap<String, String>,
    dbconf: &DbConfig,
) -> Result<Response> {
    let db_lock = MUTEXES.get().unwrap().get(db_name).unwrap();
    let mut db_lock_guard = db_lock.lock().unwrap();
    let conn = db_lock_guard.deref_mut();
    let tx = conn.transaction()?;

    let mut results = vec![];
    let mut failed: Option<(u16, usize, String)> = None; // http code, index, error

    for (idx, trx_item) in http_req.transaction.iter().enumerate() {
        #[allow(clippy::type_complexity)]
        let ret: Result<
            (Option<Vec<JsonValue>>, Option<usize>, Option<Vec<usize>>),
            (u16, String), // Error: (http code, message)
        > = if trx_item.query.is_some() == trx_item.statement.is_some() {
            Err((
                400,
                "exactly one of 'query' and 'statement' must be provided".to_string(),
            ))
        } else if let Some(query) = &trx_item.query {
            match check_stored_stmt(query, stored_statements, dbconf.use_only_stored_statements) {
                Ok(sql) => match do_query(&tx, sql, &trx_item.values) {
                    Ok(ok_payload) => Ok(ok_payload),
                    Err(err) => Err((500, err.to_string())),
                },
                Err(e) => Err((409, e.to_string())),
            }
        } else if trx_item.values.is_some() && trx_item.values_batch.is_some() {
            Err((
                400,
                "at most one of values and values_batch must be provided".to_string(),
            ))
        } else {
            let statement = trx_item.statement.as_ref().unwrap(); // always present, for the previous if's
            match check_stored_stmt(
                statement,
                stored_statements,
                dbconf.use_only_stored_statements,
            ) {
                Ok(sql) => match do_statement(&tx, sql, &trx_item.values, &trx_item.values_batch) {
                    Ok(ok_payload) => Ok(ok_payload),
                    Err(err) => Err((500, err.to_string())),
                },
                Err(e) => Err((409, e.to_string())),
            }
        };

        if let Err(err) = ret {
            failed = Some((err.0, idx, err.1));
            break;
        }

        results.push(match ret {
            Ok(val) => ResponseItem {
                success: true,
                error: None,
                result_set: val.0,
                rows_updated: val.1,
                rows_updated_batch: val.2,
            },
            Err(err) => ResponseItem {
                success: false,
                error: Some(err.1),
                result_set: None,
                rows_updated: None,
                rows_updated_batch: None,
            },
        });
    }

    Ok(match failed {
        Some(f) => {
            tx.rollback()?;
            Response::new_err(f.0, f.1 as isize, f.2)
        }
        None => {
            tx.commit()?;
            Response::new_ok(results)
        }
    })
}

pub async fn handler(
    req: HttpRequest,
    body: web::Json<req_res::Request>,
    db_conf: web::Data<Db>,
    db_name: web::Data<String>,
) -> impl Responder {
    if let Some(ac) = &db_conf.conf.auth {
        let ac_headers = if matches!(
            db_conf.conf.auth.as_ref().unwrap().mode,
            AuthMode::HttpBasic
        ) {
            match Authorization::<Basic>::parse(&req) {
                Ok(a) => Some(a),
                Err(_) => None,
            }
        } else {
            None
        };

        if !process_auth(ac, &body.credentials, &ac_headers, &db_name) {
            sleep(Duration::from_millis(1000)).await;

            return Response::new_err(ac.auth_error_code, -1, "Authorization failed".to_string());
        }
    }

    process(&db_name, body, &db_conf.stored_statements, &db_conf.conf).unwrap()
}
